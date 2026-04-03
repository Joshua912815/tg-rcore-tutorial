# Chapter3 Lab1 实验报告

## 一、实验环境

- 本地环境：macOS
- 容器：`rcore-docker`
- SBI 信息：
  - `RustSBI version 0.3.0-alpha.2, adapting to RISC-V SBI v1.0.0`
  - `Implementation: RustSBI-QEMU Version 0.2.0-alpha.2`

## 二、功能说明

本次实验为 ch3 内核补充了 `sys_trace(410)` 系统调用，实现了三类能力：读取当前任务某地址的 1 字节数据、向当前任务某地址写入 1 字节数据、查询当前任务指定系统调用的累计调用次数。同时在任务控制块中维护每个任务独立的系统调用统计信息，并在系统调用总入口统一记账，保证查询次数时“本次 `sys_trace` 调用也计入统计”。

## 三、实现说明

1. 在系统调用分发入口加入 `SYSCALL_TRACE` 处理，并定义统计数组支持的最大 syscall 编号范围。
2. 在 `TaskControlBlock` 中新增 `syscall_times` 字段，按任务维持独立统计，避免多个应用共享计数。
3. 在 `syscall()` 总入口先调用 `record_current_syscall(syscall_id)`，这样 `trace_request = 2` 查询时能自然把本次调用算进去。
4. 在 `sys_trace` 中按题意实现三种请求：
   - `trace_request = 0`：把 `id` 当作 `*const u8` 读取一个字节并返回；
   - `trace_request = 1`：把 `id` 当作 `*mut u8` 写入 `data as u8`；
   - `trace_request = 2`：返回当前任务编号为 `id` 的系统调用累计次数；
   - 其他请求返回 `-1`。
5. 为了让 `sleep` 相关测例在 QEMU 下更稳定，`sys_get_time` 采用毫秒粒度返回时间，避免启动早期读取到 `0ms`。

## 四、测例结果

在容器中执行 `cd /mnt/os && make run BASE=0`，`ch3_sleep`、`ch3_sleep1`、`ch3_trace` 均通过。关键输出如下：

```text
get_time OK! 4
current time_msec = 5
time_msec = 105 after sleeping 100 ticks, delta = 100ms!
Test sleep1 passed!
string from task trace test

Test trace OK!
Test sleep OK!
```

## 五、问答题

### 1. bad 测例的出错行为

我在同一实验环境下运行了 `ch2b_bad_address`、`ch2b_bad_instructions`、`ch2b_bad_register`。现象如下：

- `ch2b_bad_address`：内核报告页错误，输出 `PageFault in application, bad addr = 0x0`，随后杀死当前应用并继续调度下一个应用。
- `ch2b_bad_instructions`：内核报告 `IllegalInstruction in application`，说明用户态执行了不允许的特权指令或非法指令，随后终止该应用。
- `ch2b_bad_register`：同样触发 `IllegalInstruction in application`，说明用户态访问 S 态相关寄存器时会被硬件判定为非法指令。

这说明程序一旦真正进入 U 态后，既不能随意执行 S 态特权指令，也不能越权访问仅 S 态可用的寄存器；一旦违反，硬件会产生异常并转入内核处理。

### 2. `trap.S` 中 `__alltraps` 与 `__restore` 的作用

`__alltraps` 负责在 U 态陷入 S 态时保存用户上下文、切换到内核栈，并把 `TrapContext` 交给 Rust 写的 `trap_handler`；`__restore` 负责从内核栈上的 `TrapContext` 恢复寄存器和 CSR，最后通过 `sret` 返回用户态。

#### (1) L40：刚进入 `__restore` 时，`sp` 代表什么值？两种使用情景是什么？

刚进入 `__restore` 时，`sp` 指向当前任务内核栈上的 `TrapContext` 起始地址。

两种使用情景：

- 第一次启动某个应用时，通过 `TaskContext::goto_restore(init_app_cx(i))` 让任务切换后直接把控制流落到 `__restore`，把预先构造好的初始 `TrapContext` 恢复到用户态。
- 用户程序执行过程中发生 trap 后，`__alltraps -> trap_handler` 处理完毕，控制流继续落到 `__restore`，恢复刚才保存的上下文并返回用户态。

#### (2) L43-L48 特殊处理了哪些寄存器？它们对进入用户态有什么意义？

- `t0 <- sstatus`：保存并恢复用户返回时所需的状态位，尤其是 `SPP`、`SPIE` 等，它决定 `sret` 之后回到哪个特权级、是否开启中断。
- `t1 <- sepc`：保存并恢复用户态下一条将执行的 PC，`sret` 返回后会跳到这里。
- `t2 <- x2(sp)`，随后写入 `sscratch`：这里保存的是用户栈指针。后面会通过 `csrrw sp, sscratch, sp` 把它交换回 `sp`，使用户程序重新使用自己的用户栈。

#### (3) L50-L56：为何跳过了 `x2` 和 `x4`？

- 跳过 `x2(sp)` 是因为用户栈指针已经单独保存在 `TrapContext` 的 `x[2]` 中，并且会借助 `sscratch` 在最后专门恢复，不能按普通寄存器那样直接装载。
- 跳过 `x4(tp)` 是因为在本章用户程序没有使用线程本地存储语义，框架里明确注明应用不使用它，因此没有单独保存恢复。

#### (4) L60 之后，`sp` 和 `sscratch` 中的值分别有什么意义？

执行 `csrrw sp, sscratch, sp` 后：

- `sp` 变成用户栈指针，供即将返回的用户程序使用；
- `sscratch` 变成当前任务内核栈顶附近的位置（准确说是释放掉 `TrapContext` 之后的内核栈指针），供下一次 trap 进入时快速换栈。

#### (5) `__restore` 中发生状态切换的是哪一条指令？为何执行后会进入用户态？

发生特权级切换的是 `sret`。因为在它之前已经把 `sstatus` 恢复为用户返回所需状态，其中 `SPP = User`，所以 `sret` 会把处理器从 S 态切回 U 态，并跳转到 `sepc` 指定的位置继续执行。

#### (6) L13 之后，`sp` 和 `sscratch` 中的值分别有什么意义？

L13 的 `csrrw sp, sscratch, sp` 执行后：

- `sp` 变成内核栈指针，因为 trap 前内核已把它保存在 `sscratch` 中；
- `sscratch` 变成陷入前的用户栈指针。

这一步完成了从用户栈到内核栈的切换，随后内核才能安全地在自己的栈上保存上下文。

#### (7) 从 U 态进入 S 态是哪一条指令发生的？

由用户程序执行的 `ecall` 指令触发。`ecall` 会让处理器产生 “Environment call from U-mode” 异常，硬件据此切换到 S 态并跳到 `stvec` 指向的 `__alltraps`。

## 六、Honor Code

1. 在完成本次实验的过程（含此前学习的过程）中，我曾分别与 以下各位 就（与本次实验相关的）以下方面做过交流，还在代码中对应的位置以注释形式记录了具体的交流对象及内容：
我没有与他人就本实验进行过交流

2. 此外，我也参考了以下资料 ，还在代码中对应的位置以注释形式记录了具体的参考来源及内容：


3. 我独立完成了本次实验除以上方面之外的所有工作，包括代码与文档。 我清楚地知道，从以上方面获得的信息在一定程度上降低了实验难度，可能会影响起评分。

4. 我从未使用过他人的代码，不管是原封不动地复制，还是经过了某些等价转换。 我未曾也不会向他人（含此后各届同学）复制或公开我的实验代码，我有义务妥善保管好它们。 我提交至本实验的评测系统的代码，均无意于破坏或妨碍任何计算机系统的正常运转。 我清楚地知道，以上情况均为本课程纪律所禁止，若违反，对应的实验成绩将按“-100”分计。



## 七、实验体会


