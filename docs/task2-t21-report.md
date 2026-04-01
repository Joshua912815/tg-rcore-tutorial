# T2L21 设计总结报告

## 1. 设计思路与目标

### 1.1 实验目标

`T2L21` 的题目是“基于 ch3 实验的 OS 动态执行状态可观察性实验”。核心要求不是增加一个新的内核功能，而是让第三章多道程序系统在以下四类场景下都能被清楚观察：

1. 正常执行
2. 异常执行
3. 崩溃执行
4. 特定执行点

具体到 `ch3`，我认为真正需要被观察的关键状态不是泛泛的“程序运行了”，而是：

- 当前任务是谁
- 任务是否首次进入还是被恢复
- 当前 trap 原因是什么
- 最近一次 syscall 是什么
- 当前任务已经经历了多少次 enter / syscall / yield / exception / exit
- 异常时的 `scause / sepc / stval`
- 特定执行点命中后，内核如何恢复用户态继续执行
- 崩溃前，内核自己正在走哪条控制路径

因此，我把实验目标收敛为一句话：

> 为 `ch3` 建立一套“任务级状态快照 + 状态增量 + 可脚本化复现”的观测体系。

### 1.2 适合自己的学习方式

我比较不适合只看教程文字推演，也不太适合只依赖 GDB 单步。对我更有效的学习方式是：

1. 先把系统运行中的关键状态显式打印出来
2. 再把这些状态按“正常 / 异常 / 崩溃 / 断点”分场景跑出来
3. 最后把日志脚本化，避免每次都人工盯 QEMU 输出

也就是说，我更依赖“能复现、能比对、能自动检查”的实验环境，而不是只依赖一次性的人工观察。

这也是本次实验没有直接照着 baseline 去做源码级 backtrace 的原因。源码级 backtrace 很强，但实现复杂、依赖多，第一次做很容易把注意力从“理解 ch3 的状态变化”转移到“调试符号解析和调用栈恢复细节”上。对当前学习阶段来说，我更需要一套稳定、低耦合、能长期复用到后续章节的观测框架。

### 1.3 初步设想与规划

最初的规划分为四步：

1. 从 `test` 分支单独切出 `t2l21-observe`，避免和 `T2L8` 混在一起
2. 建立独立 crate，而不是直接污染原 `tg-rcore-tutorial-ch3`
3. 先跑通三类 workload：正常、断点、崩溃
4. 最后补文档、GDB 指引、Docker 回归脚本和报告

最终实现路径与这个规划基本一致，只有一个重要变化：

- 原本我只准备做“状态快照”
- 在第一次日志验证后发现，快照虽然完整，但人工对比前后状态变化的成本仍然偏高
- 因此后续又新增了“状态增量 delta 输出”作为本实验的重要改进点

## 2. 实现概述

### 2.1 交付结构

本次 `T2L21` 的核心交付如下：

- 独立 crate：
  - `tg-rcore-tutorial-ch3-observe`
- 内核观测实现：
  - `tg-rcore-tutorial-ch3-observe/src/main.rs`
  - `tg-rcore-tutorial-ch3-observe/src/observe.rs`
  - `tg-rcore-tutorial-ch3-observe/src/task.rs`
- user workload：
  - `tg-rcore-tutorial-user/src/bin/t21_ch3_normal.rs`
  - `tg-rcore-tutorial-user/src/bin/t21_ch3_breakpoint.rs`
  - `tg-rcore-tutorial-user/src/bin/t21_ch3_crash.rs`
- case 配置：
  - `tg-rcore-tutorial-user/cases.toml`
- GDB 与回归脚本：
  - `tg-rcore-tutorial-ch3-observe/gdb/t2l21.gdb`
  - `tg-rcore-tutorial-ch3-observe/scripts/launch-qemu-gdb.sh`
  - `tg-rcore-tutorial-ch3-observe/test.sh`
  - `scripts/t21-docker-regression.sh`
- 文档：
  - `tg-rcore-tutorial-ch3-observe/README.md`
  - `docs/task2-t21-logbook.md`
  - `docs/task2-t21-report.md`

### 2.2 核心设计

本实验没有复用 baseline 的源码级 backtrace 实现，而是采用了下面这套自定义设计：

#### 设计一：任务级状态快照

在 `src/observe.rs` 中，我为每个任务维护了一份 `TaskObservation`，记录：

- `loaded_entry`
- `initial_sp`
- `enters`
- `syscalls`
- `trace_requests`
- `yields`
- `timer_interrupts`
- `breakpoints`
- `exceptions`
- `exits`
- `last_syscall`
- `last_scause`
- `last_sepc`
- `last_stval`

这份结构的好处是：它直接对应了 `ch3` 真正重要的动态状态，不需要读者自己在杂乱日志里手工拼上下文。

#### 设计二：状态增量 delta 输出

仅有完整快照还不够。快照适合“看全貌”，但不适合“看变化”。

因此我又增加了一份 `LAST_DUMPS`，在每次 `dump_task` 时除了打印完整快照，还会再打印一条 delta：

- `enters += ?`
- `syscalls += ?`
- `trace_requests += ?`
- `yields += ?`
- `exceptions += ?`
- `last_sepc old -> new`
- `last_scause old -> new`

这样一来，在特定执行点前后，读者不需要再手动比对两大串字段，只看 delta 就能知道“到底哪些变量变了”。

这是我认为本实验里最有含金量的改进之一，因为它真正降低了动态状态分析的成本。

#### 设计三：指令长度感知的断点恢复

在第一次实现时，我默认 `ebreak` 是 4 字节指令，命中断点后直接把 `sepc += 4`。结果实际运行时，QEMU 一直跑不完，说明返回用户态后 PC 恢复错了。

最终的修复方案是：

- 从 `sepc` 指向的指令地址读取 `u16`
- 判断当前指令到底是 16 位压缩指令还是 32 位普通指令
- 然后动态选择 `step = 2` 或 `step = 4`

对应日志例如：

```text
[OBS][breakpoint] task=4 step=4 inst16=0x73
```

这说明断点恢复不是“拍脑袋跳 4 字节”，而是根据当前指令编码做的恢复。这比一个固定偏移的演示版本更稳健。

#### 设计四：受控崩溃与 breadcrumb 调用链

我没有照 baseline 实现源码级回溯，而是引入了 breadcrumb 机制：

- `scope("schedule_loop")`
- `scope("trace_syscall")`
- `scope("panic_stage1")`
- `scope("panic_stage2")`

在 panic 时，把当前 breadcrumb 栈打印出来。对应崩溃日志里能看到：

```text
[OBS][panic] breadcrumb_depth=4
[OBS][crumb] #0 schedule_loop
[OBS][crumb] #1 trace_syscall
[OBS][crumb] #2 panic_stage1
[OBS][crumb] #3 panic_stage2
```

它不等价于源码级 backtrace，但有两个优点：

1. 实现简单、稳定、低耦合
2. 不依赖 DWARF 符号解析，后续迁移到其他章节更容易

对课程实验来说，这是一种更工程化、更容易长期维护的崩溃观测方式。

#### 设计五：专用 trace 请求

为了让“特定执行点观测”和“受控崩溃”不用依赖外部手动操作，我扩展了 `trace` 的两个请求：

- `trace(3, ..)`：打印当前任务快照
- `trace(4, ..)`：先打印 `trace-crash` 快照，再进入受控 panic

这样用户态 workload 就能主动触发可观测场景，而不是完全依赖人工在 GDB 里单步。

### 2.3 三类 workload 设计

#### 正常路径：`t21_ch3_normal`

目标：

- 触发普通 syscall
- 触发 yield
- 在关键执行点主动 dump 快照

成功标志：

- `T21 ch3 normal OK!`

#### 特定执行点：`t21_ch3_breakpoint`

目标：

- 在用户态触发 `ebreak`
- 由内核记录断点命中
- 自动恢复用户态继续执行

成功标志：

- `T21 ch3 breakpoint resumed`

#### 崩溃路径：`t21_ch3_crash`

目标：

- 先记录运行时快照
- 再触发受控 panic
- 输出 breadcrumb 调用链

成功标志：

- `T21 crash trigger`
- `[OBS][snapshot] reason=trace-crash`
- `[OBS][panic] breadcrumb_depth=...`

## 3. 与 baseline 的对比与改进

我阅读了 baseline 的目标与交付形态，但没有复用其代码实现。对比后，我认为本实验的改进主要有四点。

### 3.1 改进一：从“静态标签展示”升级为“任务级状态模型”

baseline 更强调“在关键路径打标签”和“展示教学知识点”。这种方式适合引导读者定位流程，但对“一个任务当前到底处于什么状态”描述得不够集中。

本实验把状态集中到 `TaskObservation`，并且所有场景都围绕同一套状态模型组织。

这带来的好处是：

- 正常、异常、断点、崩溃四类场景使用同一套字段
- 日志分析不再依赖人工拼上下文
- 更容易迁移到 `ch4/ch5/ch8`

### 3.2 改进二：增加状态 delta，降低日志分析成本

这是我认为相对 baseline 最实在的改进。

baseline 的日志更适合“看到发生了什么”；本实验新增的 delta 更适合“看变量到底变了多少”。

例如在基础场景日志里：

```text
[OBS][delta] reason=ebreak task=4 enters+=1 syscalls+=0 trace_requests+=0 yields+=0 timer_interrupts+=0 breakpoints+=1 exceptions+=0 exits+=0 last_syscall:410->410, last_scause:0x0->0x0, last_sepc:0x80c000e4->0x80c001d0, last_stval:0x0->0x0
```

这条日志一眼就说明：

- 这次事件只是断点，不是 syscall
- 进入次数加了 1
- 断点次数加了 1
- `sepc` 从快照点跳到了断点地址

如果没有 delta，读者需要自己对比两条完整快照才能得出这些结论。

### 3.3 改进三：断点恢复更稳健

baseline 的断点展示更多偏向“让你停在某个地方看”。本实验进一步处理了一个工程上很容易被忽略的问题：

> `ebreak` 恢复时，到底该跳过 2 字节还是 4 字节？

我在实现中根据 `sepc` 指向的实际指令编码决定步长，而不是写死一个偏移。这让断点恢复具备了更强的鲁棒性。

这类细节虽然不显眼，但是真正决定了“这个观测工具能不能稳定使用”。

### 3.4 改进四：自动化复现更强

baseline 更偏“演示型工程”。本实验进一步强调自动化复现。

当前可以直接执行：

```bash
bash scripts/t21-docker-regression.sh rcore-docker all
```

脚本会自动验证：

- normal workload completed
- breakpoint workload resumed
- breakpoint step recorded
- exception path recorded
- baseline sleep workload completed
- summary metric emitted
- crash workload started
- crash snapshot emitted
- panic breadcrumb header emitted
- panic breadcrumb tail emitted
- panic message emitted

也就是说，本实验不仅能“演示”，还能“回归检查”。

这是相对 baseline 的一个明显工程化提升。

### 3.5 客观说明：本实验没有做源码级 backtrace


baseline 的源码级 backtrace 能展示更细粒度的函数调用栈，这是它的优势。本实验没有复现这一点，而是使用 breadcrumb 替代。

但对当前任务来说，我认为这是合理取舍，因为：

1. breadcrumb 更稳定
2. 更低耦合
3. 更容易扩展到后续章节
4. 能把主要精力放在“ch3 状态变化理解”而不是符号解析

所以这不是简单的“功能更少”，而是“做了不同方向的工程优化”。

## 4. 实际结果与证据

### 4.1 基础场景

日志文件：

- `target/t21/ch3-observe-base.log`

关键结果：

```text
T21 ch3 breakpoint resumed
T21 ch3 normal OK!
Test sleep OK!
[OBS][metric] reason=summary tasks=5 syscalls=403 trace_requests=3 yields=185 timer_interrupts=0 breakpoints=1 exceptions=1 exits=4
```

说明：

- 正常 workload 完成
- 断点命中后成功恢复
- store fault 异常被正确记录并杀死对应任务
- 最终能输出全局摘要指标

### 4.2 崩溃场景

日志文件：

- `target/t21/ch3-observe-crash.log`

关键结果：

```text
T21 crash trigger
[OBS][snapshot] reason=trace-crash ...
[OBS][panic] breadcrumb_depth=4
[OBS][crumb] #0 schedule_loop
[OBS][crumb] #1 trace_syscall
[OBS][crumb] #2 panic_stage1
[OBS][crumb] #3 panic_stage2
```

说明：

- 崩溃不是随机 panic，而是受控触发
- panic 前的状态已保留
- 控制路径可解释

### 4.3 定量结果

在当前 base 场景中，最终摘要为：

- `tasks=5`
- `syscalls=403`
- `trace_requests=3`
- `yields=185`
- `breakpoints=1`
- `exceptions=1`
- `exits=4`

这说明本实验不是只覆盖一个单点事件，而是把 `ch3` 的主要运行路径都串起来了。

## 5. 与 AI 合作的实现过程

### 5.1 协作方式

本实验不是“把题目丢给 AI 然后直接收代码”，而是采用了分阶段协作：

1. 我先确定任务范围和分支隔离策略
2. AI 帮我阅读题目、查看仓库结构、分析 baseline 目标
3. 我要求 AI 不得抄袭 baseline 实现
4. AI 在独立分支上实现、验证、补文档
5. 我再根据运行结果要求继续增强和整理

这次协作里，我要求 AI 始终满足两个约束：

- 先分支隔离，再实现
- 任何结论都要有实际日志或脚本验证支持

### 5.2 关键问题与解决过程

#### 问题一：新 crate 构建时尝试 `cargo clone`

问题表现：

- `build.rs` 默认尝试在新 crate 目录下重新 clone user crate
- 本地环境缺少 `cargo clone`，直接失败

解决方法：

- 修改 `build.rs`
- 优先复用仓库根目录现成的 `tg-rcore-tutorial-user`

这是一个很典型的“实验代码能写，但复现性差”的问题。修掉后，新 crate 的可移植性明显提升。

#### 问题二：Docker 中 user workload 无法编译

问题表现：

- `tg-rcore-tutorial-syscall/src/user.rs` 中的 `asm!("ecall")`
- 在 2024 edition 下触发 `unsafe_op_in_unsafe_fn`

解决方法：

- 为 `syscall0..6` 里的 `asm!` 增加显式 `unsafe { ... }`

这个问题和 `T2L21` 逻辑本身无关，但它是整个实验链路真正的阻塞点。如果不修，后续所有 user workload 都无法运行。

#### 问题三：断点命中后程序跑不完

问题表现：

- `ebreak` 之后 QEMU 一直跑不结束

最初误判：

- 假设 `ebreak` 固定是 4 字节，直接 `sepc += 4`

最终定位：

- 需要根据 `sepc` 指向的实际指令编码判断步长

解决方法：

- 读取 `inst16`
- 根据编码决定 `step = 2` 或 `4`

这是本实验最关键的一次 bug 修复，也直接产出了相对 baseline 的一个真实改进点。

#### 问题四：快照虽然全，但不好读

问题表现：

- `snapshot` 信息很完整
- 但两次快照之间到底什么变了，还得人工比对

解决方法：

- 增加 `delta` 输出

这一步让日志从“可用”变成了“好用”。

### 5.3 对 AI 协作的评价

优点：

- 对代码搜索、快速试错、脚本补齐和 Docker 验证很高效
- 能把零散的工程工作收束成一个完整交付

局限：

- AI 初始实现有时会选择“能工作但解释性一般”的方案
- 如果没有我反复强调“不要抄 baseline”“要有含金量改进”，最终结果很容易停留在基础版本

所以更合理的用法不是“让 AI 替我完成实验”，而是：

> 把 AI 当成一个高效率实现与验证助手，而不是替代自己的设计判断。

## 6. 学习效果评估

### 6.1 知识和能力提升

通过这次实验，我觉得自己提升最明显的是三点。

#### 提升一：对 `ch3` 关键状态的理解更具体了

以前我知道 `ch3` 有时钟中断、yield、syscall、多任务切换，但理解偏流程图。

这次实验之后，我能更明确地说出：

- 哪些变量能代表任务当前状态
- 哪些 trap 会修改哪些字段
- 正常、异常、断点、崩溃四类路径的共同点和区别

#### 提升二：对“观测性设计”有了工程意识

以前更倾向于“把日志打出来就行”，现在更能意识到：

- 日志格式要统一
- 场景要可脚本复现
- 运行结果要能自动检查
- 观测信息既要完整，也要便于阅读

这其实已经超出了单个 `ch3` 机制本身，而是更接近工程实践。

#### 提升三：更会用 AI 做工程协作

这次我更明确体会到：

- AI 适合做高频试错和重复劳动
- 人必须负责方向、边界和质量标准

这是一种很实用的能力提升。

### 6.2 可能的不足

如果要说“下降”或者“风险”，也有一点需要诚实说明：

- 因为 AI 能快速补实现，有时容易让人想跳过细读底层细节

所以我认为合理做法是：

- 让 AI 先把系统跑起来
- 但关键 bug 和关键设计点一定要自己理解

如果完全把理解过程交给 AI，那么知识掌握会变浅。

### 6.3 与本校现有教学实验教程的对比

#### 定性对比

本校现有教程更偏“章节讲解 + 学生自行调试”；本实验更偏“观测驱动 + 自动化复现”。

现有教程的优点：

- 理论结构完整
- 章节推进清楚

本实验的优点：

- 更容易看到运行态真实变化
- 更适合定位 bug
- 更适合做报告时给出可验证证据

#### 定量对比

以 `ch3` 为例，现有教程通常只需要：

- `cargo run`
- 看输出是否正确

而本实验进一步提供了：

- 2 套完整场景日志
  - `base`
  - `crash`
- 11 个自动化断言
  - base 6 项
  - crash 5 项
- 1 份独立 crate
- 1 套 GDB 启动脚本
- 1 套 Docker 复现脚本

从可验证性和可复现性上看，本实验明显强于普通章节实验。

## 7. 总结

这次 `T2L21` 的核心价值不在于“功能做得多”，而在于把 `ch3` 的动态运行状态真正组织成了一套可观察、可复现、可对比、可解释的体系。

相对 baseline，我认为本实验最有价值的改进有三项：

1. 任务级状态模型，而不是只看零散标签
2. 状态 delta 输出，显著降低日志分析成本
3. 指令长度感知的断点恢复与脚本化 Docker 回归

如果后续继续做 `T2L23` 或更高章节，我会优先复用本次实验形成的三样东西：

1. 统一状态快照结构
2. delta 观测思路
3. Docker 自动回归框架

这说明本次实验不仅完成了 `T2L21`，也为后续章节的可观察性实验打下了一个更可持续的基础。
