# T2L21: ch3 Dynamic State Observatory

`tg-rcore-tutorial-ch3-observe` 是一个基于 `ch3` 的独立观测版 crate。它不追求新增 OS 功能，而是把第三章多道程序执行时最关键的状态变化记录出来，便于回答四类问题：

1. 正常执行时，任务是怎样进入、发起 syscall、yield、退出的？
2. 出现用户态异常时，`scause / sepc / stval` 怎么变化？
3. 在特定执行点打断时，内核如何记录断点并恢复执行？
4. 发生受控崩溃时，如何保留一条能解释路径的 breadcrumb 调用链？

## 我们自己的实现

这个 crate 没有复用 baseline 的 backtrace 实现，而是采用了更轻量的教学型方案：

- `src/observe.rs`
  - 维护每个任务的状态快照
  - 输出统一的 `[OBS][...]` 日志
  - 在 panic 前打印 breadcrumb 调用链
- `src/main.rs`
  - 对 `SupervisorTimer / UserEnvCall / Breakpoint / 普通异常 / panic` 分别布置观测点
  - 给 `trace` 扩展了两个调试请求：
    - `trace(3, ..)`：转储当前任务快照
    - `trace(4, ..)`：触发受控崩溃
- `tg-rcore-tutorial-user/src/bin/t21_ch3_*.rs`
  - `t21_ch3_normal`：正常路径和特定执行点快照
  - `t21_ch3_breakpoint`：用户态断点恢复
  - `t21_ch3_crash`：受控崩溃与 panic breadcrumb

## 观测日志格式

核心日志前缀如下：

- `[OBS][load]`：任务装载
- `[OBS][enter]`：进入或恢复用户态
- `[OBS][syscall]`：系统调用边界
- `[OBS][yield]`：主动让出 CPU
- `[OBS][breakpoint]`：命中 `ebreak`
- `[OBS][exception]`：用户态异常
- `[OBS][snapshot]`：完整任务快照
- `[OBS][metric]`：整轮运行摘要
- `[OBS][panic]` / `[OBS][crumb]`：panic 前的调用链

## 运行方式

在仓库根目录执行：

```bash
bash scripts/t21-docker-regression.sh rcore-docker all
```

或者直接在 crate 目录执行：

```bash
cd tg-rcore-tutorial-ch3-observe
bash test.sh all
```

`test.sh` 会产出两个日志：

- `target/t21/ch3-observe-base.log`
- `target/t21/ch3-observe-crash.log`

## GDB 复现

```bash
cd tg-rcore-tutorial-ch3-observe
bash scripts/launch-qemu-gdb.sh
```

另开一个终端：

```bash
cd tg-rcore-tutorial-ch3-observe
riscv64-unknown-elf-gdb -x gdb/t2l21.gdb
```

建议重点观察：

- `rust_main`
- `panic_stage1`
- `task::TaskControlBlock::handle_syscall`

## 已验证的实际表现

来自 `target/t21/ch3-observe-base.log`：

- `T21 ch3 breakpoint resumed`
- `T21 ch3 normal OK!`
- `Test sleep OK!`
- `[OBS][metric] reason=summary tasks=5 syscalls=193 trace_requests=3 yields=80 timer_interrupts=0 breakpoints=1 exceptions=1 exits=4`

来自 `target/t21/ch3-observe-crash.log`：

- `T21 crash trigger`
- `[OBS][snapshot] reason=trace-crash ...`
- `[OBS][panic] breadcrumb_depth=4`
- `[OBS][crumb] #0 schedule_loop`
- `[OBS][crumb] #1 trace_syscall`
- `[OBS][crumb] #2 panic_stage1`
- `[OBS][crumb] #3 panic_stage2`
