# T2L8: Engineering-Quality tg-rcore-tutorial Bundle

这是我为 `T2L8`“完善现有实验（工程质量方向的个性化改造）”整理的最终提交版 crate。  
它不是单一章节源码快照，而是一个**可复现实验包**，把本次改造涉及的代码、文档、user workload、回归脚本和报告一起打包，便于自己和他人继续学习、复现和扩展。

## 学生提交信息

- crate 名称：`joshua912815-tg-rcore-tutorial-t2l8`
- crate 版本：`0.8.0-preview.1`
- 仓库地址：`https://github.com/Joshua912815/tg-rcore-tutorial`
- 仓库页面：`https://github.com/Joshua912815/tg-rcore-tutorial/tree/t2l8-crate`
- 建议 tag：`joshua912815-tg-rcore-tutorial-t2l8-v0.8.0-preview.1`
- keywords：`ai` `ai4ose` `kernel` `learning` `os`

## 这个 crate 做了什么

本次 T2L8 没有把重点放在新增单一内核功能，而是把 `tg-rcore-tutorial` 改造成一套更容易学习、调试和复现的实验系统。具体包括：

- 统一 tracing / metrics 框架：
  - 在 `tg-rcore-tutorial-console` 中新增 `event!` / `metric!`
  - 在 `ch3/ch4/ch8` 中接入统一的 `[EVENT]` / `[METRIC]`
- 更强的 user workload 与回归入口：
  - `t8_ch3_observe`
  - `t8_ch4_vm_probe`
  - `t8_ch8_usertest`
- 更好的验证方式：
  - 本地脚本：`scripts/t8-regression.sh`
  - Docker 脚本：`scripts/t8-docker-regression.sh`
- 更完整的学习文档：
  - 最终报告 `report.md`
  - 设计说明 `docs/task2-t8-report.md`
  - 调试手册 `docs/t8-debug-playbook.md`
  - 章节模板 `docs/t8-chapter-template.md`
  - workload 说明 `docs/t8-workloads.md`

## 复现方式

### 方式一：从 crates.io 获取

```bash
cargo clone joshua912815-tg-rcore-tutorial-t2l8
cd joshua912815-tg-rcore-tutorial-t2l8
make run
```

如果你已经有 `rcore-docker` 镜像，也可以直接在容器里复现：

```bash
cargo clone joshua912815-tg-rcore-tutorial-t2l8
cd joshua912815-tg-rcore-tutorial-t2l8
make run-docker
```

### 方式二：从 Git 仓库获取

```bash
git clone https://github.com/Joshua912815/tg-rcore-tutorial.git
cd tg-rcore-tutorial
git checkout t2l8-crate
make run
```

或：

```bash
git clone https://github.com/Joshua912815/tg-rcore-tutorial.git
cd tg-rcore-tutorial
git checkout t2l8-crate
make run-docker
```

## 推荐阅读顺序

1. 先读 [report.md](./report.md)
2. 再读 [docs/task2-t8-report.md](./docs/task2-t8-report.md)
3. 再看 [docs/t8-workloads.md](./docs/t8-workloads.md)
4. 最后结合代码看：
   - `tg-rcore-tutorial-console`
   - `tg-rcore-tutorial-ch3`
   - `tg-rcore-tutorial-ch4`
   - `tg-rcore-tutorial-ch8`
   - `tg-rcore-tutorial-user`

## 验证结果

本 crate 在 `rcore-docker` 环境中已经完成过一轮实际验证，能看到如下关键成功标记：

- `ch3`：`T8 ch3 observe OK!`
- `ch4`：`T8 ch4 vm observe OK!`
- `ch8`：`T8 ch8 Usertests passed!`

并且日志中能直接看到：

- `[EVENT][ch3][schedule] ...`
- `[EVENT][ch4][vm] ...`
- `[EVENT][ch8][sync] condvar_wait ...`
- `[EVENT][ch8][sync] deadlock-detect enabled=true`
- `[METRIC][ch8][sync_blocked_events] ...`

## 项目结构

```text
.
├── bundle/                         # crates.io 打包的完整子 crate 压缩包
├── docs/
│   ├── task2-t8-report.md         # 设计说明
│   ├── t8-debug-playbook.md       # 调试手册
│   ├── t8-chapter-template.md     # 章节模板
│   └── t8-workloads.md            # workload 说明
├── scripts/
│   ├── extract_submodules.sh      # 解包 bundle
│   ├── t8-regression.sh           # 本地回归
│   ├── t8-docker-check.sh         # Docker 编译检查
│   └── t8-docker-regression.sh    # Docker 回归
├── report.md                      # 最终提交报告
├── Makefile                       # 统一入口：make run / make run-docker
└── src/lib.rs                     # bundle crate 说明
```

解包后可得到完整工作区，其中最关键的改造集中在：

- `tg-rcore-tutorial-console`
- `tg-rcore-tutorial-checker`
- `tg-rcore-tutorial-syscall`
- `tg-rcore-tutorial-user`
- `tg-rcore-tutorial-ch3`
- `tg-rcore-tutorial-ch4`
- `tg-rcore-tutorial-ch8`

## 为什么我认为这个 crate 有学习价值

这个 crate 的价值不只是“多写了一些代码”，而是把原本偏分散的章节实验做成了一套更完整的学习资产：

- 对自己来说，它强化了“观测点 -> workload -> 回归 -> 文档”这条工程化学习链路；
- 对别人来说，它提供了可以直接运行的入口、可以直接看的日志、可以直接复现的脚本，以及可以直接参考的调试手册和总结报告。

如果你希望基于 `tg-rcore-tutorial` 做更高完成度的个性化实验，这个 crate 可以作为一个工程质量增强方向的起点。
