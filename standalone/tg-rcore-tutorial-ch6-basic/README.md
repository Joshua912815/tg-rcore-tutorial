# tg-rcore-tutorial-ch6-basic

`tg-rcore-tutorial-ch6-basic` 是 `tg-rcore-tutorial` Chapter 6 基础实验的独立可复现发布包，只包含本章基础实验所需的内核代码、`easy-fs` 修改、用户态测例、测试脚本和报告材料。

## 基本信息

- Chapter: `6`
- 实验类型: `基础实验`
- crates.io crate: `tg-rcore-tutorial-ch6-basic`
- 版本: `0.6.0-preview.1`
- 仓库: <https://github.com/Joshua912815/tg-rcore-tutorial>
- 对应目录: `standalone/tg-rcore-tutorial-ch6-basic`
- 计划对应 tag: `ch6-basic-crate-v0.6.0-preview.1`
- 文档路径: `docs/filesystem-hardlink-forward-compat-report-ch6.md`

## 本 crate 包含什么

本 crate 对应 `tg-rcore-tutorial-ch6/exercise.md` 的 Chapter 6 基础实验，核心内容包括：

- 实现 `linkat`
- 实现 `unlinkat`
- 实现 `fstat`
- 在 `easy-fs` 中支持硬链接、目录项复用、最后一个链接删除后的 inode 和数据块回收
- 保持 chapter4/chapter5 的前向兼容，确保 `mmap/munmap`、`spawn/wait/waitpid` 等测例继续通过

本 crate 不包含扩展实验、附加题、游戏移植或 Doom 等非 Chapter 6 基础实验内容。

## 与 AI 合作的最终材料

提交材料优先使用已经整理好的中文报告，而不是英文草稿或模拟聊天记录：

- [Chapter6 基础实验记录](docs/filesystem-hardlink-forward-compat-report-ch6.md)

该文档明确覆盖：

- 与 AI 合作的实现过程
- 学习效果评估
- 交互方式
- 问题/bug/解决过程
- 验证过程
- 能力提升
- 与校内教程的对比

## 复现方式

### 方式 1：从 crates.io 获取源码后复现

```bash
cargo clone tg-rcore-tutorial-ch6-basic
cd tg-rcore-tutorial-ch6-basic
cargo run --features exercise
```

进入用户 shell 后执行：

```text
ch6_usertest
```

也可以直接运行脚本：

```bash
./test.sh base
./test.sh exercise
```

### 方式 2：从仓库指定 tag 复现

```bash
git clone https://github.com/Joshua912815/tg-rcore-tutorial.git
cd tg-rcore-tutorial
git checkout ch6-basic-crate-v0.6.0-preview.1
cd standalone/tg-rcore-tutorial-ch6-basic
cargo run --features exercise
```

## 环境说明

按照课程要求，推荐在 `rcore-docker` 中运行。示例：

```bash
docker run -it --name codex-ch6-basic \
  -v "$PWD":/mnt \
  -w /mnt \
  rcore-docker bash
```

在容器内进入 crate 目录后运行：

```bash
cargo check --features exercise
./test.sh base
./test.sh exercise
```

## 测试结论

本 crate 在清理旧 Docker 容器后，使用全新容器完成验证，结果如下：

- `cargo check --features exercise` 通过
- `./test.sh base` 通过，checker 结果为 `Test PASSED: 15/15`
- `./test.sh exercise` 通过，checker 结果为 `Test PASSED: 33/33`
- 手动运行 `cargo run --features exercise` 后执行 `ch6_usertest`，输出 `ch6 Usertests passed!`

## 可复现性说明

为确保老师和助教从 crates.io 下载的真实发布包也能复现，本 crate 做了这些处理：

- 将 Chapter 6 内核依赖的本地 `tg-*` crate 放入 `vendor/`
- 将 `tg-rcore-tutorial-user` 放入 `vendor/`，避免依赖父仓库目录
- 将 `tg-rcore-tutorial-checker` 放入 `vendor/`，测试脚本优先使用本地 checker
- 根 crate 使用 `[patch.crates-io]` 指向包内 `vendor/`，保证解压后的发布包能直接构建
- `build.rs` 优先使用包内 `vendor/tg-rcore-tutorial-user`

因此，复现时不需要假定“本地刚好还有上层 workspace”。
