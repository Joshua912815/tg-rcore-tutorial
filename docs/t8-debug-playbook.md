# T8 调试手册：tg-rcore-tutorial 常见问题与定位方法

这份手册是 T8“工程质量型改造”的配套资产，目标不是替代源码阅读，而是在实验失败时给出一条更稳定的排查路径。

## 一、统一排查顺序

建议按下面顺序定位：

1. 先确认构建是否成功  
   观察 `cargo build` 或 `cargo run` 是否在进入 QEMU 前就失败。

2. 再确认 QEMU 是否真正启动  
   如果输出为空，优先怀疑：
   - runner 配置错误；
   - 内核镜像未生成；
   - 入口点或链接脚本异常；
   - 很早发生 panic 或非法指令。

3. 再看结构化事件日志  
   T8 新增的 `[EVENT][章节][类别] ...` 行可以快速判断：
   - 是否进入了 syscall 路径；
   - 是否发生了调度切换；
   - 是否触发了缺页或同步阻塞；
   - 是否出现了死锁拒绝。

4. 最后回到关键寄存器与关键数据结构  
   常用观察点：
   - `scause`
   - `stval`
   - `sepc`
   - 当前 task/proc/thread
   - ready queue / wait queue / 资源持有关系

## 二、ch3 常见问题

### 1. 表现：任务似乎没有被抢占

优先检查：

- 是否执行了 `sie::set_stimer()`；
- 是否在每次进入用户态前重新设置 `set_timer(time::read64() + interval)`；
- 是否在处理中断后正确清除了旧定时器状态。

定位方法：

- 看是否出现 `[EVENT][ch3][schedule] ... timer-expired`；
- 如果完全没有，先怀疑时钟中断链路；
- 如果偶尔出现但切换异常，检查主循环中的轮转推进是否正常。

### 2. 表现：某个 syscall 会重复执行，像“卡住了一样”

优先检查：

- 处理 syscall 后是否执行了 `ctx.move_next()`；
- `a0` 返回值是否已经写回；
- 用户态是否在 `ecall` 后重新回到同一条指令。

定位方法：

- 看事件日志是否连续出现同一个 syscall；
- 看 `sepc` 是否始终停留在同一地址；
- 这是 `UserEnvCall` 路径里最常见的错误之一。

### 3. 表现：任务突然被杀死

优先检查：

- `scause` 是异常还是中断；
- 是否是不支持的 syscall；
- 是否是非法指令或访问异常。

定位方法：

- 看 `[EVENT][ch3][trap] ...`；
- 再结合原有错误日志中的 trap 类型。

## 三、ch4 常见问题

### 1. 表现：地址翻译失败或系统调用访问用户指针时报错

优先检查：

- 用户地址是否真的已映射；
- 页表权限是否包含需要的 `R/W/U/V` 位；
- `translate()` 前传入的虚拟地址是否来自当前地址空间。

定位方法：

- 看 `[EVENT][ch4][vm] ...` 是否记录了 `stval` 和 `sepc`；
- 如果是 `LoadPageFault` / `StorePageFault`，优先检查权限和映射范围；
- 如果是普通 `LoadFault` / `StoreFault`，进一步检查地址对齐和非法地址。

### 2. 表现：进程切换后直接崩溃

优先检查：

- 传送门页表项是否正确共享给用户地址空间；
- `satp` 是否切换到正确的根页表；
- 用户栈映射是否有效。

定位方法：

- 若崩溃发生在第一次切换，优先看 `Process::new()` 的地址空间构造；
- 若崩溃发生在 syscall 返回后，优先看 trap 返回路径和 `ctx.move_next()`。

## 四、ch8 常见问题

### 1. 表现：线程阻塞后再也醒不过来

优先检查：

- 阻塞时是否调用了 `make_current_blocked()`；
- 解锁或 `up/signal` 后是否调用了 `re_enque()`；
- wait queue 中记录的线程 ID 是否正确。

定位方法：

- 看 `[EVENT][ch8][sync] ... blocked` 是否出现；
- 再看是否有对应的 `wake_tid=...` 事件；
- 如果有阻塞没有唤醒，优先检查 unlock / up / signal 路径。

### 2. 表现：互斥锁释放后行为异常，像是“丢唤醒”

优先检查：

- `unlock()` 时是否先决定唤醒对象，再改变锁状态；
- 是否错误地把锁直接清空而不是转交给被唤醒线程；
- 条件变量等待是否正确处理了解锁和重获取锁。

定位方法：

- 看 `mutex_unlock` 和 `condvar_wait` 事件日志；
- 对照 wait queue 与当前 owner 状态是否一致。

### 3. 表现：死锁检测总是误报或完全不报

优先检查：

- 是否调用了 `enable_deadlock_detect(true)`；
- 资源注册是否完整；
- 每次获取、阻塞、释放资源时，死锁状态是否同步更新。

定位方法：

- 看 `[EVENT][ch8][sync] ... deadlock`；
- 再核对 `DeadlockState` 中 mutex owner、semaphore total、thread state 三类数据是否一致。

## 五、与 checker 配合使用

T8 对 `tg-rcore-tutorial-checker` 做了一个小改进：当测试失败时，额外提示失败的大类原因。

可优先根据提示判断：

- `output was empty`
  一般优先检查构建、QEMU 启动、早期 panic。

- `missing expected patterns`
  一般表示内核启动了，但核心功能没有走到预期路径。

- `forbidden patterns found`
  一般表示触发了错误输出、panic、异常 trap 或测试明确禁止的行为。

这可以减少“只看到 FAIL，不知道从哪开始看”的时间成本。
