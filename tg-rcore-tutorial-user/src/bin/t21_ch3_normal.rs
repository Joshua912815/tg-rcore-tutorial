#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;

use user_lib::{get_time, sched_yield, trace};

#[unsafe(no_mangle)]
extern "C" fn main() -> i32 {
    println!("T21 normal workload start");
    assert_eq!(0, trace(3, 0, 0));

    let start = get_time();
    for round in 0..3 {
        sched_yield();
        let elapsed = get_time() - start;
        println!("T21 normal round={round} elapsed={elapsed}");
    }

    assert_eq!(0, trace(3, 0, start as usize));
    println!("T21 ch3 normal OK!");
    0
}
