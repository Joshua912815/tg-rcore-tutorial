#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;

use core::sync::atomic::{AtomicUsize, Ordering};
use user_lib::{
    condvar_create, condvar_signal, condvar_wait, enable_deadlock_detect, exit, mutex_create,
    mutex_lock, mutex_unlock, sched_yield, semaphore_create, semaphore_down, semaphore_up, sleep,
    thread_create, waittid,
};

const SEM_ID: usize = 0;
const BLOCK_MUTEX_ID: usize = 0;
const CV_ID: usize = 0;
const CV_MUTEX_ID: usize = 1;
const DEADLOCK_MUTEX_ID: usize = 2;

static MUTEX_READY: AtomicUsize = AtomicUsize::new(0);
static SEM_READY: AtomicUsize = AtomicUsize::new(0);
static CV_PHASE: AtomicUsize = AtomicUsize::new(0);
static CV_READY: AtomicUsize = AtomicUsize::new(0);

// 教学目标：
// 用一个最小同步组合 workload 依次触发 mutex/semaphore/condvar/deadlock-detect，
// 让 ch8 的统一 sync tracing 有明确、可重复的用户态入口。

fn lock_blocking(id: usize) {
    while mutex_lock(id) != 0 {
        sched_yield();
    }
}

fn down_blocking(id: usize) {
    while semaphore_down(id) != 0 {
        sched_yield();
    }
}

unsafe fn mutex_waiter() -> isize {
    println!("T8 ch8 workload: mutex waiter start");
    lock_blocking(BLOCK_MUTEX_ID);
    MUTEX_READY.store(1, Ordering::SeqCst);
    println!("T8 ch8 workload: mutex waiter acquired");
    assert_eq!(mutex_unlock(BLOCK_MUTEX_ID), 0);
    exit(0)
}

unsafe fn sem_waiter() -> isize {
    println!("T8 ch8 workload: semaphore waiter start");
    down_blocking(SEM_ID);
    SEM_READY.store(1, Ordering::SeqCst);
    println!("T8 ch8 workload: semaphore waiter acquired");
    exit(0)
}

unsafe fn condvar_waiter() -> isize {
    println!("T8 ch8 workload: condvar waiter start");
    lock_blocking(CV_MUTEX_ID);
    while CV_READY.load(Ordering::SeqCst) == 0 {
        CV_PHASE.store(1, Ordering::SeqCst);
        let _ = condvar_wait(CV_ID, CV_MUTEX_ID);
    }
    assert_eq!(mutex_unlock(CV_MUTEX_ID), 0);
    println!("T8 ch8 workload: condvar waiter resumed");
    exit(0)
}

#[unsafe(no_mangle)]
extern "C" fn main() -> i32 {
    println!("T8 ch8 workload: start");

    assert_eq!(mutex_create(true) as usize, BLOCK_MUTEX_ID);
    assert_eq!(mutex_create(true) as usize, CV_MUTEX_ID);
    assert_eq!(mutex_create(true) as usize, DEADLOCK_MUTEX_ID);
    assert_eq!(semaphore_create(0) as usize, SEM_ID);
    assert_eq!(condvar_create() as usize, CV_ID);

    assert_eq!(mutex_lock(BLOCK_MUTEX_ID), 0);
    let mutex_tid = thread_create(mutex_waiter as *const () as usize, 0) as usize;
    sleep(20);
    assert_eq!(MUTEX_READY.load(Ordering::SeqCst), 0);
    assert_eq!(mutex_unlock(BLOCK_MUTEX_ID), 0);
    assert_eq!(waittid(mutex_tid), 0);
    assert_eq!(MUTEX_READY.load(Ordering::SeqCst), 1);

    let sem_tid = thread_create(sem_waiter as *const () as usize, 0) as usize;
    sleep(20);
    assert_eq!(SEM_READY.load(Ordering::SeqCst), 0);
    assert_eq!(semaphore_up(SEM_ID), 0);
    assert_eq!(waittid(sem_tid), 0);
    assert_eq!(SEM_READY.load(Ordering::SeqCst), 1);

    let cv_tid = thread_create(condvar_waiter as *const () as usize, 0) as usize;
    while CV_PHASE.load(Ordering::SeqCst) == 0 {
        sleep(1);
    }
    assert_eq!(mutex_lock(CV_MUTEX_ID), 0);
    CV_READY.store(1, Ordering::SeqCst);
    assert_eq!(condvar_signal(CV_ID), 0);
    assert_eq!(mutex_unlock(CV_MUTEX_ID), 0);
    assert_eq!(waittid(cv_tid), 0);

    assert_eq!(enable_deadlock_detect(true), 0);
    assert_eq!(mutex_lock(DEADLOCK_MUTEX_ID), 0);
    assert_eq!(mutex_lock(DEADLOCK_MUTEX_ID), -0xdead);
    assert_eq!(mutex_unlock(DEADLOCK_MUTEX_ID), 0);
    assert_eq!(enable_deadlock_detect(false), 0);

    println!("T8 ch8 sync observe OK!");
    0
}
