#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;

use user_lib::{exec, fork, waitpid};

const TESTS: &[&str] = &["sync_sem", "test_condvar", "ch8_deadlock_mutex1"];

// 教学目标：
// 为 T8 的 ch8 回归提供一个独立入口，只运行最小同步观测 workload，
// 避免与原有 exercise 大测试集相互干扰。

#[unsafe(no_mangle)]
extern "C" fn main() -> i32 {
    let mut pids = [0isize; TESTS.len()];
    for (i, &test) in TESTS.iter().enumerate() {
        println!("T8 Usertests: Running {}", test);
        let pid = fork();
        if pid == 0 {
            exec(test);
            panic!("unreachable!");
        } else {
            pids[i] = pid;
        }
    }

    let mut xstate = 0;
    for (i, &test) in TESTS.iter().enumerate() {
        let wait_pid = waitpid(pids[i], &mut xstate);
        assert_eq!(pids[i], wait_pid);
        println!(
            "\x1b[32mT8 Usertests: Test {} in Process {} exited with code {}\x1b[0m",
            test, pids[i], xstate
        );
        assert_eq!(xstate, 0);
    }

    println!("T8 ch8 Usertests passed!");
    0
}
