//! T2L21 的状态观测模块。
//!
//! 这个模块不做源码级符号回溯，而是维护一份教学友好的运行时状态快照：
//! - 每个任务的关键计数器与最近一次 trap 信息
//! - 特定执行点的状态转储
//! - 面向 panic 的 breadcrumb 调用链

const TASK_CAPACITY: usize = 32;
const BREADCRUMB_CAPACITY: usize = 16;

#[derive(Clone, Copy)]
struct TaskObservation {
    loaded_entry: usize,
    initial_sp: usize,
    enters: usize,
    syscalls: usize,
    trace_requests: usize,
    yields: usize,
    timer_interrupts: usize,
    breakpoints: usize,
    exceptions: usize,
    exits: usize,
    last_syscall: usize,
    last_scause: usize,
    last_sepc: usize,
    last_stval: usize,
}

impl TaskObservation {
    const ZERO: Self = Self {
        loaded_entry: 0,
        initial_sp: 0,
        enters: 0,
        syscalls: 0,
        trace_requests: 0,
        yields: 0,
        timer_interrupts: 0,
        breakpoints: 0,
        exceptions: 0,
        exits: 0,
        last_syscall: usize::MAX,
        last_scause: 0,
        last_sepc: 0,
        last_stval: 0,
    };
}

static mut TASKS: [TaskObservation; TASK_CAPACITY] = [TaskObservation::ZERO; TASK_CAPACITY];
static mut LAST_DUMPS: [TaskObservation; TASK_CAPACITY] = [TaskObservation::ZERO; TASK_CAPACITY];
static mut CRUMB_DEPTH: usize = 0;
static mut CRUMBS: [&str; BREADCRUMB_CAPACITY] = [""; BREADCRUMB_CAPACITY];

/// breadcrumb 作用域守卫。
pub(crate) struct ScopeGuard {
    active: bool,
}

impl Drop for ScopeGuard {
    fn drop(&mut self) {
        if !self.active {
            return;
        }
        unsafe {
            if CRUMB_DEPTH > 0 {
                CRUMB_DEPTH -= 1;
                CRUMBS[CRUMB_DEPTH] = "";
            }
        }
    }
}

/// 进入一个 breadcrumb 作用域。
pub(crate) fn scope(label: &'static str) -> ScopeGuard {
    unsafe {
        if CRUMB_DEPTH < BREADCRUMB_CAPACITY {
            CRUMBS[CRUMB_DEPTH] = label;
            CRUMB_DEPTH += 1;
        }
    }
    ScopeGuard { active: true }
}

/// 初始化任务的观测状态。
pub(crate) fn init_task(task_id: usize, entry: usize, sp: usize) {
    unsafe {
        TASKS[task_id] = TaskObservation {
            loaded_entry: entry,
            initial_sp: sp,
            ..TaskObservation::ZERO
        };
        LAST_DUMPS[task_id] = TaskObservation::ZERO;
    }
    println!("[OBS][load] task={task_id} entry={entry:#x} sp={sp:#x}");
}

/// 记录首次进入或重新进入用户态。
pub(crate) fn on_enter(task_id: usize, sepc: usize, first: bool) {
    let enters = unsafe {
        TASKS[task_id].enters += 1;
        TASKS[task_id].last_sepc = sepc;
        TASKS[task_id].enters
    };
    let phase = if first { "first-enter" } else { "resume" };
    println!("[OBS][enter] task={task_id} phase={phase} enters={enters} sepc={sepc:#x}");
}

/// 记录时钟中断导致的抢占。
pub(crate) fn on_timer(task_id: usize, sepc: usize) {
    let count = unsafe {
        TASKS[task_id].timer_interrupts += 1;
        TASKS[task_id].last_sepc = sepc;
        TASKS[task_id].timer_interrupts
    };
    println!("[OBS][timer] task={task_id} timer_interrupts={count} sepc={sepc:#x}");
}

/// 记录一次系统调用。
pub(crate) fn on_syscall(task_id: usize, syscall_id: usize, sepc: usize) {
    let count = unsafe {
        TASKS[task_id].syscalls += 1;
        TASKS[task_id].last_syscall = syscall_id;
        TASKS[task_id].last_sepc = sepc;
        TASKS[task_id].syscalls
    };
    println!("[OBS][syscall] task={task_id} id={syscall_id} count={count} sepc={sepc:#x}");
}

/// 记录一次 trace 请求。
pub(crate) fn on_trace_request(task_id: usize, request: usize) {
    let count = unsafe {
        TASKS[task_id].trace_requests += 1;
        TASKS[task_id].trace_requests
    };
    println!("[OBS][trace] task={task_id} request={request} count={count}");
}

/// 记录一次主动让出。
pub(crate) fn on_yield(task_id: usize, next_task: usize) {
    let count = unsafe {
        TASKS[task_id].yields += 1;
        TASKS[task_id].yields
    };
    println!("[OBS][yield] from={task_id} to={next_task} yields={count}");
}

/// 记录一次断点命中。
pub(crate) fn on_breakpoint(task_id: usize, sepc: usize) {
    let count = unsafe {
        TASKS[task_id].breakpoints += 1;
        TASKS[task_id].last_sepc = sepc;
        TASKS[task_id].breakpoints
    };
    println!("[OBS][breakpoint] task={task_id} hits={count} sepc={sepc:#x}");
}

/// 记录异常路径。
pub(crate) fn on_exception(task_id: usize, reason: &str, scause: usize, sepc: usize, stval: usize) {
    let count = unsafe {
        TASKS[task_id].exceptions += 1;
        TASKS[task_id].last_scause = scause;
        TASKS[task_id].last_sepc = sepc;
        TASKS[task_id].last_stval = stval;
        TASKS[task_id].exceptions
    };
    println!(
        "[OBS][exception] task={task_id} reason={reason} exceptions={count} scause={scause:#x} sepc={sepc:#x} stval={stval:#x}"
    );
}

/// 记录退出路径。
pub(crate) fn on_exit(task_id: usize, code: usize) {
    let count = unsafe {
        TASKS[task_id].exits += 1;
        TASKS[task_id].exits
    };
    println!("[OBS][exit] task={task_id} code={code} exits={count}");
}

/// 打印某个任务的当前快照。
pub(crate) fn dump_task(task_id: usize, reason: &str) {
    let (obs, prev) = unsafe { (TASKS[task_id], LAST_DUMPS[task_id]) };
    println!(
        "[OBS][snapshot] reason={reason} task={task_id} entry={:#x} sp={:#x} enters={} syscalls={} trace_requests={} yields={} timer_interrupts={} breakpoints={} exceptions={} exits={} last_syscall={} last_scause={:#x} last_sepc={:#x} last_stval={:#x}",
        obs.loaded_entry,
        obs.initial_sp,
        obs.enters,
        obs.syscalls,
        obs.trace_requests,
        obs.yields,
        obs.timer_interrupts,
        obs.breakpoints,
        obs.exceptions,
        obs.exits,
        obs.last_syscall,
        obs.last_scause,
        obs.last_sepc,
        obs.last_stval
    );
    println!(
        "[OBS][delta] reason={reason} task={task_id} enters+={} syscalls+={} trace_requests+={} yields+={} timer_interrupts+={} breakpoints+={} exceptions+={} exits+={} last_syscall:{}->{}, last_scause:{:#x}->{:#x}, last_sepc:{:#x}->{:#x}, last_stval:{:#x}->{:#x}",
        obs.enters.saturating_sub(prev.enters),
        obs.syscalls.saturating_sub(prev.syscalls),
        obs.trace_requests.saturating_sub(prev.trace_requests),
        obs.yields.saturating_sub(prev.yields),
        obs.timer_interrupts.saturating_sub(prev.timer_interrupts),
        obs.breakpoints.saturating_sub(prev.breakpoints),
        obs.exceptions.saturating_sub(prev.exceptions),
        obs.exits.saturating_sub(prev.exits),
        prev.last_syscall,
        obs.last_syscall,
        prev.last_scause,
        obs.last_scause,
        prev.last_sepc,
        obs.last_sepc,
        prev.last_stval,
        obs.last_stval
    );
    unsafe {
        LAST_DUMPS[task_id] = obs;
    }
}

/// 打印整轮运行的摘要指标。
pub(crate) fn dump_summary(task_count: usize) {
    let mut syscalls = 0usize;
    let mut trace_requests = 0usize;
    let mut yields = 0usize;
    let mut timer_interrupts = 0usize;
    let mut breakpoints = 0usize;
    let mut exceptions = 0usize;
    let mut exits = 0usize;

    for task_id in 0..task_count {
        let obs = unsafe { TASKS[task_id] };
        syscalls += obs.syscalls;
        trace_requests += obs.trace_requests;
        yields += obs.yields;
        timer_interrupts += obs.timer_interrupts;
        breakpoints += obs.breakpoints;
        exceptions += obs.exceptions;
        exits += obs.exits;
    }

    println!(
        "[OBS][metric] reason=summary tasks={task_count} syscalls={syscalls} trace_requests={trace_requests} yields={yields} timer_interrupts={timer_interrupts} breakpoints={breakpoints} exceptions={exceptions} exits={exits}"
    );
}

/// 在 panic 或崩溃前输出 breadcrumb 调用链。
pub(crate) fn dump_breadcrumbs() {
    let depth = unsafe { CRUMB_DEPTH };
    println!("[OBS][panic] breadcrumb_depth={depth}");
    for index in 0..depth {
        let label = unsafe { CRUMBS[index] };
        println!("[OBS][crumb] #{index} {label}");
    }
}
