//! 进程与线程管理模块
//!
//! ## 与第七章的区别
//!
//! 第七章中 `Process` 既是资源容器又是执行单元。
//! 第八章将两者分离：
//! - **Process**：资源容器，管理地址空间、文件描述符、**同步原语列表**、信号
//! - **Thread**：执行单元，管理 TID 和上下文
//!
//! 同一进程的所有线程共享 `Process` 中的资源。
//!
//! ## 新增字段
//!
//! | 字段 | 说明 |
//! |------|------|
//! | `semaphore_list` | 信号量列表（进程内所有线程共享） |
//! | `mutex_list` | 互斥锁列表 |
//! | `condvar_list` | 条件变量列表 |
//!
//! 教程阅读建议：
//!
//! - 先看 `Process` 与 `Thread` 的字段分工：明确“资源归进程、执行归线程”；
//! - 再看 `fork/exec/from_elf`：理解跨线程模型后，进程复制与替换语义如何变化；
//! - 最后结合 `processor.rs` 看线程生命周期与进程资源回收的关系。

use crate::{
    build_flags, fs::Fd, map_portal, parse_flags, processor::ProcessorInner, Sv39, Sv39Manager,
    PROCESSOR,
};
use alloc::{
    alloc::alloc_zeroed,
    boxed::Box,
    collections::BTreeMap,
    sync::Arc,
    vec::Vec,
};
use core::alloc::Layout;
use spin::Mutex;
use tg_kernel_context::{foreign::ForeignContext, LocalContext};
use tg_kernel_vm::{
    page_table::{MmuMeta, VAddr, PPN, VPN},
    AddressSpace,
};
use tg_signal::Signal;
use tg_signal_impl::SignalImpl;
use tg_sync::{Condvar, Mutex as MutexTrait, Semaphore};
use tg_task_manage::{ProcId, ThreadId};
use xmas_elf::{
    header::{self, HeaderPt2, Machine},
    program, ElfFile,
};

/// 线程（执行单元）
///
/// 每个线程有独立的 TID 和上下文（寄存器状态、satp）。
/// 同一进程的多个线程共享地址空间。
pub struct Thread {
    /// 线程 ID（不可变）
    pub tid: ThreadId,
    /// 执行上下文（包含 LocalContext + satp）
    pub context: ForeignContext,
}

impl Thread {
    /// 创建新线程
    pub fn new(satp: usize, context: LocalContext) -> Self {
        Self {
            tid: ThreadId::new(),
            context: ForeignContext { context, satp },
        }
    }
}

#[derive(Clone)]
struct ThreadDeadlockState {
    mutex_waiting: Option<usize>,
    sem_waiting: Option<usize>,
    sem_held: Vec<usize>,
}

/// 进程内死锁检测辅助状态
pub(crate) struct DeadlockState {
    enabled: bool,
    mutex_owner: Vec<Option<ThreadId>>,
    semaphore_total: Vec<usize>,
    thread_state: BTreeMap<ThreadId, ThreadDeadlockState>,
}

impl DeadlockState {
    /// 创建空的死锁检测状态
    pub(crate) fn new() -> Self {
        Self {
            enabled: false,
            mutex_owner: Vec::new(),
            semaphore_total: Vec::new(),
            thread_state: BTreeMap::new(),
        }
    }

    #[inline]
    pub(crate) fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    #[inline]
    pub(crate) fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub(crate) fn register_mutex(&mut self, mutex_id: usize) {
        if self.mutex_owner.len() <= mutex_id {
            self.mutex_owner.resize(mutex_id + 1, None);
        }
        self.mutex_owner[mutex_id] = None;
    }

    pub(crate) fn register_semaphore(&mut self, sem_id: usize, total: usize) {
        if self.semaphore_total.len() <= sem_id {
            self.semaphore_total.resize(sem_id + 1, 0);
        }
        self.semaphore_total[sem_id] = total;
        let sem_count = self.semaphore_total.len();
        for state in self.thread_state.values_mut() {
            state.sem_held.resize(sem_count, 0);
        }
    }

    pub(crate) fn would_mutex_deadlock(&self, tid: ThreadId, mutex_id: usize) -> bool {
        let Some(Some(mut owner_tid)) = self.mutex_owner.get(mutex_id).copied() else {
            return false;
        };
        for _ in 0..=self.mutex_owner.len() {
            if owner_tid == tid {
                return true;
            }
            let Some(waiting_mutex) = self.thread_state.get(&owner_tid).and_then(|s| s.mutex_waiting)
            else {
                return false;
            };
            let Some(Some(next_owner)) = self.mutex_owner.get(waiting_mutex).copied() else {
                return false;
            };
            owner_tid = next_owner;
        }
        false
    }

    pub(crate) fn note_mutex_acquired(&mut self, tid: ThreadId, mutex_id: usize) {
        self.register_mutex(mutex_id);
        self.ensure_thread_state(tid).mutex_waiting = None;
        self.mutex_owner[mutex_id] = Some(tid);
        self.prune_thread_state(tid);
    }

    pub(crate) fn note_mutex_blocked(&mut self, tid: ThreadId, mutex_id: usize) {
        self.register_mutex(mutex_id);
        self.ensure_thread_state(tid).mutex_waiting = Some(mutex_id);
    }

    pub(crate) fn note_mutex_released(&mut self, mutex_id: usize, waking_tid: Option<ThreadId>) {
        if self.mutex_owner.len() <= mutex_id {
            return;
        }
        let previous_owner = self.mutex_owner[mutex_id];
        match waking_tid {
            Some(tid) => {
                self.ensure_thread_state(tid).mutex_waiting = None;
                self.mutex_owner[mutex_id] = Some(tid);
                self.prune_thread_state(tid);
            }
            None => self.mutex_owner[mutex_id] = None,
        }
        if let Some(previous_owner) = previous_owner {
            self.prune_thread_state(previous_owner);
        }
    }

    pub(crate) fn note_condvar_wait_result(
        &mut self,
        tid: ThreadId,
        mutex_id: usize,
        acquired: bool,
        waking_tid: Option<ThreadId>,
    ) {
        self.note_mutex_released(mutex_id, waking_tid);
        if acquired {
            self.note_mutex_acquired(tid, mutex_id);
        } else {
            self.note_mutex_blocked(tid, mutex_id);
        }
    }

    pub(crate) fn would_semaphore_deadlock(&self, tid: ThreadId, sem_id: usize) -> bool {
        if sem_id >= self.semaphore_total.len() {
            return false;
        }
        let mut work = self.semaphore_total.clone();
        for state in self.thread_state.values() {
            for (idx, held) in state.sem_held.iter().enumerate() {
                if work[idx] < *held {
                    return true;
                }
                work[idx] -= *held;
            }
        }
        if work[sem_id] > 0 {
            return false;
        }

        let mut states = self.thread_state.clone();
        let sem_count = self.semaphore_total.len();
        if let Some(state) = states.get_mut(&tid) {
            state.sem_held.resize(sem_count, 0);
            state.sem_waiting = Some(sem_id);
        } else {
            states.insert(
                tid,
                ThreadDeadlockState {
                    mutex_waiting: None,
                    sem_waiting: Some(sem_id),
                    sem_held: vec![0; sem_count],
                },
            );
        }

        let mut finished: BTreeMap<ThreadId, bool> = BTreeMap::new();
        loop {
            let mut progressed = false;
            let mut has_unfinished = false;
            for (thread_id, state) in states.iter() {
                if finished.get(thread_id).copied().unwrap_or(false) {
                    continue;
                }
                has_unfinished = true;
                let can_finish = match state.sem_waiting {
                    Some(waiting_sem) => work.get(waiting_sem).copied().unwrap_or(0) > 0,
                    None => true,
                };
                if can_finish {
                    for (idx, held) in state.sem_held.iter().enumerate() {
                        work[idx] += *held;
                    }
                    finished.insert(*thread_id, true);
                    progressed = true;
                }
            }
            if !has_unfinished {
                return false;
            }
            if !progressed {
                return true;
            }
        }
    }

    pub(crate) fn note_semaphore_acquired(&mut self, tid: ThreadId, sem_id: usize) {
        if sem_id >= self.semaphore_total.len() {
            return;
        }
        let state = self.ensure_thread_state(tid);
        state.sem_waiting = None;
        state.sem_held[sem_id] += 1;
    }

    pub(crate) fn note_semaphore_blocked(&mut self, tid: ThreadId, sem_id: usize) {
        if sem_id >= self.semaphore_total.len() {
            return;
        }
        self.ensure_thread_state(tid).sem_waiting = Some(sem_id);
    }

    pub(crate) fn note_semaphore_up(
        &mut self,
        tid: ThreadId,
        sem_id: usize,
        waking_tid: Option<ThreadId>,
    ) {
        if sem_id >= self.semaphore_total.len() {
            return;
        }
        {
            let state = self.ensure_thread_state(tid);
            if state.sem_held[sem_id] > 0 {
                state.sem_held[sem_id] -= 1;
            }
        }
        self.prune_thread_state(tid);
        if let Some(waking_tid) = waking_tid {
            let waking = self.ensure_thread_state(waking_tid);
            waking.sem_waiting = None;
            waking.sem_held[sem_id] += 1;
        }
    }

    pub(crate) fn note_thread_exit(&mut self, tid: ThreadId) {
        if let Some(state) = self.thread_state.get_mut(&tid) {
            state.mutex_waiting = None;
            state.sem_waiting = None;
        }
        self.prune_thread_state(tid);
    }

    fn ensure_thread_state(&mut self, tid: ThreadId) -> &mut ThreadDeadlockState {
        let sem_count = self.semaphore_total.len();
        self.thread_state.entry(tid).or_insert_with(|| ThreadDeadlockState {
            mutex_waiting: None,
            sem_waiting: None,
            sem_held: vec![0; sem_count],
        })
    }

    fn prune_thread_state(&mut self, tid: ThreadId) {
        let can_remove = self.thread_state.get(&tid).map(|state| {
            state.mutex_waiting.is_none()
                && state.sem_waiting.is_none()
                && state.sem_held.iter().all(|held| *held == 0)
                && !self.mutex_owner.iter().any(|owner| owner == &Some(tid))
        }).unwrap_or(false);
        if can_remove {
            self.thread_state.remove(&tid);
        }
    }
}

/// 进程（资源容器）
///
/// 管理地址空间、文件描述符、同步原语、信号等共享资源。
/// 一个进程可以包含多个线程。
pub struct Process {
    /// 进程 ID
    pub pid: ProcId,
    /// 地址空间（所有线程共享）
    pub address_space: AddressSpace<Sv39, Sv39Manager>,
    /// 文件描述符表（所有线程共享）
    pub fd_table: Vec<Option<Mutex<Fd>>>,
    /// 堆底地址
    pub heap_bottom: usize,
    /// 当前程序 break 位置（堆顶）
    pub program_brk: usize,
    /// 信号处理器
    pub signal: Box<dyn Signal>,
    /// 信号量列表（**本章新增**，所有线程共享）
    pub semaphore_list: Vec<Option<Arc<Semaphore>>>,
    /// 互斥锁列表（**本章新增**，所有线程共享）
    pub mutex_list: Vec<Option<Arc<dyn MutexTrait>>>,
    /// 条件变量列表（**本章新增**，所有线程共享）
    pub condvar_list: Vec<Option<Arc<Condvar>>>,
    /// 死锁检测辅助状态（**本章练习**）
    pub(crate) deadlock: DeadlockState,
}

impl Process {
    /// exec：替换当前进程的地址空间和主线程上下文
    ///
    /// 注意：只支持单线程进程执行 exec
    pub fn exec(&mut self, elf: ElfFile) {
        let (proc, thread) = Process::from_elf(elf).unwrap();
        self.address_space = proc.address_space;
        self.heap_bottom = proc.heap_bottom;
        self.program_brk = proc.program_brk;
        let processor: *mut ProcessorInner = PROCESSOR.get_mut() as *mut ProcessorInner;
        unsafe {
            let pthreads = (*processor).get_thread(self.pid).unwrap();
            (*processor).get_task(pthreads[0]).unwrap().context = thread.context;
        }
    }

    /// fork：创建子进程（复制地址空间和主线程上下文）
    ///
    /// 子进程继承父进程的地址空间（深拷贝）、文件描述符和信号配置。
    /// 同步原语列表不继承（子进程创建空的列表）。
    pub fn fork(&mut self) -> Option<(Self, Thread)> {
        let pid = ProcId::new();
        // 深拷贝地址空间
        let parent_addr_space = &self.address_space;
        let mut address_space: AddressSpace<Sv39, Sv39Manager> = AddressSpace::new();
        parent_addr_space.cloneself(&mut address_space);
        map_portal(&address_space);
        // 复制主线程上下文
        let processor: *mut ProcessorInner = PROCESSOR.get_mut() as *mut ProcessorInner;
        let pthreads = unsafe { (*processor).get_thread(self.pid).unwrap() };
        let context = unsafe {
            (*processor).get_task(pthreads[0]).unwrap().context.context.clone()
        };
        let satp = (8 << 60) | address_space.root_ppn().val();
        let thread = Thread::new(satp, context);
        // 复制文件描述符表
        let new_fd_table: Vec<Option<Mutex<Fd>>> = self.fd_table
            .iter()
            .map(|fd| fd.as_ref().map(|f| Mutex::new(f.lock().clone())))
            .collect();
        Some((
            Self {
                pid,
                address_space,
                fd_table: new_fd_table,
                heap_bottom: self.heap_bottom,
                program_brk: self.program_brk,
                signal: self.signal.from_fork(),
                // 子进程的同步原语列表初始为空
                semaphore_list: Vec::new(),
                mutex_list: Vec::new(),
                condvar_list: Vec::new(),
                deadlock: DeadlockState::new(),
            },
            thread,
        ))
    }

    /// 从 ELF 文件创建进程和主线程
    ///
    /// 解析 ELF 段，建立地址空间，分配用户栈，创建初始上下文。
    pub fn from_elf(elf: ElfFile) -> Option<(Self, Thread)> {
        let entry = match elf.header.pt2 {
            HeaderPt2::Header64(pt2)
                if pt2.type_.as_type() == header::Type::Executable
                    && pt2.machine.as_machine() == Machine::RISC_V =>
            { pt2.entry_point as usize }
            _ => None?,
        };

        const PAGE_SIZE: usize = 1 << Sv39::PAGE_BITS;
        const PAGE_MASK: usize = PAGE_SIZE - 1;

        let mut address_space = AddressSpace::new();
        let mut max_end_va: usize = 0;
        for program in elf.program_iter() {
            if !matches!(program.get_type(), Ok(program::Type::Load)) { continue; }
            let off_file = program.offset() as usize;
            let len_file = program.file_size() as usize;
            let off_mem = program.virtual_addr() as usize;
            let end_mem = off_mem + program.mem_size() as usize;
            assert_eq!(off_file & PAGE_MASK, off_mem & PAGE_MASK);
            max_end_va = max_end_va.max(end_mem);
            let mut flags: [u8; 5] = *b"U___V";
            if program.flags().is_execute() { flags[1] = b'X'; }
            if program.flags().is_write() { flags[2] = b'W'; }
            if program.flags().is_read() { flags[3] = b'R'; }
            address_space.map(
                VAddr::new(off_mem).floor()..VAddr::new(end_mem).ceil(),
                &elf.input[off_file..][..len_file],
                off_mem & PAGE_MASK,
                parse_flags(unsafe { core::str::from_utf8_unchecked(&flags) }).unwrap(),
            );
        }
        let heap_bottom = VAddr::<Sv39>::new(max_end_va).ceil().base().val();
        // 分配 2 页用户栈
        let stack = unsafe {
            alloc_zeroed(Layout::from_size_align_unchecked(
                2 << Sv39::PAGE_BITS, 1 << Sv39::PAGE_BITS,
            ))
        };
        address_space.map_extern(
            VPN::new((1 << 26) - 2)..VPN::new(1 << 26),
            PPN::new(stack as usize >> Sv39::PAGE_BITS),
            build_flags("U_WRV"),
        );
        map_portal(&address_space);
        let satp = (8 << 60) | address_space.root_ppn().val();
        let mut context = LocalContext::user(entry);
        *context.sp_mut() = 1 << 38;
        let thread = Thread::new(satp, context);

        Some((
            Self {
                pid: ProcId::new(),
                address_space,
                fd_table: vec![
                    // stdin
                    Some(Mutex::new(Fd::Empty { read: true, write: false })),
                    // stdout
                    Some(Mutex::new(Fd::Empty { read: false, write: true })),
                    // stderr
                    Some(Mutex::new(Fd::Empty { read: false, write: true })),
                ],
                heap_bottom,
                program_brk: heap_bottom,
                signal: Box::new(SignalImpl::new()),
                semaphore_list: Vec::new(),
                mutex_list: Vec::new(),
                condvar_list: Vec::new(),
                deadlock: DeadlockState::new(),
            },
            thread,
        ))
    }

    /// 修改程序 break 位置（实现 sbrk 系统调用）
    pub fn change_program_brk(&mut self, size: isize) -> Option<usize> {
        let old_brk = self.program_brk;
        let new_brk = self.program_brk as isize + size;
        if new_brk < self.heap_bottom as isize {
            return None;
        }
        let new_brk = new_brk as usize;

        let old_brk_ceil = VAddr::<Sv39>::new(old_brk).ceil();
        let new_brk_ceil = VAddr::<Sv39>::new(new_brk).ceil();

        if size > 0 {
            if new_brk_ceil.val() > old_brk_ceil.val() {
                self.address_space
                    .map(old_brk_ceil..new_brk_ceil, &[], 0, build_flags("U_WRV"));
            }
        } else if size < 0 {
            if old_brk_ceil.val() > new_brk_ceil.val() {
                self.address_space.unmap(new_brk_ceil..old_brk_ceil);
            }
        }

        self.program_brk = new_brk;
        Some(old_brk)
    }
}
