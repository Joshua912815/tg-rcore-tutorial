#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;

use user_lib::trace;

#[unsafe(no_mangle)]
extern "C" fn main() -> i32 {
    println!("T21 crash trigger");
    assert_eq!(0, trace(3, 0, 0));
    trace(4, 0, 0x21);
    0
}
