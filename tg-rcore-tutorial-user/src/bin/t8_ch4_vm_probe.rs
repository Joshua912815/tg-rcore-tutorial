#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;

use user_lib::{mmap, munmap, trace_read, trace_write};

// 教学目标：
// 用一个短小、可成功结束的 workload 触发 ch4 的 mmap/munmap/trace 路径，
// 与已有 fault 样例一起组成“可成功 + 可失败”的虚存观测对照。

#[unsafe(no_mangle)]
extern "C" fn main() -> i32 {
    const START: usize = 0x1000_2000;
    const LEN: usize = 4096;
    const READONLY: usize = 1;

    println!("T8 ch4 workload: start");

    assert_eq!(0, mmap(START, LEN, READONLY));
    assert!(trace_read(START as *const u8).is_some());
    assert_eq!(-1, trace_write(START as *const u8, 7));
    assert_eq!(0, munmap(START, LEN));
    assert_eq!(None, trace_read(START as *const u8));

    println!("T8 ch4 vm observe OK!");
    0
}
