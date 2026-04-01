#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;

use user_lib::trace;

#[unsafe(no_mangle)]
extern "C" fn main() -> i32 {
    println!("T21 breakpoint workload start");
    assert_eq!(0, trace(3, 0, 0));
    unsafe {
        core::arch::asm!(".4byte 0x00100073");
    }
    println!("T21 ch3 breakpoint resumed");
    0
}
