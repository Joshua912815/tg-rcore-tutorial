# T2L8 设计总结报告

## 一、设计思路与目标

我选择 `T2L8`“完善现有实验（工程质量方向的个性化改造）”作为本次任务，核心原因是这个方向最符合我的学习方式。我更适合通过“现象可见、数据可查、错误可定位、结果可复现”的方式理解操作系统，而不是只靠一次性做完功能点后人工观察少量输出。对我来说，先把实验做成一个小型工程系统，再去理解机制细节，学习效率更高。

基于这个判断，我最初给自己的规划不是新增一个很大的内核功能，而是补齐现有教程的四个薄弱环节：

- 把分散的日志统一成一套可读、可筛选的 tracing/metrics 接口；
- 把“能不能跑”提升为“为什么失败、失败后怎么查”的测试体验；
- 给代表性章节补 user workload 和回归脚本，让现象能重复出现；
- 把这些内容整理成可直接交付的文档、脚本和报告。

结合当前仓库结构，我最后把 T8 收敛为三个具体落地点：

1. 在内核侧统一输出 `[EVENT]` 和 `[METRIC]`。
2. 在 user 侧补最小 workload，并提供一键回归脚本。
3. 在文档侧沉淀调试手册、章节模板、workload 说明和最终报告。

最终目标不是“多写一点代码”，而是把原本偏分散的章节实验，提升为一套更像工程化教学系统的实验资产。换句话说，我希望交付的不是零散 patch，而是一套“看得见、跑得通、能复用、能定位问题”的 T8 增强版实验包。

## 二、与 AI 合作的实现过程

### 2.1 协作方式

在这次任务里，我把自己定位为需求裁剪者和最终决策者，把 AI 定位为代码库分析助手、实现助手和调试助手。实际协作方式不是“让 AI 一次性生成答案”，而是多轮推进：

1. 先让 AI 阅读 `TASK2.md` 和代码库，判断 T8 是否适合做成低风险、高完成度的任务。
2. 再让 AI 结合仓库现状提出可落地的实现切口，而不是空泛建议。
3. 然后让 AI 在真实代码上逐步实现、验证、修正。
4. 最后再把实现结果反推回报告，形成“方案 - 代码 - 验证 - 总结”的闭环。

这种协作方式对我最有价值的地方在于：AI 能快速扫出仓库里哪些组件适合作为支点，但哪些内容应该保留、哪些应该收缩、哪些该算作提交版本，需要我自己基于任务边界做判断。

### 2.2 主要实现过程

这次实现大致经历了五个阶段。

第一阶段是梳理仓库结构。AI 先帮助我确认了 T8 最适合从 `tg-rcore-tutorial-console`、`tg-rcore-tutorial-checker`、`ch3/ch4/ch8` 和 `tg-rcore-tutorial-user` 这几块入手，因为它们分别对应统一日志、测试反馈、代表性内核路径和用户态 workload。

第二阶段是先做“内核侧统一观测”。我在 [tg-rcore-tutorial-console/src/lib.rs](/Users/joshua/Desktop/大二下课程资料/tg-rcore-tutorial/tg-rcore-tutorial-console/src/lib.rs) 中增加了 `event!` 和 `metric!`，并把它们接入到：

- [tg-rcore-tutorial-ch3/src/main.rs](/Users/joshua/Desktop/大二下课程资料/tg-rcore-tutorial/tg-rcore-tutorial-ch3/src/main.rs)
- [tg-rcore-tutorial-ch4/src/main.rs](/Users/joshua/Desktop/大二下课程资料/tg-rcore-tutorial/tg-rcore-tutorial-ch4/src/main.rs)
- [tg-rcore-tutorial-ch8/src/main.rs](/Users/joshua/Desktop/大二下课程资料/tg-rcore-tutorial/tg-rcore-tutorial-ch8/src/main.rs)

这样调度、异常、虚存、同步四类核心现象都能走同一套输出格式。

第三阶段是补“user 侧最小 workload”。这里不是盲目新建很多程序，而是围绕能直接展示 T8 改造成果的场景做最小补充：

- [t8_ch3_observe.rs](/Users/joshua/Desktop/大二下课程资料/tg-rcore-tutorial/tg-rcore-tutorial-user/src/bin/t8_ch3_observe.rs)：触发 `yield/get_time/sleep/write`
- [t8_ch4_vm_probe.rs](/Users/joshua/Desktop/大二下课程资料/tg-rcore-tutorial/tg-rcore-tutorial-user/src/bin/t8_ch4_vm_probe.rs)：触发 `mmap/munmap/trace`
- [t8_ch8_usertest.rs](/Users/joshua/Desktop/大二下课程资料/tg-rcore-tutorial/tg-rcore-tutorial-user/src/bin/t8_ch8_usertest.rs)：串联 `sync_sem`、`test_condvar`、`ch8_deadlock_mutex1`

同时，我把这些入口接到了 [cases.toml](/Users/joshua/Desktop/大二下课程资料/tg-rcore-tutorial/tg-rcore-tutorial-user/cases.toml) 和 [initproc.rs](/Users/joshua/Desktop/大二下课程资料/tg-rcore-tutorial/tg-rcore-tutorial-user/src/bin/initproc.rs)，保证它们不是“放在那里但没人执行”的演示文件，而是真正能被章节运行链路带起来的 user workload。

第四阶段是补回归脚本。我新增了：

- [scripts/t8-regression.sh](/Users/joshua/Desktop/大二下课程资料/tg-rcore-tutorial/scripts/t8-regression.sh)
- [scripts/t8-docker-regression.sh](/Users/joshua/Desktop/大二下课程资料/tg-rcore-tutorial/scripts/t8-docker-regression.sh)

这两份脚本把“跑章节 + 抓日志 + 校验 `[EVENT]/[METRIC]` + 校验 user workload 成功标记”串成了同一个流程。对我来说，这一步很关键，因为它把 T8 从“有代码”推进成了“有可复现实验入口”。

第五阶段是整理文档。我把设计和验证结果分别沉淀到：

- [docs/task2-t8-report.md](/Users/joshua/Desktop/大二下课程资料/tg-rcore-tutorial/docs/task2-t8-report.md)
- [docs/t8-debug-playbook.md](/Users/joshua/Desktop/大二下课程资料/tg-rcore-tutorial/docs/t8-debug-playbook.md)
- [docs/t8-chapter-template.md](/Users/joshua/Desktop/大二下课程资料/tg-rcore-tutorial/docs/t8-chapter-template.md)
- [docs/t8-workloads.md](/Users/joshua/Desktop/大二下课程资料/tg-rcore-tutorial/docs/t8-workloads.md)

### 2.3 协作过程中碰到的问题与解决

这部分是我觉得最能体现 AI 协作价值的地方，因为真正消耗时间的并不是“写几个新文件”，而是沿着问题一路排查到底。

问题一是 Docker 里一开始只能做 `cargo check`，完整 user workload 跑不起来。最初看上去像是环境缺工具，但继续排查后发现，章节构建脚本在找 user crate 时走到了 `cargo-clone` 分支。解决方法不是绕过去，而是把回归脚本显式设置 `TG_USER_DIR` 指向本地 [tg-rcore-tutorial-user](/Users/joshua/Desktop/大二下课程资料/tg-rcore-tutorial/tg-rcore-tutorial-user)，从根上消掉这条不稳定分支。

问题二是 user workload 构建被 `unsafe_op_in_unsafe_fn` 卡住。这个问题不是 T8 新引入的 bug，而是仓库在 2024 edition 下的一个现存兼容性问题。最后我在 [tg-rcore-tutorial-syscall/src/user.rs](/Users/joshua/Desktop/大二下课程资料/tg-rcore-tutorial/tg-rcore-tutorial-syscall/src/user.rs) 里把 `asm!("ecall")` 包进显式 `unsafe`，才把 user 程序真正编起来。

问题三是容器里没有 `rg`，导致初版回归脚本自己校验失败。这个问题很典型：逻辑对了，但工程细节没收口。最后我把 [scripts/t8-regression.sh](/Users/joshua/Desktop/大二下课程资料/tg-rcore-tutorial/scripts/t8-regression.sh) 改成“优先 `rg`，没有就回退 `grep`”，让脚本不依赖单一工具。

问题四是 `ch8` 章节一开始跑到了 `user_shell`，不是我想要的 workload。排查后发现是 `initproc` 根据 `CHAPTER` 选择入口，而我最初的脚本没有设置正确章节标识。解决方法是给 `ch8` 单独引入 `CHAPTER=t8-8`，在 [initproc.rs](/Users/joshua/Desktop/大二下课程资料/tg-rcore-tutorial/tg-rcore-tutorial-user/src/bin/initproc.rs) 里映射到 `t8_ch8_usertest`。

问题五是我自己设计的第一个 `t8_ch8_sync_probe` 自动回归版本不稳定。这个问题很有代表性：AI 能很快写出“理论上完整”的 workload，但真放进现有并发测试环境里，接口语义和调度时序并不一定像设想的那样稳定。最后我没有死扛这个单程序方案，而是把 `ch8` 的最终提交版本收缩成 `t8_ch8_usertest`，用仓库里已经稳定的 `sync_sem + test_condvar + ch8_deadlock_mutex1` 组成一条更稳的 T8 回归链。这一步其实很重要，因为它说明我不是机械接受 AI 的第一次方案，而是把 AI 产出当作可迭代草案，再结合真实验证结果收敛到最终版本。

## 三、学习效果评估

### 3.1 对知识和能力的影响

从知识层面看，这次 T8 让我真正把“用户程序 - 系统调用 - 内核处理 - 日志输出 - 脚本校验”这条链路打通了。以前我对这些模块更多是分章理解，现在则更清楚它们在工程上是怎么接起来的。

从能力层面看，我认为自己提升最明显的是三点：

- 代码库级别的定位能力提升了。我不再只盯着某一章的 `main.rs`，而是会主动去追 build 脚本、user crate、`initproc` 和 Docker 环境。
- 调试能力提升了。以前更容易把“跑不起来”归因成单一问题，现在会先判断是构建链、运行链、工具链还是测试链出问题。
- 工程收敛能力提升了。最典型的例子就是 `t8_ch8_sync_probe`，我最后没有执着于一个不稳定的“更完整”方案，而是把它收敛成提交上更稳、更可验证的 `t8_ch8_usertest`。

如果一定要说“下降”或者代价，也有一条是客观存在的：我在 T8 上投入了更多工程化时间，因此对新内核机制的广度扩张速度会稍慢一些。也就是说，这次学习更偏“把已有内容学深、学稳”，而不是继续大幅扩展新的主题。但从这次任务目标来看，我认为这种取舍是合理的。

### 3.2 与本校现有教学实验教程的对比

这里我把“本校现有教学实验教程”理解为当前课程里常见的章节式实验方式，也就是：阅读章节说明、修改内核代码、运行章节、人工观察输出。T8 的价值不是替代这种方式，而是在它之上补出更强的工程支撑。

下面给出一个定量对比。

| 指标 | 改造前 | T8 改造后 |
| --- | --- | --- |
| 统一事件日志接口 | 0 套 | 1 套（`event!` / `metric!`） |
| 代表性接入章节 | 分散日志 | 3 章统一接入（`ch3/ch4/ch8`） |
| T8 专用 user workload / usertest 入口 | 0 | 4 个（`t8_ch3_observe`、`t8_ch4_vm_probe`、`t8_ch8_usertest`、`t8_ch8_sync_probe`） |
| 独立回归脚本 | 0 | 2 个（本地 + Docker） |
| 已完成 Docker 运行验证的章节 | 0 | 3 个（`ch3/ch4/ch8`） |
| T8 配套文档 | 0 | 4 份（报告、调试手册、章节模板、workload 说明） |

定性上，我认为差异主要体现在四点。

第一，可观察性更强。原有教程更偏“读代码猜运行过程”，T8 改造后可以直接看到 `[EVENT][ch3][schedule] ...`、`[EVENT][ch4][vm] ...`、`[EVENT][ch8][sync] ...` 这种统一事件。

第二，可验证性更强。原有流程更多依赖手工运行和人工看输出；现在可以直接用 [scripts/t8-regression.sh](/Users/joshua/Desktop/大二下课程资料/tg-rcore-tutorial/scripts/t8-regression.sh) 或 [scripts/t8-docker-regression.sh](/Users/joshua/Desktop/大二下课程资料/tg-rcore-tutorial/scripts/t8-docker-regression.sh) 做脚本化回归。

第三，可调试性更强。以前很多问题只表现为“没过”或“卡住”；现在至少可以先从构建脚本、`TG_USER_DIR`、`CHAPTER` 选择、QEMU 锁文件、`[EVENT]/[METRIC]` 丢失等角度快速分类。

第四，学习闭环更完整。以前实验更像“做完一个功能”；现在更像“做完一个功能，再把它变成可复现、可比较、可解释的实验资产”。

### 3.3 最终评价

如果只看新增代码量，T8 不是最炫目的任务；但如果看教学价值，我认为它是一个非常适合做成个性化成果的方向。因为它直接改善了后续同学使用教程的体验，也迫使我自己从“会做一章”提升到“能维护一条完整实验链路”。

对我而言，这次任务最大的收获不是某一条系统调用或某一个内核技巧，而是建立了一个更清晰的工程化学习模型：

- 先定义观测点；
- 再补最小 workload；
- 然后做脚本化回归；
- 最后把调试经验和结果写成能复用的文档。

我认为这个模型会继续影响我后面做其他实验的方式，也比单纯多实现一个功能点更符合我当前阶段的学习需要。

## 四、最终交付说明

本次 T2L8 最终提交版本包括：

- 最终报告：[report.md](/Users/joshua/Desktop/大二下课程资料/tg-rcore-tutorial/report.md)
- 扩展说明：[docs/task2-t8-report.md](/Users/joshua/Desktop/大二下课程资料/tg-rcore-tutorial/docs/task2-t8-report.md)
- 调试手册：[docs/t8-debug-playbook.md](/Users/joshua/Desktop/大二下课程资料/tg-rcore-tutorial/docs/t8-debug-playbook.md)
- 章节模板：[docs/t8-chapter-template.md](/Users/joshua/Desktop/大二下课程资料/tg-rcore-tutorial/docs/t8-chapter-template.md)
- workload 说明：[docs/t8-workloads.md](/Users/joshua/Desktop/大二下课程资料/tg-rcore-tutorial/docs/t8-workloads.md)
- 本地回归脚本：[scripts/t8-regression.sh](/Users/joshua/Desktop/大二下课程资料/tg-rcore-tutorial/scripts/t8-regression.sh)
- Docker 回归脚本：[scripts/t8-docker-regression.sh](/Users/joshua/Desktop/大二下课程资料/tg-rcore-tutorial/scripts/t8-docker-regression.sh)

如果按老师建议的提交包结构理解，这次交付已经覆盖了：

- `docs/`：教程和说明文档
- `kernel/`：章节与公共组件实现
- `user/`：workload 与 usertest
- `scripts/`：一键回归入口
- `report.md`：最终提交报告
