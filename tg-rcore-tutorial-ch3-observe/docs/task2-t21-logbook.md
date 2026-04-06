# T2L21 开发记录

## 目标

把 `ch3` 做成一个可独立复现的动态状态观测实验，覆盖：

- 正常路径
- 用户态异常
- 用户态断点
- 受控崩溃

## 实现摘要

- 新增 `TaskObservation` 统一状态模型
- 新增 `snapshot + delta` 观测输出
- 新增 `trace(3)` 和 `trace(4)` 两类调试请求
- 新增 breakpoint 恢复日志与指令长度判断
- 新增 crash breadcrumb 日志
- 新增本地回归、Docker 回归与 GDB 入口

## 实际证据

基础日志 `target/t21/ch3-observe-base.log` 中可以看到：

- `T21 ch3 breakpoint resumed`
- `T21 ch3 normal OK!`
- `[OBS][breakpoint] task=4 step=4 inst16=0x73`
- `[OBS][delta] task=...`
- `[OBS][metric] reason=summary tasks=5 syscalls=193 trace_requests=3 yields=80 timer_interrupts=0 breakpoints=1 exceptions=1 exits=4`

崩溃日志 `target/t21/ch3-observe-crash.log` 中可以看到：

- `T21 crash trigger`
- `[OBS][snapshot] reason=trace-crash`
- `[OBS][panic] breadcrumb_depth=4`
- `[OBS][crumb] #0 schedule_loop`
- `[OBS][crumb] #1 trace_syscall`
- `[OBS][crumb] #2 panic_stage1`
- `[OBS][crumb] #3 panic_stage2`
