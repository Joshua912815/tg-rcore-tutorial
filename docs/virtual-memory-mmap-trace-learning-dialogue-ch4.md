# Chapter4 基础实验记录

## 一、实验任务与完成情况

本次完成的是 `tg-rcore-tutorial-ch4` 的基础实验。根据 `codex.md` 和 `tg-rcore-tutorial-ch4/exercise.md` 的要求，本章需要完成以下内容：

1. 在引入地址空间后，重写 `trace` 系统调用，使其在用户虚拟地址场景下仍能正常工作。
2. 实现匿名 `mmap/munmap`，支持按页建立和解除用户空间映射。
3. 在 docker 的 `rcore-docker` 环境中完成编译、运行和测试验证。
4. 记录与 AI 协作的实现过程、遇到的问题、解决方法，以及学习效果评估。

本次实验已经完成，最终验证结果如下：

- `cargo check` 通过
- `cargo run --features exercise` 通过
- `./test.sh exercise` 通过
- `./test.sh all` 通过

其中 `./test.sh all` 的结果为：

- `ch4 base`：`Test PASSED: 6/6`
- `ch4 exercise`：`Test PASSED: 16/16`

---

## 二、与 AI 合作的实现过程

### 1. 我是如何与 AI 交互的

本次不是让 AI 直接“写完全部代码然后照抄”，而是把 AI 当成一个协助分析和排错的编程助手，主要通过以下方式协作：

1. 先让 AI 阅读 `codex.md`、`exercise.md`、当前章节代码和 checker，用它帮助我梳理任务边界。
2. 再让 AI 对照用户态测例，反推 syscall 需要满足的行为约束，而不是只看函数签名。
3. 在实现过程中，让 AI 先给出“该改哪些位置、为什么改、哪些条件必须检查”的分析，再真正落到代码。
4. 在测试失败后，把失败输出继续交给 AI，要求它基于实际报错定位具体 bug，而不是泛泛讨论。
5. 最后让 AI 在 docker 环境里执行验证，确认不是“看起来对”，而是确实通过测例。

这种交互方式的重点是：AI 负责加速定位、拆解和验证；我负责判断实现是否符合章节语义，避免把不正确的建议直接写进内核。

### 2. 需求分析阶段

AI 首先帮助我确认了 chapter4 的核心变化：引入地址空间后，用户传给内核的地址已经不是可直接解引用的物理/恒等映射地址，而是用户虚拟地址。因此：

- `trace` 不能沿用 chapter3 中直接读写裸指针的做法。
- `clock_gettime` 这类已有 syscall 的写法可以作为参考，因为它已经展示了如何通过 `translate()` 访问用户指针。
- `mmap/munmap` 的实现重点不只是“能映射”，而是要严格符合题目要求中的错误语义。

在这一阶段，AI 帮我做的最有价值的一件事是把实验拆成了两层：

- syscall 语义层：参数检查、权限检查、区间检查、错误返回值
- 地址空间实现层：真正的页映射、解除映射、地址翻译

这样后续写代码时就不容易把检查逻辑和页表操作混在一起。

### 3. 代码定位阶段

AI 协助我重点阅读了这些文件：

- `codex.md`
- `tg-rcore-tutorial-ch4/exercise.md`
- `tg-rcore-tutorial-ch4/src/main.rs`
- `tg-rcore-tutorial-ch4/src/process.rs`
- `tg-rcore-tutorial-kernel-vm/src/space/mod.rs`
- `tg-rcore-tutorial-checker/src/cases/ch4.rs`
- `tg-rcore-tutorial-user/src/bin/ch4_*.rs`

通过这一步，我明确了以下事实：

- `AddressSpace::translate()` 已经能做“页表遍历 + 标志检查”。
- `AddressSpace::map()` 和 `unmap()` 已经提供了建立/解除映射的基础能力。
- chapter4 的主要工作不是重写虚拟内存模块，而是在 syscall 层把语义补完整。
- checker 并不只看是否 panic，还会检查是否出现指定输出，因此验证必须跑完整套脚本。

### 4. 实现阶段

#### 4.1 重写 `trace`

AI 给出的关键建议是：`trace` 必须在 syscall 层显式区分三类请求：

- `trace_request = 0`：读用户地址，要求该地址用户可见且可读
- `trace_request = 1`：写用户地址，要求该地址用户可见且可写
- `trace_request = 2`：查询 syscall 计数，chapter4 仍需要兼容 chapter3 的相关测试

因此本次实现没有照搬 chapter3 的裸指针访问，而是改成：

- 先判断地址是否属于合法用户地址范围
- 再调用 `address_space.translate::<u8>(..., flags)`
- 翻译成功后才进行 `read_volatile` / `write_volatile`
- 翻译失败时返回 `-1`

#### 4.2 恢复 syscall 计数能力

这是本次实现中一个容易遗漏的点。  
因为 chapter4 已经没有 chapter3 的 `task.rs` 结构了，如果只实现读写 trace，而不恢复 syscall 计数，那么 `ch3_trace` 在本章测试里会退化。

AI 帮我确认后，我把 syscall 计数直接补进了 `Process` 结构：

- 新增 `syscall_ids`
- 新增 `syscall_counts`
- 新增 `record_syscall()`
- 新增 `syscall_count()`

然后在 `schedule()` 分发 syscall 前记录当前 syscall。这样 `trace(2, ...)` 查询时，本次 `trace` 自己也已经计入统计，行为与 chapter3 保持一致。

#### 4.3 实现 `mmap/munmap`

AI 在这部分最主要的帮助，是把需要检查的条件一条条列清楚，避免只实现“happy path”。

最终 syscall 层补充了这些判断：

- 地址是否页对齐
- `prot` 是否只使用低 3 位
- `prot & 0x7` 是否非 0
- 映射区间是否溢出
- 映射区间是否处于合法用户地址范围
- `mmap` 的页区间是否与已有映射重叠
- `munmap` 的页区间是否全部已经映射

同时增加了一组小辅助函数，用于统一处理：

- 页大小常量
- 用户空间上界
- 字节区间到 VPN 区间的转换
- `prot -> VmFlags` 转换
- 页区间重叠判断

这样 `mmap/munmap` 的主体代码保持得比较清晰。

---

## 三、遇到的问题、bug 与解决过程

### 问题 1：`trace` 初始占位实现完全不可用

**现象：**

- `trace` 原本只是返回 `-1`
- chapter3 的 trace 行为在 chapter4 中全部失效

**AI 的作用：**

- 帮我快速定位到 `clock_gettime` 是最合适的参考实现
- 明确指出 `translate()` 是本章实现用户指针访问的标准路径

**解决办法：**

- 将 `trace` 改为基于 `translate()` 的安全读写
- 对读和写分别使用不同权限标志
- 对非法地址统一返回 `-1`

### 问题 2：只实现 `trace` 读写还不够，syscall 计数也必须恢复

**现象：**

- 只重写读写逻辑后，`ch3_trace` 相关输出无法完全满足预期

**AI 的作用：**

- 通过阅读用户态测例提醒我：chapter4 的 exercise 仍然依赖 chapter3 的 trace 统计能力

**解决办法：**

- 在 `Process` 中增加稀疏 syscall 统计表
- 在 `schedule()` 中分发 syscall 前先记录计数

### 问题 3：第一次通过大部分测例，但 `trace_read(isize::MAX)` 失败

**现象：**

在第一次跑 `cargo run --features exercise` 时，`ch4_trace.rs` 中的这一句失败：

```rust
assert_eq!(None, trace_read(isize::MAX as usize as *const _));
```

实际返回值是 `Some(0)`。

**原因分析：**

AI 根据失败现象给出的判断是正确的：  
单纯依赖 `VAddr::new()` 不够，因为非规范 Sv39 地址可能在构造过程中被折叠到低位地址，导致错误命中已有映射。

**解决办法：**

- 在 syscall 层新增显式用户地址合法性检查
- `trace` 对单地址要求 `addr < 1 << 38`
- `mmap/munmap` 对整个区间要求落在合法用户地址范围内

这个修复是本次实验里最关键的一次排错，因为它不是语法错误，也不是普通越界，而是“地址语义层面”的 bug。

### 问题 4：docker 中 build 脚本默认尝试 `cargo clone`

**现象：**

- 直接在容器中 `cargo check` 时，build.rs 尝试执行 `cargo clone tg-rcore-tutorial-user`
- 容器中没有安装 `cargo-clone`

**AI 的作用：**

- 复用 chapter3 的环境经验，直接判断可以通过环境变量绕过这一步

**解决办法：**

运行时显式设置：

```bash
TG_USER_DIR=/mnt/tg-rcore-tutorial-user
```

这样 build 脚本直接使用仓库中的本地用户程序目录，不再依赖 `cargo-clone`。

### 问题 5：容器内 Rust 2024 lint 干扰编译输出

**现象：**

依赖库中会出现 `unsafe_op_in_unsafe_fn` 相关警告，影响实验期的验证体验。

**解决办法：**

运行时设置：

```bash
RUSTFLAGS=-Aunsafe_op_in_unsafe_fn
```

这不是本章逻辑问题，但属于实验环境稳定性的一部分。

---

## 四、验证过程

本次实验实际使用过的关键验证命令如下：

```bash
docker run --rm \
  -e TG_USER_DIR=/mnt/tg-rcore-tutorial-user \
  -e RUSTFLAGS=-Aunsafe_op_in_unsafe_fn \
  -e RUSTUP_HOME=/usr/local/rustup \
  -v /tmp/rcore-rustup:/usr/local/rustup \
  -v "$PWD":/mnt \
  -w /mnt/tg-rcore-tutorial-ch4 \
  rcore-docker cargo check
```

```bash
docker run --rm \
  -e TG_USER_DIR=/mnt/tg-rcore-tutorial-user \
  -e RUSTFLAGS=-Aunsafe_op_in_unsafe_fn \
  -e RUSTUP_HOME=/usr/local/rustup \
  -v /tmp/rcore-rustup:/usr/local/rustup \
  -v "$PWD":/mnt \
  -w /mnt/tg-rcore-tutorial-ch4 \
  rcore-docker cargo run --features exercise
```

```bash
docker run --rm \
  -e TG_USER_DIR=/mnt/tg-rcore-tutorial-user \
  -e RUSTFLAGS=-Aunsafe_op_in_unsafe_fn \
  -e RUSTUP_HOME=/usr/local/rustup \
  -v /tmp/rcore-rustup:/usr/local/rustup \
  -v "$PWD":/mnt \
  -w /mnt/tg-rcore-tutorial-ch4 \
  rcore-docker ./test.sh exercise
```

```bash
docker run --rm \
  -e TG_USER_DIR=/mnt/tg-rcore-tutorial-user \
  -e RUSTFLAGS=-Aunsafe_op_in_unsafe_fn \
  -e RUSTUP_HOME=/usr/local/rustup \
  -v /tmp/rcore-rustup:/usr/local/rustup \
  -v "$PWD":/mnt \
  -w /mnt/tg-rcore-tutorial-ch4 \
  rcore-docker ./test.sh all
```

从验证流程看，本次不是“改完直接交”，而是经历了：

1. 静态检查
2. exercise 直接运行
3. exercise 脚本验证
4. all 回归验证

其中最重要的回归点是：修复 chapter4 的 `trace/mmap/munmap` 后，不能把 chapter4 base 的 `sbrk` 等已有功能带坏。

---

## 五、学习效果评估

### 1. 知识与能力提升

按我自己的主观评价，本次实验后的提升主要集中在下面几个方面：

| 项目 | 实验前 | 实验后 | 变化 |
| --- | --- | --- | --- |
| 对“用户虚拟地址 vs 内核可访问指针”的理解 | 2/5 | 4/5 | 明显提升 |
| 对 syscall 语义边界的把握 | 2/5 | 4/5 | 明显提升 |
| 对页权限与 `prot`/PTE 对应关系的理解 | 2/5 | 4/5 | 明显提升 |
| 使用 docker 排查实验环境问题的能力 | 3/5 | 4/5 | 有提升 |
| 独立阅读 tutorial 代码并定位修改点的能力 | 2/5 | 4/5 | 明显提升 |

这里最核心的提升不是“记住了 `mmap` 怎么写”，而是理解了两个更底层的点：

1. 内核处理用户指针时，真正需要检查的是“地址是否合法 + 是否可翻译 + 是否满足权限要求”，不是只看某一个条件。
2. 测试失败时，不能只盯着失败的那一行，而要反推“我到底违反了哪条内核语义”。

### 2. 与本校现有教学实验教程的对比

这一部分我先给出定性分析，再给出可量化的过程对比。

#### 定性对比

和传统只跟教程逐步实现相比，这次 AI 协作带来的主要差异有三点：

1. **问题定位更快。**  
   传统方式下，我通常需要自己在多个文件来回切换，才能确认“该改 syscall 层还是地址空间层”；AI 可以先帮我把路径缩小。

2. **边界条件更容易补全。**  
   例如 `prot` 非法、区间重叠、`munmap` 存在未映射页、非规范用户地址等，这些点只看接口定义很容易漏掉，但 AI 对照测例后能快速列出来。

3. **排错反馈更直接。**  
   当 `trace_read(isize::MAX)` 出现 `Some(0)` 这种反常行为时，AI 能迅速给出“地址规范性检查缺失”的方向，减少无效排查。

但 AI 协作也有明显边界：

- AI 可以帮我定位和归纳，但不能替代我对实验语义的判断。
- 如果我自己不读 `exercise.md` 和用户态测例，只让 AI 直接写代码，仍然很容易得到“看似能跑、实则语义不完整”的实现。

#### 可量化的过程对比

本次实验中可以客观统计的过程数据有：

- 重点阅读文件数：至少 8 个
- 实际修改核心源码文件数：2 个
- 关键验证命令数：4 条
- 主要 bug/阻塞点数：5 类
- 真正影响通过测例的关键 bug：1 个（非规范地址被错误折叠）

与传统教程式完成方式相比，本次 AI 协作更像“先建立问题地图，再实现，再回归验证”，而不是“按步骤照抄实现”。  
就我个人体验而言，这种方式更适合写最终总结报告，因为过程中的判断依据、失败原因和修复逻辑都更容易被完整记录下来。

### 3. 不足与反思

本次学习也暴露出我自己的几个问题：

- 一开始容易把重点放在“把 syscall 写出来”，而忽略“chapter4 相比 chapter3 的本质变化是地址空间隔离”。
- 对 Sv39 非规范地址这一点原本没有足够敏感，直到测例失败后才真正意识到不能只依赖 `VAddr::new()`。
- 在环境层面，如果没有 AI 提醒复用 `TG_USER_DIR` 和 docker 运行经验，我可能会在 build 脚本问题上浪费更多时间。

因此，这次实验对我最大的价值不只是“做完了 ch4”，而是让我开始用“接口语义 + 地址空间 + 验证闭环”的方式来理解教学 OS 实验。

---

## 六、本次记录对最终总报告的可复用内容

后续在总报告中，本章可以直接复用的素材包括：

- chapter4 的任务目标与完成情况
- `trace` 从裸指针访问迁移到地址翻译访问的原因
- `mmap/munmap` 的主要参数检查逻辑
- 非规范地址导致 `trace_read(isize::MAX)` 返回错误结果的 bug 案例
- docker 环境下 `TG_USER_DIR`、`RUSTFLAGS` 的实际使用经验
- 学习效果评估中关于“AI 加快定位，但不能替代语义判断”的结论
