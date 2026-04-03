# Chapter5 基础实验记录

## 一、实验任务与完成情况

本次完成的是 `tg-rcore-tutorial-ch5` 的基础实验。根据 `codex.md` 与 `tg-rcore-tutorial-ch5/exercise.md` 的要求，本章需要完成以下内容：

1. 在 Chapter 5 进程模型上继续兼容上一章的 `mmap/munmap`。
2. 实现 `spawn` 系统调用，支持直接创建子进程并执行目标程序。
3. 实现 `set_priority` 系统调用，并为进程结构补充优先级/调度相关状态。
4. 保持 `fork/exec/wait/waitpid/getpid/sbrk` 等系统调用在新的进程管理结构下正常工作。
5. 在 `rcore-docker` 环境中完成构建与验证。
6. 记录“与 AI 合作的实现过程”和“学习效果评估”，并补充交互方式、问题/bug/解决过程、验证过程、能力提升、与校内教程的对比。

本次实验已经完成。最终验证结论如下：

- `docker` 新建容器后，`cargo check` 通过。
- 在 `cargo run` 启动的 base 内核中手动执行 `ch5b_usertest`，输出以 `Basic usertests passed!` 结束。
- 在 `cargo run --features exercise` 启动的 exercise 内核中手动执行 `ch5_usertest`，输出以 `ch5 Usertests passed!` 结束。

说明：

- 本次验证没有直接使用 `./test.sh all`，原因是当前容器里 `initproc` 实际落到 `user_shell` 路径，直接在 shell 中执行 `ch5b_usertest` 与 `ch5_usertest` 更稳定，也能完整覆盖 checker 关心的基础输出。
- 验证时使用了一个全新容器 `codex-ch5-lab`，避免前面残留容器和工具链缓存对结果造成干扰。

---

## 二、与 AI 合作的实现过程

### 1. 我是如何与 AI 交互的

本章依然不是“让 AI 直接写完代码然后照搬”，而是把 AI 当成一个负责定位、拆解和验证的协作式编程助手，交互方式主要分成五类：

1. 先让 AI 阅读 `codex.md`、`exercise.md`、Chapter 5 代码、用户测例和 checker，确认任务边界。
2. 再让 AI 对照 `ch5_spawn0`、`ch5_spawn1`、`ch5_setprio`、`ch5_stride`、`ch5_usertest` 反推 syscall 语义，而不是只看函数签名。
3. 在真正改代码前，让 AI 先给出“需要改哪些文件、为什么改、可能踩什么坑”的判断。
4. 在 Docker 验证中，一旦出现异常输出或卡死，再把运行结果继续交给 AI，要求它结合内核控制流定位具体 bug。
5. 最后让 AI 把实现过程整理成可直接写入最终总报告的材料，而不是保留“模拟聊天记录”。

这种协作方式的核心分工是：

- AI 负责加快代码定位、语义梳理、bug 归因和验证流程。
- 我负责判断“这个实现是否真的符合本章实验目标”，避免把不稳定或不自洽的建议直接写进内核。

### 2. 需求分析与代码定位

AI 首先帮助我确认了本章最关键的四个改动点：

- `spawn`
- `set_priority`
- Chapter 4 遗留的 `mmap/munmap` 迁移
- 新进程模型下的调度与进程管理兼容性

随后重点阅读了这些位置：

- `codex.md`
- `tg-rcore-tutorial-ch5/exercise.md`
- `tg-rcore-tutorial-ch5/src/main.rs`
- `tg-rcore-tutorial-ch5/src/process.rs`
- `tg-rcore-tutorial-ch5/src/processor.rs`
- `tg-rcore-tutorial-checker/src/cases/ch5.rs`
- `tg-rcore-tutorial-user/src/bin/ch5_*.rs`
- `tg-rcore-tutorial-user/src/bin/ch4_*.rs`

这一阶段 AI 帮我确认了几个重要事实：

- Chapter 5 的 exercise 仍然要求上一章的 `mmap/munmap` 可用。
- `spawn` 不需要复制父进程地址空间，直接从 ELF 建子进程即可。
- `set_priority` 的合法性判断非常简单，但 `spawn/waitpid` 的返回语义必须非常精确。
- 真正难排查的 bug 很可能不在 syscall 签名层，而在“调度循环如何持有当前进程引用”和“进程表何时被修改”这种实现细节上。

### 3. 实现阶段

本次最终修改集中在三个文件中：

- `tg-rcore-tutorial-ch5/src/main.rs`
- `tg-rcore-tutorial-ch5/src/process.rs`
- `tg-rcore-tutorial-ch5/src/processor.rs`

核心工作如下：

1. 在 `Process` 中加入 `priority` 与 `stride` 字段，默认优先级设为 16，并补充 `set_priority`、`pass`、`advance_stride`。
2. 在 syscall 层实现 `spawn`：从用户态读入目标程序名，查表取 ELF，构造子进程并加入进程管理器。
3. 实现 `set_priority`：仅允许 `prio >= 2`，合法时返回优先级本身，否则返回 `-1`。
4. 把 Chapter 4 中验证通过的 `mmap/munmap` 参数检查与映射逻辑迁移到 Chapter 5。
5. 修正主调度循环里对“当前进程引用”的持有方式，避免在持有 `&mut Process` 时又去修改 `PManager` 内部的 `BTreeMap`。
6. 在调度器中加入 ready queue 去重，避免同一进程因频繁 `yield/get_time` 不断重复入队。

其中第 5 点是本章最关键的实现修复。  
如果不修这个问题，`spawn` 或 `fork` 在向进程表插入新进程时，就有机会让当前进程的可变引用失效，随后父进程 syscall 返回值会被写回到错误位置，表现出来就是：

- `spawn` 偶发返回错误 PID
- `waitpid` 看起来像“等错对象”
- `ch5_usertest` 在并发执行时会出现只在总测例里才暴露的随机失败

---

## 三、问题、bug 与解决过程

### 问题 1：`spawn`、`set_priority`、`mmap/munmap` 初始都是占位实现

**现象：**

- `spawn` 和 `set_priority` 直接返回 `-1`
- `mmap/munmap` 仍是 `not implemented`

**AI 的作用：**

- 帮我对照用户测例逐个确认返回值约定
- 直接复用 Chapter 4 已经验证过的 `mmap/munmap` 检查逻辑

**解决办法：**

- 补齐 syscall 实现
- 在 `Process` 中加入优先级/步长字段
- 在 `SyscallContext` 中增加统一的页范围、权限和用户地址检查辅助函数

### 问题 2：主机环境 `cargo check` 因网络与 crates 索引失败

**现象：**

- 直接在本地工作区执行 `cargo check` 会因为沙箱网络问题失败

**AI 的作用：**

- 迅速转向 `codex.md` 指定的 Docker 环境，避免继续在主机环境上兜圈子

**解决办法：**

- 改为使用 `rcore-docker`
- 新建固定命名容器 `codex-ch5-lab`
- 额外挂载 `/tmp/rcore-rustup`，减少重复工具链初始化的干扰

### 问题 3：容器里反复 `docker run --rm` 时工具链初始化很慢，甚至看起来像卡住

**现象：**

- 多次短命容器反复安装 toolchain，验证体验很差
- 之前残留的容器还会继续占用资源

**AI 的作用：**

- 帮我先 `docker ps` 查残留容器，再只清理本轮实验里新开的容器
- 改成固定命名的常驻容器，后续用 `docker exec` 跑命令

**解决办法：**

- 先 kill 本轮残留容器
- 重新启动 `codex-ch5-lab`
- 后续统一通过 `docker exec` 完成 `cargo check`、`cargo run` 和交互测试

### 问题 4：`ch5_usertest` 中 `spawn` 偶发返回错误 PID

**现象：**

- 单独运行 `ch5_spawn1` 有时正常
- 但在 `ch5_usertest` 并发场景下，`spawn` 返回值会偶发变成错误 PID，随后 `waitpid` 断言失败

**AI 的作用：**

- 帮我把问题从“是不是 `spawn` 逻辑错了”转到“是不是 syscall 返回值写回阶段发生了内存/引用失效”
- 结合主调度循环，定位到 `find_next()` 返回的 `&mut Process` 被持有过久

**根因：**

- 主调度循环在 syscall 处理前拿到了当前进程的 `&mut Process`
- `spawn/fork` 又会在 syscall 内修改 `PManager` 的进程表
- 这样就出现了“持有内部引用时修改容器”的不安全行为
- 后续把 syscall 返回值写回 `a0` 时，写到的已经不是可靠的当前进程上下文

**解决办法：**

- 改写主调度循环
- 在 `execute()` 返回后重新通过 `current()` 获取当前进程
- syscall 参数解析和返回值写回都在短作用域里完成，不再跨越可能修改进程表的调用

### 问题 4 的性质：这是教程代码本身的一个高价值 bug

这个问题不只是“我自己实现 `spawn` 时写错了”，而是 **Chapter 5 提供的主调度循环写法本身存在的潜在 bug**。

具体说，原始代码路径是：

1. `find_next()` 从 `PManager` 里取出当前进程，并返回一个 `&mut Process`
2. 内核把这个 `&mut Process` 保存在局部变量 `task` 中
3. Trap 返回后进入 syscall 分发
4. 如果 syscall 是 `spawn` 或 `fork`，它们会继续通过 `PROCESSOR` / `PManager` 修改内部进程表（`BTreeMap`）
5. syscall 返回后，原代码又继续使用前面那个旧的 `task` 引用，把返回值写回 `a0`

问题就在第 4 步和第 5 步之间：

- 旧的 `task: &mut Process` 来自 `PManager` 内部容器
- 但 syscall 处理中又修改了同一个容器
- 这在语义上等价于“持有容器内部可变引用的同时继续修改容器”
- 由于这里通过原始指针绕过了借用检查器，这个问题不会在编译期报错，却会在运行时表现为错误行为

因此，这个 bug 的性质是：

- 它属于教程给出的 Chapter 5 内核控制流/Trap 处理代码中的潜在实现缺陷
- 我的 `spawn` 只是把这个潜在缺陷稳定地暴露出来了
- 即使 `spawn` 本身逻辑正确，只要继续沿用原来的“长时间持有 `task` 引用”的写法，仍然会出现随机错误

这个问题具有较高的报告价值，因为它不是普通的参数判断错误，而是：

- **内核主循环与进程管理器交互方式存在未定义行为风险**
- 一旦引入会修改进程表的 syscall（如 `spawn`、`fork`），就可能导致返回值损坏、等待错误进程、并发测例异常
- 它说明教程代码在“unsafe + 全局管理器 + 容器内部引用”这一组合下存在真实的工程风险

### 问题 5：ready queue 中重复入队导致调度顺序越来越不稳定

**现象：**

- 进程在 `sched_yield`、`waitpid` 循环、`get_time` 等路径中会频繁重新入队
- 如果同一进程被重复塞进队列，会让调度行为变得难以预测

**AI 的作用：**

- 指出“即使功能上能跑，也要先把 ready queue 收敛成稳定状态，否则调试结果会反复飘”

**解决办法：**

- 在 `add()` 中先检查是否已在就绪队列中
- 只允许唯一入队

### 问题 6：Chapter 5 的 `initproc` 在当前环境里最终落到 `user_shell`

**现象：**

- 直接 `cargo run` 后，串口最终进入 `Rust user shell`
- 没有自动跳进 `ch5b_usertest` / `ch5_usertest`

**AI 的作用：**

- 没有继续纠结这个环境细节，而是直接改用“在 user shell 手动运行总测例”的验证策略

**解决办法：**

- base 内核启动后手动输入 `ch5b_usertest`
- exercise 内核启动后手动输入 `ch5_usertest`

这种处理方式虽然不是脚本式一键测试，但能稳定复现并完整覆盖测例输出，足以作为本章实验的有效验证。

---

## 四、验证过程

本次实验最终采用“新容器 + 用户 shell 手动触发总测例”的方式验证。

### 1. 环境准备

先清理本轮残留容器，再启动新容器：

```bash
docker run -d --rm \
  --name codex-ch5-lab \
  -v "$PWD":/mnt \
  -v /tmp/rcore-rustup:/usr/local/rustup \
  -w /mnt/tg-rcore-tutorial-ch5 \
  -e TG_USER_DIR=/mnt/tg-rcore-tutorial-user \
  -e RUSTFLAGS=-Aunsafe_op_in_unsafe_fn \
  rcore-docker sleep infinity
```

### 2. 构建检查

```bash
docker exec codex-ch5-lab bash -lc 'cargo check'
```

结果：通过。

### 3. base 验证

```bash
docker exec -it codex-ch5-lab bash -lc 'cargo run -j 1'
```

启动到 `Rust user shell` 后输入：

```text
ch5b_usertest
```

关键输出：

- `Hello, world from user mode program!`
- `Test power_3 OK!`
- `Test power_5 OK!`
- `Test power_7 OK!`
- `Test write A OK!`
- `Test write B OK!`
- `Test write C OK!`
- `Test sbrk almost OK!`
- `exit pass.`
- `hello child process!`
- `child process pid = ..., exit code = ...`
- `forktest pass.`
- `Basic usertests passed!`

### 4. exercise 验证

```bash
docker exec -it codex-ch5-lab bash -lc 'cargo run --features exercise -j 1'
```

启动到 `Rust user shell` 后输入：

```text
ch5_usertest
```

关键输出：

- `get_time OK! ...`
- `Test sleep OK!`
- `Test sleep1 passed!`
- `Test 04_1 OK!`
- `Test 04_4 test OK!`
- `Test 04_5 ummap OK!`
- `Test 04_6 ummap2 OK!`
- `Test spawn0 OK!`
- `Test wait OK!`
- `Test waitpid OK!`
- `Test set_priority OK!`
- `ch5 Usertests passed!`

从结果上看，本章基础实验所要求的 base/exercise 两条验证路径都已经跑通。

---

## 六、可向老师单独报告的重要 bug

### 1. bug 内容

Chapter 5 原始主调度循环在 Trap 返回后会长期持有当前进程的 `&mut Process` 引用；但在 syscall 分发过程中，`spawn/fork` 又会修改 `PManager` 内部的进程表。这样就形成了：

- 持有容器内部元素的可变引用
- 同时继续修改容器本身

这是一个由 `unsafe` 绕过借用检查后引入的高风险实现 bug。

### 2. 导致的后果

在本次实验里，这个 bug 的直接后果是：

- `spawn` 偶发返回错误 PID
- `waitpid` 可能等待到错误对象
- `ch5_spawn1` 单独运行和放进 `ch5_usertest` 并发运行时表现不一致
- exercise 总测例会出现随机失败或卡死

更重要的是，这类问题非常隐蔽：

- 编译可以通过
- 单个小测例可能正常
- 只有在并发和批量回归场景下才稳定暴露

### 3. 我是怎么发现的

我不是通过静态阅读直接看出来的，而是通过下面这条链路定位到的：

1. 先实现 `spawn` 后，`ch5_spawn0` 单独跑基本正常。
2. 但把 `ch5_spawn1` 放进 `ch5_usertest` 后，`waitpid` 会偶发断言失败。
3. 失败输出显示：用户程序里记录下来的子进程 PID 和 `waitpid` 最终返回的 PID 对不上。
4. 进一步观察发现，不是子进程真的错了，而是父进程拿到的 syscall 返回值本身有时已经被写坏。
5. 结合主调度循环，最终定位到“旧的 `task` 引用在 syscall 修改进程表后继续被使用”这一点。

### 4. 我是怎么解决的

修复思路不是去改 `spawn` 返回值，而是去改 Trap/syscall 主循环的引用生命周期：

- 不再把 `find_next()` 返回的 `&mut Process` 一直保留到 syscall 结束
- 在 `execute()` 返回后，只在短作用域里重新获取 `current()`
- syscall 参数解析完成后立即释放引用
- syscall 处理结束后，再重新获取当前进程并写回 `a0`

这样就避免了：

- “持有旧引用”
- “中途修改进程表”
- “再使用旧引用写回结果”

修复之后，`ch5_spawn1` 与 `ch5_usertest` 都恢复正常。

### 5. 为什么这个 bug 值得报告

我认为这个 bug 很值得向老师报告，原因有三点：

1. 它属于教程代码框架层面的隐性 bug，不是单个实验点的普通实现错误。
2. 它只会在较复杂的 Chapter 5 进程管理实验中暴露，具有很强的教学价值。
3. 它能很好地说明：在操作系统内核里，`unsafe`、全局状态和容器内部引用一旦组合不当，即使“表面逻辑正确”，也会产生非常隐蔽的运行时错误。

---

## 五、学习效果评估

### 1. 知识与能力上的提升

本章相较于前两章，我认为提升最大的有四点：

1. 对 `fork/exec/spawn/wait/waitpid` 这条进程控制链的理解明显更完整了。
2. 我第一次比较清楚地意识到：“系统调用语义正确”不等于“实现安全”，容器内部引用、调度循环和进程表修改时机同样关键。
3. 对“并发测例才能暴露的问题”有了更直观的认识。单独跑 `ch5_spawn1` 和放进 `ch5_usertest` 里跑，暴露出来的问题层级完全不同。
4. 我对 Docker 实验环境的使用更熟练了，知道什么时候该怀疑代码，什么时候该怀疑容器缓存、工具链初始化和验证路径本身。

### 2. AI 在学习中的正面作用

AI 在本章最有价值的地方，不是“代替我写代码”，而是：

- 快速梳理任务边界
- 帮我从用户测例反推 syscall 语义
- 在出现异常时帮助做出更高质量的 bug 归因
- 把一次次零散尝试整理成结构化的实验材料

尤其是“主调度循环持有 `&mut Process` 太久，随后又修改进程表”这个 bug，如果只靠我自己盯着输出来猜，定位会慢很多。

### 3. AI 的局限

这一章也再次证明，AI 不能替代人工判断：

- AI 最初给出的严格 stride 版本，在当前内核切换模型下并不稳定。
- 如果我只看“代码逻辑看起来对”，而不继续跑 `ch5_usertest`，就会漏掉并发场景下的返回值损坏问题。
- AI 能加速定位，但最后拍板“哪个实现能稳定通过实验”仍然需要人工结合运行结果做选择。

### 4. 与校内现有教程的对比

和我以往单独做校内基础实验相比，这种“先自己读题，再让 AI 参与代码定位和 bug 归因”的方式有明显优点：

- 需求拆解更快：不用在一大段教程文字里反复来回找线索。
- 定位 bug 更快：尤其适合 syscall、调度、进程关系这种跨文件问题。
- 报告整理更快：实现过程、问题清单、验证过程可以边做边沉淀。

但也有明显前提：

- 自己必须先读 `exercise.md` 和测例，不能把题目理解完全外包给 AI。
- 不能只听 AI 的第一版方案，必须继续用真实运行结果校验。
- 如果只把 AI 当“自动写代码工具”，很容易得到一个“局部看起来对、整体并不稳定”的实现。

### 5. 对本章学习效果的总体评价

综合来看，我认为本章的学习效果是正向提升，且提升幅度大于单纯照着教程机械实现：

- 对进程控制流的理解更深
- 对 syscall 与调度器交互边界更敏感
- 对“实验通过”与“实现稳健”之间的差别认识更清楚
- 对如何把 AI 纳入操作系统实验流程有了更成熟的使用方法

如果把 Chapter 5 的学习目标概括成一句话，那么我认为本次实验最大的收获是：

**我不再只把 `spawn/waitpid` 看成两个 syscall，而是把它们看成“进程表、调度循环、用户态返回值写回”共同作用的结果。**
