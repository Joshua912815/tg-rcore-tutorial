#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;

use user_lib::{count_syscall, get_time, sched_yield, sleep};

const SYS_WRITE: usize = 64;
const SYS_SCHED_YIELD: usize = 124;
const SYS_CLOCK_GETTIME: usize = 113;

// 教学目标：
// 用最小用户态 workload 同时触发 write / clock_gettime / yield，
// 让 ch3 的 [EVENT]/[METRIC] 输出有一个可重复、可命名的观测样本。

#[unsafe(no_mangle)]
extern "C" fn main() -> i32 {
    println!("T8 ch3 workload: start");
    let start = get_time();
    sched_yield();
    sleep(30);
    let end = get_time();
    println!("T8 ch3 workload: delta={}ms", end - start);

    assert!(count_syscall(SYS_CLOCK_GETTIME) >= 2);
    assert!(count_syscall(SYS_SCHED_YIELD) > 0);
    assert!(count_syscall(SYS_WRITE) >= 2);

    println!("T8 ch3 observe OK!");
    0
}
