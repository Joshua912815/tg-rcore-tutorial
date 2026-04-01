# T2L21 开发记录

## 目标

本轮实验选择 `T2L21`，目标是做一个的 `ch3` 动态执行状态可观察性版本。实现重点不是源码级符号回溯，而是构建一套可以稳定复现的观测流程：

- 正常路径：任务装载、进入、syscall、yield、退出
- 异常路径：用户态异常时的 `scause / sepc / stval`
- 特定执行点：`ebreak` 命中与恢复
- 崩溃路径：受控 panic 与 breadcrumb 调用链

## 实现概览

新增 crate：

- `tg-rcore-tutorial-ch3-observe`

关键文件：

- `tg-rcore-tutorial-ch3-observe/src/observe.rs`
- `tg-rcore-tutorial-ch3-observe/src/main.rs`
- `tg-rcore-tutorial-ch3-observe/src/task.rs`
- `tg-rcore-tutorial-user/src/bin/t21_ch3_normal.rs`
- `tg-rcore-tutorial-user/src/bin/t21_ch3_breakpoint.rs`
- `tg-rcore-tutorial-user/src/bin/t21_ch3_crash.rs`
- `scripts/t21-docker-regression.sh`

## 实际观测结果

基础场景日志：`target/t21/ch3-observe-base.log`

- 断点恢复成功：
  - `T21 ch3 breakpoint resumed`
- 正常路径成功：
  - `T21 ch3 normal OK!`
- 异常路径成功：
  - `[OBS][exception] task=2 reason=store-fault ...`
- 断点细节成功：
  - `[OBS][breakpoint] task=4 step=4 inst16=0x73`
- 运行摘要成功：
  - `[OBS][metric] reason=summary tasks=5 syscalls=193 trace_requests=3 yields=80 timer_interrupts=0 breakpoints=1 exceptions=1 exits=4`

崩溃场景日志：`target/t21/ch3-observe-crash.log`

- 受控崩溃触发成功：
  - `T21 crash trigger`
- panic 前快照成功：
  - `[OBS][snapshot] reason=trace-crash ...`
- breadcrumb 成功：
  - `[OBS][panic] breadcrumb_depth=4`
  - `[OBS][crumb] #0 schedule_loop`
  - `[OBS][crumb] #1 trace_syscall`
  - `[OBS][crumb] #2 panic_stage1`
  - `[OBS][crumb] #3 panic_stage2`

## Docker 验证

本轮在 `rcore-docker` 中验证，使用持久容器 `my-rcore-t21` 复用工具链缓存。

建议复现命令：

```bash
bash scripts/t21-docker-regression.sh rcore-docker all
```
