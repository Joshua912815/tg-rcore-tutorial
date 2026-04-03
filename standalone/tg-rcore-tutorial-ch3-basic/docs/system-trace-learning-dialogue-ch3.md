# ch3 system trace 学习对话记录

## 说明

- 工作目录：`tg-rcore-tutorial-ch3`
- 角色设定：我是学过 CSAPP、但第一次系统性写教学 OS 内核实验的初学者
- 对话对象：另一个 AI 助手
- 学习目标：对照 README，理解 `sys_trace` 练习要求，并完成实现与验证

---

## 对话记录

**我：**  
我先看 README。这里说 ch3 已经有多任务和时钟中断了，现在要加一个新的系统调用 `sys_trace`，ID 是 410。它有三种功能：读用户内存、写用户内存、查询系统调用计数。我理解得对吗？

**AI：**  
对。你可以把它看成一个很小的“调试接口”：

- `trace_request = 0`：把 `id` 当成用户地址，读 1 个字节
- `trace_request = 1`：把 `id` 当成用户地址，写 1 个字节
- `trace_request = 2`：把 `id` 当成 syscall 编号，返回当前任务对这个 syscall 的调用次数

而且 README 特别强调了一点：**查询计数时，这次 `trace` 调用本身也要被统计进去。**

**我：**  
这个要求挺像 CSAPP 里那种“先想清楚状态放在哪里”的题。README 提示说系统调用计数可以考虑在 `TaskControlBlock::handle_syscall()` 里统计，所以核心修改面应该是 `src/task.rs` 和 `src/main.rs`。

**AI：**  
是的。你可以先把题目拆成两件事：

1. `main.rs` 里把 `Trace for SyscallContext` 真正实现出来  
2. `task.rs` 里给“当前任务”建立 syscall 次数统计

**我：**  
我又看了测试程序 `tg-rcore-tutorial-user/src/bin/ch3_trace.rs`。它会先调用一些 `get_time()`、`sleep()`，再通过 `count_syscall(SYS_...)` 去查计数，还会打印：

```text
string from task trace test
Test trace OK!
```

所以如果最终输出里有这两句，基本就说明 trace 练习通过了。

**AI：**  
没错。更具体地说，这个测试覆盖了三类需求：

- 统计 `CLOCK_GETTIME` / `TRACE` / `WRITE` / `SCHED_YIELD` / `EXIT`
- 检查 `trace_read` 能不能读一个字节
- 检查 `trace_write` 能不能写一个字节

**我：**  
先看 `main.rs` 里的占位实现。现在 `impl Trace for SyscallContext` 只是打印一句 `trace: not implemented` 然后返回 `-1`。这部分直接按题意填充就行：

- 读：`(id as *const u8).read_volatile()`
- 写：`(id as *mut u8).write_volatile(data as u8)`
- 查计数：返回当前任务的 syscall 次数

我觉得这部分不难，难点是“当前任务”的计数怎么拿。

**AI：**  
对。真正的设计点在 `task.rs`。

**我：**  
我先看 `TaskControlBlock::handle_syscall()`。这里会：

1. 从 `a7` 取 syscall id  
2. 从 `a0-a5` 取参数  
3. 调 `tg_syscall::handle(...)` 分发  
4. 把返回值写回 `a0`

所以如果要统计 syscall 次数，**最自然的时机就是在分发前记一次**。这样查询 `trace(2, ...)` 时，这次 trace 本身已经被计数了。

**AI：**  
这个判断非常对。这里正好对应 README 里的提示。

**我：**  
我第一反应是：直接给 `TaskControlBlock` 新增一个大数组，比如 `syscall_count[512]`，按 syscall id 下标计数。这样查询的时候是 O(1)。

**AI：**  
这是最直观的方案，但你要小心一个教学内核里常见的问题：**对象放在哪里。**

**我：**  
我明白了，`tcbs` 是在 `rust_main` 里定义的局部数组，也就是说这些 TCB 本身是在**内核栈**上。  
而每个 TCB 里本来就已经有 8 KiB 用户栈，如果我再给每个任务加一个 4 KiB 左右的 syscall 统计数组，总大小会膨胀得很厉害。

**AI：**  
对，这就是这个实验里一个非常值得学的点：  
“数据结构设计”不仅是接口问题，也是**内存布局**问题。

**我：**  
我实际试了一次，结果内核启动后只打了 banner，后面用户程序不跑了。这个现象很像是局部大数组把内核栈挤爆了。  
所以这个直觉方案虽然逻辑简单，但放在这个程序结构里不合适。

**AI：**  
这个排查过程本身就很有价值。你已经不是只在“功能层面”看问题，而是在“实现载体”层面看问题了。

**我：**  
那我换个思路：  
不要把统计信息塞进 `TaskControlBlock`，而是放到 `task.rs` 的静态区，也就是 BSS / data 里。  
比如：

- 一个 `CURRENT_TASK` 记录当前运行的是哪个任务
- 一个静态的“每个任务的 syscall 记录表”

这样 TCB 尺寸就不会继续涨。

**AI：**  
这就是更适合本题的方案。

**我：**  
不过我又不想真的开一个 `32 x 512` 的大矩阵，虽然它在静态区比在栈上安全，但还是显得很笨重。  
看测试程序，真正关心的 syscall 号并不多，所以我决定做一个**稀疏记录表**：

- 每个任务记录最多 16 个“出现过的 syscall id”
- 以及各自的调用次数

如果某个 syscall 第一次出现，就塞进空槽；后面再出现就累加。

**AI：**  
这个设计很适合教学题：

- 空间小
- 实现简单
- 足以覆盖当前测试

**我：**  
于是 `task.rs` 里我最后做了这几件事：

1. 用 `AtomicUsize` 保存当前任务编号  
2. 用静态数组保存每个任务的 syscall id 和 count  
3. 在加载任务时初始化该任务的 trace 状态  
4. 在调度循环进入某个任务前，设置当前任务编号  
5. 在 `handle_syscall()` 里先记账，再调用 `tg_syscall::handle(...)`

这就把“当前执行的任务”和“这个任务的 syscall 历史”连起来了。

**AI：**  
而且这个设计还有一个好处：`trace()` 查询计数时，不需要借用整个 `TaskControlBlock`，只要根据当前任务编号查静态表即可，结构上更干净。

**我：**  
`main.rs` 里的 `trace()` 实现也就顺理成章了：

- `0`：读字节并返回
- `1`：写字节并返回 `0`
- `2`：从当前任务的 syscall 统计表里查次数
- 其他请求返回 `-1`

这里 README 明说了本章不要求地址安全检查，只要通过测试即可，所以我就直接按裸指针读写了。

**AI：**  
这点很重要。ch3 还没引入地址空间，题目本身就允许你先接受这种“不安全但可工作的”实现；ch4 才会把这个接口升级成带权限检查的版本。

**我：**  
还有一个环境问题我也顺手解决了。  
在 `rcore-docker` 里跑的时候，`tg-rcore-tutorial-user` 的构建会因为 Rust 2024 的 `unsafe_op_in_unsafe_fn` lint 报错，所以我额外加了：

```bash
RUSTFLAGS=-Aunsafe_op_in_unsafe_fn
```

然后为了避免 `cargo clone` 依赖，我还显式指定了：

```bash
TG_USER_DIR=/workspace/tg-rcore-tutorial-user
```

**AI：**  
这属于“让实验环境稳定下来”的必要工程动作。思路是对的：  
不是去改题目外的仓库，而是通过环境变量把构建路径和 lint 行为先调顺。

**我：**  
我先运行：

```bash
docker run --rm \
  -e TG_USER_DIR=/workspace/tg-rcore-tutorial-user \
  -e RUSTFLAGS=-Aunsafe_op_in_unsafe_fn \
  -v "$PWD":/workspace \
  -w /workspace/tg-rcore-tutorial-ch3 \
  rcore-docker cargo run --features exercise
```

关键输出是：

```text
[ INFO] load app0 to 0x80400000
[ INFO] load app1 to 0x80600000
[ INFO] load app2 to 0x80800000
string from task trace test
Test trace OK!
```

这说明 trace 测例已经跑通了。

**AI：**  
最好再用官方测试脚本跑一次，形成“功能正确 + 练习检查器通过”的双重确认。

**我：**  
于是我又运行：

```bash
docker run --rm \
  -e TG_USER_DIR=/workspace/tg-rcore-tutorial-user \
  -e RUSTFLAGS=-Aunsafe_op_in_unsafe_fn \
  -v "$PWD":/workspace \
  -w /workspace/tg-rcore-tutorial-ch3 \
  rcore-docker bash -lc './test.sh exercise'
```

最后得到：

```text
Test PASSED: 7/7
✓ ch3 练习测试通过
```

**AI：**  
那就说明你这次不只是“看起来能跑”，而是正式通过了本章练习。

**我：**  
我最后总结一下这题我真正学到的东西：

1. `trace` 本身不复杂，真正的核心是“把状态放在哪里”  
2. 在内核里，局部数组、静态区、用户栈、内核栈这些放置位置差别非常大  
3. `handle_syscall()` 是统计 syscall 的最佳切入点，因为它正好处在统一分发入口  
4. README 给出的提示其实非常关键，读题时不能只看接口定义，还要看“实现提示”和测试程序

**AI：**  
这个总结很到位。你已经不是在机械补代码，而是在按操作系统实现的视角思考问题了。

---

## 本次实现摘要

- 在 `src/main.rs` 中实现了 `Trace for SyscallContext`
- 在 `src/task.rs` 中加入了“当前任务 syscall 统计”的静态侧表
- 在系统调用分发前记录 syscall 次数，保证 `trace(2, ...)` 会把本次 `trace` 计入
- 避免把大计数表直接塞进 `TaskControlBlock`，从而规避内核栈空间压力

## 验证记录

### 1. 直接运行练习

```bash
docker run --rm \
  -e TG_USER_DIR=/workspace/tg-rcore-tutorial-user \
  -e RUSTFLAGS=-Aunsafe_op_in_unsafe_fn \
  -v "$PWD":/workspace \
  -w /workspace/tg-rcore-tutorial-ch3 \
  rcore-docker cargo run --features exercise
```

观察到关键输出：

```text
string from task trace test
Test trace OK!
```

### 2. 官方脚本验证

```bash
docker run --rm \
  -e TG_USER_DIR=/workspace/tg-rcore-tutorial-user \
  -e RUSTFLAGS=-Aunsafe_op_in_unsafe_fn \
  -v "$PWD":/workspace \
  -w /workspace/tg-rcore-tutorial-ch3 \
  rcore-docker bash -lc './test.sh exercise'
```

最终输出：

```text
Test PASSED: 7/7
✓ ch3 练习测试通过
```
