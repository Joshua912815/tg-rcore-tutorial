# T2L21: ch3 Dynamic State Observatory

`joshua912815-tg-rcore-tutorial-t2l21` 是一个可独立复现的 `T2L21` 实验 crate。它基于 `ch3`，目标不是新增内核功能，而是让第三章多道程序的关键动态状态可以被稳定复现、脚本化验证，并能直接用于教学和自学。

## Crate 信息

- crate 名称：`joshua912815-tg-rcore-tutorial-t2l21`
- 版本：`0.21.0-preview.3`
- 仓库：[Joshua912815/tg-rcore-tutorial](https://github.com/Joshua912815/tg-rcore-tutorial)
- 分支：`t2l21-observe`
- 建议 tag：`joshua912815-tg-rcore-tutorial-t2l21-v0.21.0-preview.3`
- keywords：`ai` `ai4ose` `kernel` `learning` `os`

## 解决的问题

这个实验重点回答四类问题：

1. 正常执行时，任务是怎样进入、发起 syscall、yield、退出的？
2. 出现用户态异常时，`scause / sepc / stval` 怎么变化？
3. 在特定执行点打断时，内核如何记录断点并恢复执行？
4. 发生受控崩溃时，如何保留一条足够解释控制路径的调用线索？

## 相比 baseline 的改进

本实验参考了 baseline 的题意，但没有照搬其实现，而是做了三项更适合教学和脚本化复现的改进：

1. 任务级状态模型。不是只在局部打印零散信息，而是为每个任务维护统一的 `TaskObservation`。
2. `snapshot + delta` 双层观测。除了完整快照，还打印相对上一次快照的状态增量，直接降低日志比对成本。
3. 指令长度感知的断点恢复。命中 `ebreak` 后不是固定 `sepc += 4`，而是根据指令编码判断步长是 `2` 还是 `4`。

这三点的价值在于：观测结果更稳定、日志更容易分析、后续迁移到 `ch4/ch5` 也更方便。

## 目录结构

- `src/`：观测版内核实现
- `bundle/tg-rcore-tutorial-user.tar.gz`：随 crate 打包的最小 user workload bundle，构建时自动解包
- `docs/`：实验说明、开发记录、设计总结
- `scripts/`：GDB 与 Docker 复现脚本
- `report.md`：最终提交版报告入口
- `test.sh`：本地一键回归脚本

## 复现方式

### 方式一：crates.io

```bash
cargo clone joshua912815-tg-rcore-tutorial-t2l21
cd joshua912815-tg-rcore-tutorial-t2l21
make run
```

如果本机没有完整 RISC-V/QEMU 工具链，可以改用 Docker：

```bash
make run-docker IMAGE=rcore-docker
```

### 方式二：git 仓库

```bash
git clone https://github.com/Joshua912815/tg-rcore-tutorial
cd tg-rcore-tutorial
git checkout t2l21-observe
cd tg-rcore-tutorial-ch3-observe
make run
```

## 运行结果

`make run` 会执行 `bash test.sh all`，产生两份日志：

- `target/t21/ch3-observe-base.log`
- `target/t21/ch3-observe-crash.log`

已验证的关键输出包括：

- `T21 ch3 breakpoint resumed`
- `T21 ch3 normal OK!`
- `Test sleep OK!`
- `[OBS][delta] task=...`
- `[OBS][metric] reason=summary tasks=5 syscalls=193 trace_requests=3 yields=80 timer_interrupts=0 breakpoints=1 exceptions=1 exits=4`
- `T21 crash trigger`
- `[OBS][snapshot] reason=trace-crash`
- `[OBS][panic] breadcrumb_depth=4`
- `[OBS][crumb] #3 panic_stage2`

## GDB 复现

```bash
make gdb
```

另开一个终端：

```bash
riscv64-unknown-elf-gdb -x gdb/t2l21.gdb
```

建议重点观察：

- `rust_main`
- `panic_stage1`
- `task::TaskControlBlock::handle_syscall`

## 文档入口

- [最终报告](report.md)
- [设计总结](docs/task2-t21-report.md)
- [开发记录](docs/task2-t21-logbook.md)
