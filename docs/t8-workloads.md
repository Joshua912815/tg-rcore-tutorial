# T8 Workloads 与回归入口

这份文档记录 T8 新增的用户态 workload、它们对应的观测目标，以及配套回归脚本。

## 1. ch3：时间片与系统调用观测

- user workload: `tg-rcore-tutorial-user/src/bin/t8_ch3_observe.rs`
- 目标行为：
  - 主动 `sched_yield`
  - 调用 `sleep/get_time`
  - 产生明确成功标记 `T8 ch3 observe OK!`
- 期望日志：
  - `[EVENT][ch3][schedule] ...`
  - `[EVENT][ch3][syscall] ...`
  - `[METRIC][ch3][timer_events] ...`

## 2. ch4：虚存与 trace 观测

- user workload: `tg-rcore-tutorial-user/src/bin/t8_ch4_vm_probe.rs`
- 目标行为：
  - `mmap` 只读页
  - `trace_read/trace_write`
  - `munmap`
  - 产生明确成功标记 `T8 ch4 vm observe OK!`
- 期望日志：
  - `[EVENT][ch4][syscall] ...`
  - `[EVENT][ch4][vm] ...`
  - `[METRIC][ch4][vm_fault_events] ...`

说明：`[EVENT][ch4][vm]` 由章节已有 fault workload 共同提供，T8 workload 负责提供成功路径的可读样例。

## 3. ch8：同步与死锁检测观测

- user workload:
  - `tg-rcore-tutorial-user/src/bin/t8_ch8_usertest.rs`
  - 串联运行 `sync_sem`、`test_condvar`、`ch8_deadlock_mutex1`
- 目标行为：
  - 触发 semaphore 阻塞/唤醒
  - 触发 mutex 阻塞/唤醒
  - 触发 condvar 等待/唤醒
  - 触发 deadlock detect 拒绝
  - 产生明确成功标记 `T8 ch8 Usertests passed!`
- 期望日志：
  - `[EVENT][ch8][sync] mutex_lock ...`
  - `[EVENT][ch8][sync] sem_down ...`
  - `[EVENT][ch8][sync] condvar_wait ...`
  - `[EVENT][ch8][sync] deadlock-detect enabled=true`
  - `[METRIC][ch8][sync_blocked_events] ...`

## 4. 回归脚本

- 本地回归：`scripts/t8-regression.sh`
- Docker 回归：`scripts/t8-docker-regression.sh`

推荐命令：

```bash
scripts/t8-regression.sh all
scripts/t8-docker-regression.sh rcore-docker all
```
