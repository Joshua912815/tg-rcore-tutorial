# Chapter 3 基础实验独立发布 Crate

`joshua912815-tg-rcore-tutorial-ch3-basic` 是 `tg-rcore-tutorial` Chapter 3 基础实验的独立可复现发布包，目标是让助教和老师可以直接从 crates.io 包或当前 Git 仓库的对应 tag 复现实验结果。

## 发布信息

- crate 名称：`joshua912815-tg-rcore-tutorial-ch3-basic`
- 章节范围：Chapter 3 基础实验
- 当前仓库：<https://github.com/Joshua912815/tg-rcore-tutorial>
- 对应 tag：`ch3-basic-crate-v0.3.0-preview.1`
- crate 版本：`0.3.0-preview.1`
- 文档目录：`docs/`

## 本包包含的内容

- `src/`：Chapter 3 内核源码，仅包含本章基础实验所需代码，保留可选 `exercise` feature 用于 trace 相关验证。
- `vendor/tg-rcore-tutorial-user/`：复现 Chapter 3 所需的最小用户态程序子集，不依赖父 workspace。
- `exercise.md`：本章题面。
- `docs/system-trace-learning-dialogue-ch3.md`：已整理好的中文 AI 协作对话记录。
- `docs/ch3-learning-evaluation.md`：学习效果评估、问题归因、验证过程与对比说明。
- `docs/ch3-lab1-reference.md`：已有实验报告参考材料。

## 复现方式

### 方式一：从 crates.io 下载

```bash
cargo clone joshua912815-tg-rcore-tutorial-ch3-basic
cd joshua912815-tg-rcore-tutorial-ch3-basic
cargo run
./test.sh base
./test.sh exercise
```

### 方式二：从 Git 仓库 tag 复现

```bash
git clone https://github.com/Joshua912815/tg-rcore-tutorial.git
cd tg-rcore-tutorial
git checkout ch3-basic-crate-v0.3.0-preview.1
cd standalone/tg-rcore-tutorial-ch3-basic
cargo run
./test.sh base
./test.sh exercise
```

## 环境要求

- 推荐在 `rcore-docker` 容器中运行。
- 默认使用 `.cargo/config.toml` 中的 RISC-V 目标和 QEMU runner。
- 用户态程序已经内置在 `vendor/tg-rcore-tutorial-user/`，不需要额外依赖父仓库。

推荐启动方式：

```bash
docker run -it --rm --name ch3-basic-crate \
  -v "$PWD":/mnt \
  -w /mnt \
  rcore-docker bash
```

容器内执行：

```bash
cargo run
./test.sh base
./test.sh exercise
```

## 测试结论

本次发布前按 standalone crate 形态完成了以下验证：

- `cargo package`
- 在干净 Docker 环境中执行 `./test.sh base`，结果 `Test PASSED: 4/4`
- 在干净 Docker 环境中执行 `./test.sh exercise`，结果 `Test PASSED: 7/7`
- 后续还会对 `target/package/*.crate` 解压目录做独立复现，确保发布包本身可用

最终结果以实际发布版本对应的 tag 为准；本 README 会与发布版本保持一致。

## 文档导航

- [题面说明](exercise.md)
- [AI 协作对话记录](docs/system-trace-learning-dialogue-ch3.md)
- [学习效果评估](docs/ch3-learning-evaluation.md)
- [实验报告参考](docs/ch3-lab1-reference.md)
