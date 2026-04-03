# `ch2-moving-tangram` 与 AI 协作交互记录整理

## 1. 记录说明

本文档用于整理我在完成 `ch2-moving-tangram` 实验过程中与 AI 的交互过程，目的是为后续总报告提供一份可以直接引用的过程材料。

与正式实验报告相比，这份记录更强调：

1. 我在什么阶段向 AI 提出了什么问题
2. AI 帮我做了哪些分析
3. AI 给出的建议是否被采纳
4. 最终哪些问题被解决，哪些地方需要我自己再判断

这次我依然没有把 AI 当成“直接给最终答案的黑盒工具”，而是把它作为一名辅助开发者来协作。整个过程更接近“我先给任务边界，AI 帮我分析仓库、收敛实现路径、定位 bug、推进验证，我再继续做结果判断和方向修正”。

## 2. 交互总览

本次与 AI 的协作大致分为 7 个阶段：

1. 明确实验要求与任务边界
2. 在剩余候选游戏里判断哪个最容易实现
3. 建立独立分支并阅读 `ch1`/`ch2` 代码
4. 实现 `ch2-moving-tangram`
5. 修复构建和运行过程中的问题
6. 定位并修复黑条与花屏 bug
7. 编写独立实验报告与过程文档

## 3. 分阶段交互记录

### 3.1 第一阶段：明确任务边界

#### 我的输入重点

我先把原始实验要求重新发给 AI，并明确提出：

- 需要具体参考根目录 `README.md` 中“游戏应用和支持游戏的内核简要描述如下”这一部分
- 当前已经完成了 `ch1-tangram` 和 `ch8-doom`
- 现在需要在剩余游戏里再完成一个额外实验
- 希望 AI 先帮我判断哪个最容易做，并说明理由

#### AI 的主要作用

AI 没有直接凭印象回答，而是先去读取 `README.md` 中对应的游戏列表，并把候选项逐个梳理出来：

- `ch2-moving-tangram`
- `ch3-snake`
- `ch4-tetris`
- `ch5-pingpong`
- `ch6-breakout`
- `ch7-pacman`

随后 AI 结合各章节能力边界和实现复杂度，判断最容易的是：

- `ch2-moving-tangram`

#### AI 给出的主要理由

AI 的理由主要有三点：

1. `ch1-tangram` 已经完成，因此 framebuffer、图形数据和绘制逻辑可以大量复用。
2. `ch2-moving-tangram` 只是在 `ch1` 的静态显示基础上加入“多程序批处理逐块显示”的特征，技术跨度最小。
3. 它不需要实时输入、碰撞检测、计分、文件系统、信号或线程同步，因此比后面的 `snake`、`tetris`、`pacman` 更轻量。

#### 这一阶段的价值

这一阶段帮助我把“第三个游戏做什么”从一个主观选择题，变成了一个基于仓库现状和技术增量的工程判断题。

### 3.2 第二阶段：建立独立分支

#### 我的要求

在决定做 `ch2-moving-tangram` 之后，我要求 AI：

- 新开一个 `ch2-moving-tangram` 分支来完成本实验

#### 实际发生的情况

AI 首先检查了当前仓库状态，确认：

1. 当前分支是 `test`
2. 仓库中已有 `ch1-tangram`、`ch8-doom` 等相关分支
3. 工作区里存在 `.gitignore` 的未提交修改

随后 AI 尝试创建新分支，但第一次因为 `.git` 写权限受限失败。之后它没有停下来等我手工处理，而是通过授权流程完成了：

- `git switch -c ch2-moving-tangram`

最终成功切换到：

- `ch2-moving-tangram`

#### 我的体会

这一步虽然不复杂，但体现了一个很实际的协作点：

- AI 不只是写代码，也能把分支切换、状态检查、权限判断这些工程前置工作一起处理掉。

### 3.3 第三阶段：阅读 `ch1` 和 `ch2` 代码，收敛实现路径

#### 我的目标

在真正开始写代码前，我希望先弄清：

1. `ch1-tangram` 具体是怎么实现的；
2. 当前 `ch2` 的批处理主循环结构是什么；
3. 应该让用户程序自己绘图，还是让内核统一绘图。

#### AI 的分析过程

AI 读取了以下内容：

- `ch1-tangram` 分支中的 `tg-rcore-tutorial-ch1/src/main.rs`
- `tg-rcore-tutorial-ch1/src/framebuffer.rs`
- `tg-rcore-tutorial-ch1/src/tangram.rs`
- 当前分支中的 `tg-rcore-tutorial-ch2/src/main.rs`
- `tg-rcore-tutorial-ch2/build.rs`
- `tg-rcore-tutorial-user/src/lib.rs`
- `tg-rcore-tutorial-user/cases.toml`

#### AI 帮我明确的几个事实

1. `ch1` 已经有完整的最小 GPU + framebuffer 渲染链路，可以直接迁移图形部分。
2. `ch2` 的核心价值仍然是批处理系统，因此最好保留“多个程序顺序执行”的主循环。
3. 如果让每个用户程序自己直接管理 GPU，会把实现复杂度抬高；更稳妥的是让内核统一持有 GPU 和 framebuffer，用户程序只负责发出“绘制第几块”的请求。

#### AI 最终给出的实现路径

AI 最终建议采用：

1. `ch1` 的图形模块移植到 `ch2`
2. `ch2` 主循环保留不变
3. 新增一个最小自定义 syscall
4. 用户态拆成 13 个小程序，每个程序请求绘制一块

我采纳了这个方案。

#### 这一阶段的价值

这一步帮助我避免了两类常见弯路：

1. 不会把 `ch2-moving-tangram` 做成“只是另一份静态 `ch1-tangram`”
2. 也不会把用户态设计得过于复杂

### 3.4 第四阶段：实现 `ch2-moving-tangram`

这一阶段是本次协作中代码量最大的一段。

#### AI 协助完成的主要修改

AI 帮我完成了以下几类工作：

1. 修改 `tg-rcore-tutorial-ch2/Cargo.toml`
   - 新增 `virtio-drivers = "0.1.0"`

2. 重写 `tg-rcore-tutorial-ch2/src/main.rs`
   - 移植 VirtIO-GPU 初始化
   - 增加 DMA Arena 和 BumpAllocator
   - 维护图形状态
   - 增加自定义 syscall `0x1000`
   - 让批处理循环在每个用户程序退出后继续加载下一个程序

3. 新增 `tg-rcore-tutorial-ch2/src/framebuffer.rs`
   - 封装 framebuffer 像素绘制能力

4. 新增 `tg-rcore-tutorial-ch2/src/tangram.rs`
   - 定义 13 个 Tangram 图形块
   - 支持清屏和按编号逐块绘制

5. 修改 `tg-rcore-tutorial-user/src/lib.rs`
   - 新增 `draw_tangram_piece(piece_id)` 封装
   - 新增 `run_tangram_piece_app(piece_id, label)` 辅助函数

6. 修改 `tg-rcore-tutorial-user/cases.toml`
   - 把 `ch2` 的案例列表替换成 13 个 `ch2_tangram_*` 程序

7. 新增 `tg-rcore-tutorial-user/src/bin/ch2_tangram_*.rs`
   - 每个程序只负责请求绘制一块

#### AI 的实现特点

AI 并不是一开始就写成一大坨，而是先给出了一个明确的最小方案：

1. 图形部分尽量复用 `ch1`
2. 用户态尽量小
3. 内核接口尽量窄
4. 先打通“能顺序画出来”再谈优化

这种推进方式让我比较容易检查每一层改动是不是都符合 `ch2` 的教学主题。

### 3.5 第五阶段：构建链路与工具链问题

这一阶段是我觉得 AI 协作很实用的一段，因为它没有停留在“代码写完”层面，而是继续把实验真正跑通。

#### 问题一：`cargo check` 与交叉编译环境不完整

最开始构建时，AI 发现：

1. 本机没有 `riscv64gc-unknown-none-elf` target
2. 本机没有 `llvm-tools-preview`
3. `ch2/build.rs` 默认还会找 `cargo clone` 和 `rust-objcopy`

AI 的处理不是简单报错，而是分成两步：

1. 先用 `TG_SKIP_USER_APPS=1 cargo check` 验证内核本体有没有类型错误；
2. 再继续补全工具链，解决“完整运行”的问题。

#### 问题二：`tg-rcore-tutorial-syscall` 在 Rust 2024 下编译失败

当 `ch2` 需要真正编译用户程序时，`tg-rcore-tutorial-syscall/src/user.rs` 中的 `asm!` 因为 Rust 2024 的规则变化而报错。

AI 很快判断出：

- 问题不在 `moving-tangram` 本身
- 而在用户态 syscall 封装里 `unsafe fn` 内部没有显式 `unsafe { asm!(...) }`

随后 AI 直接补上了这些显式 `unsafe {}` 包裹，使整个用户程序构建恢复正常。

#### 问题三：`build.rs` 只认 `rust-objcopy`

最开始完整构建时，`build.rs` 在打包用户程序阶段失败，因为本机没有安装 `rust-objcopy`。

AI 采取了两个动作：

1. 修改 `.cargo/config.toml`，让 `ch2` 优先使用仓库里的本地 `tg-rcore-tutorial-user`
2. 修改 `build.rs`，让它依次尝试：
   - `RUST_OBJCOPY`
   - `rust-objcopy`
   - Rust toolchain 自带的 `llvm-objcopy`

这样在安装 `llvm-tools-preview` 后，整个构建链路就闭环了。

#### 问题四：需要真正安装工具链组件

在确认仓库内逻辑已经收敛后，AI 直接推进执行：

- `rustup target add riscv64gc-unknown-none-elf`
- `rustup component add llvm-tools-preview`

而不是把这一步留给我手工操作。

这一点让我觉得它不只是“代码助手”，更像是一个能把实验完整推进下去的协作开发者。

### 3.6 第六阶段：QEMU 首次运行与 GPU 挂载问题

#### 现象

第一次真正 `cargo run` 时，QEMU 启动了，但内核很快 panic：

```text
failed to create VirtIO MMIO transport: ZeroDeviceId
```

#### AI 的分析

AI 很快判断出：

- 不是 GPU 驱动 API 用错了
- 也不是 framebuffer 绘制逻辑错了
- 而是 QEMU 里 GPU 设备被挂到了错误的 MMIO 总线上

最初 runner 中写的是：

- `virtio-mmio-bus.1`

但当前 `ch2` 只有一个 VirtIO 设备，因此应该挂到：

- `virtio-mmio-bus.0`

#### 解决

AI 立即修改了 `tg-rcore-tutorial-ch2/.cargo/config.toml`，将设备挂载位置改正。

修改后再次运行，GPU 初始化成功，日志出现：

```text
[ INFO] moving tangram display ready at 1280x800 with 13 pieces
```

#### 这一阶段的价值

这一步让我认识到，图形实验里很多问题根本不在渲染算法，而在运行环境配置。

### 3.7 第七阶段：黑条与颜色错乱 bug 的定位与修复

这是本次 `ch2-moving-tangram` 中最关键的一次协作。

#### 我的输入重点

在确认程序已经跑起来后，我发现图形窗口中间出现了：

1. 一条大黑带
2. 黑带上方有彩色噪点
3. 图像上下像被撕裂

我把截图直接发给 AI，并要求它判断是什么问题。

#### AI 的第一判断

AI 没有先去怀疑：

- 多边形顶点数据错了
- 缩放比例错了
- 颜色通道顺序错了

它的第一判断是：

> 这更像是 framebuffer 或 DMA 对应的内存被其他数据覆盖了。

我觉得这个判断非常关键，因为它一下子把问题从“图形算法层”提升到了“系统内存布局层”。

#### AI 的分析过程

随后 AI 继续回溯地址布局，发现：

1. `ch2` 中新增了较大的静态图形相关内存：
   - 16 MiB DMA Arena
   - 1 MiB Kernel Heap
2. 但 `ch2` 的用户程序原本仍然装载到：
   - `0x8040_0000`

AI 由此判断：

- 用户程序的装载地址与新增的 DMA / heap / framebuffer 相关内存发生了重叠或破坏
- 当批处理系统每次把用户程序装进同一个低地址区域时，就会把本该保留的图形内存写坏

#### AI 给出的修复方法

AI 直接修改了：

- `tg-rcore-tutorial-user/cases.toml`

把 `ch2` 的 `base` 从：

- `0x8040_0000`

改成：

- `0x8200_0000`

#### 修复结果

修复后再次运行，日志显示：

```text
[ INFO] load app0 to 0x82000000
```

同时图形窗口恢复正常，黑条和颜色错乱消失。

#### 我的体会

我觉得这是本次协作最有价值的一次，因为它体现了：

1. 图像异常往往只是外在表现
2. 真正的根因可能是地址空间布局
3. AI 在这种“从现象反推系统根因”的问题上比单纯写代码更有价值

### 3.8 第八阶段：报告撰写

在 `ch2-moving-tangram` 真正跑通之后，我要求 AI：

1. 按 `ch1-tangram` 的报告格式写一份 `ch2-moving-tangram` 独立报告
2. 还要额外补一份和 `ch1-tangram-ai-log.md` 对应的 AI 交互记录整理版

AI 首先根据 `ch1-tangram` 分支中的：

- `docs/ch1-tangram-report.md`
- `docs/ch1-tangram-ai-log.md`

来对齐格式，而不是自己重新发明一套结构。

随后它生成了：

- `docs/ch2-moving-tangram-report.md`
- `docs/ch2-moving-tangram-ai-log.md`

并且把黑条 bug 的原因和修复过程都整理进了文档。

#### 这一阶段的价值

这一步让我少了很多重复整理聊天记录和回忆技术细节的工作量，尤其是：

1. 构建链路问题
2. GPU 总线挂载问题
3. 黑条和花屏的真正根因

这些如果不及时整理，后面写总报告时很容易丢失细节。

## 4. 我认为本次 AI 协作中最有效的地方

### 4.1 AI 帮我把“选题问题”转成了“最小增量实现问题”

它没有只说“`ch2-moving-tangram` 看起来更简单”，而是明确指出：

1. 复用 `ch1-tangram`
2. 新增点只在批处理与逐块显示
3. 不需要引入更高章节机制

这让我在选题时就避免走了高复杂度路线。

### 4.2 AI 在系统层 bug 定位上很有效

黑条 bug 就是最典型的例子。它没有被图形表象误导，而是快速怀疑内存覆盖，并最终把根因定位到了用户程序装载基址与图形静态区冲突。

### 4.3 AI 能把“写代码”推进到“真正跑起来”

本次实验里，AI 不只是补代码，还推动了：

1. Rust target 安装
2. `llvm-tools-preview` 安装
3. `objcopy` 回退链路修复
4. QEMU runner 修复
5. 真机运行验证

这让它更像是一个完整的协作开发者，而不是单纯的代码生成器。

## 5. 本次 AI 协作中需要警惕的地方

### 5.1 AI 提供的是高概率正确路径，但仍需要我自己做最终判断

例如黑条问题，虽然 AI 很快定位到“内存覆盖”，但是否真的修好，仍然需要我自己重新运行并观察图形窗口。

### 5.2 文档角色仍然需要我明确指定

和 `ch1` 类似，如果我不明确说明“这是独立报告”还是“总报告子章节”，AI 容易按它自己觉得合理的方式组织内容。

### 5.3 AI 会优先选择“最小可行方案”

这在工程上是优点，但也意味着：

1. 当前 syscall 是专用的最小接口；
2. 当前动态效果是逐块顺序显示，而不是更复杂动画；
3. 当前地址布局是手工抬高基址，而不是完整的动态内存规划。

这些方案都够用，但是否还要进一步抽象和泛化，仍然需要我自己判断。

## 6. 我对这次 AI 协作的总体评价

如果用一句话总结这次合作，我会这样写：

> AI 在本次 `ch2-moving-tangram` 实验中，显著提高了我选题判断、代码复用、构建链路收敛、运行环境修复和 bug 根因定位的效率，尤其在黑条/花屏问题上，帮助我从图像现象快速回溯到系统地址布局冲突这一真正根因；但最终的实验要求核对、图形效果确认和提交材料把关，仍然必须由我自己完成。

如果要给这次 AI 协作做一个主观评分，我会给：

- 选题分析辅助：`9/10`
- 代码实现辅助：`8/10`
- 构建与运行推进：`9/10`
- bug 定位辅助：`9/10`
- 对最终提交的完全替代能力：`3/10`

最后这一项分数仍然偏低，不是因为 AI 不够有用，而是因为操作系统实验的最终正确性、可视化效果和提交结构，仍然必须由我自己承担最后责任。

## 7. 可直接用于总报告的简短总结

下面这段文字可以直接摘录到后续总报告中：

> 在 `ch2-moving-tangram` 实验中，我首先让 AI 阅读根目录 `README.md` 中关于 `ch2-moving-tangram` 的描述，并结合当前已经完成的 `ch1-tangram` 与 `ch8-doom`，帮助我判断剩余候选游戏中最容易实现的是 `ch2-moving-tangram`，因为它可以最大程度复用 `ch1` 的图形渲染链路，同时新增点主要集中在 `ch2` 的批处理逐块执行。之后我与 AI 采用“先读仓库、再收敛方案、最后逐步实现和验证”的方式推进开发：先复用 `ch1` 的 VirtIO-GPU 与 framebuffer 模块，再在 `ch2` 中增加最小图形 syscall 和 13 个逐块绘制用户程序，并最终打通了从构建到运行的完整链路。实验过程中，AI 不仅帮助我处理了 Rust target、`llvm-tools-preview`、`objcopy` 回退和 QEMU GPU 设备挂载等工程问题，还在图形出现黑条和颜色错乱时，帮助我从表面图像异常快速定位到用户程序装载地址与 DMA/heap 图形内存区冲突这一根因，并通过将装载基址从 `0x8040_0000` 提高到 `0x8200_0000` 修复了问题。整体上，AI 显著提高了我的实现和排错效率，但实验要求理解、图形效果验收和最终文档把关仍然由我自己负责。
