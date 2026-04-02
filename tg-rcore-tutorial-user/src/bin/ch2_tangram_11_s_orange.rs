#![no_std]
#![no_main]

extern crate user_lib;

#[unsafe(no_mangle)]
extern "C" fn main() -> i32 {
    user_lib::run_tangram_piece_app(11, "S orange parallelogram")
}
