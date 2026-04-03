# tg-rcore-tutorial-ch5-basic

`tg-rcore-tutorial-ch5-basic` 是本仓库中 Chapter 5 基础实验的独立发布 crate，只对应 `tg-rcore-tutorial-ch5` 的基础实验实现，不混入其它章节的独立发布内容。

## 基本信息

- Chapter：5
- crates.io crate 名：`tg-rcore-tutorial-ch5-basic`
- 仓库地址：[Joshua912815/tg-rcore-tutorial](https://github.com/Joshua912815/tg-rcore-tutorial)
- 对应 tag：`ch5-basic-crate-v0.5.0-preview.2`
- 独立 crate 目录：`standalone/tg-rcore-tutorial-ch5-basic`
- 版本：`0.5.0-preview.2`

## 实验内容

本 crate 对应 `tg-rcore-tutorial-ch5/exercise.md` 中的 Chapter 5 基础实验，覆盖：

- `spawn` 系统调用
- `set_priority` 系统调用
- Chapter 4 遗留的 `mmap/munmap` 在 Chapter 5 进程模型下的迁移
- `fork/exec/wait/waitpid/getpid/sbrk` 的兼容与回归

## 文档

本 crate 内置了实验文档与报告材料：

- [docs/process-management-spawn-scheduling-report-ch5.md](docs/process-management-spawn-scheduling-report-ch5.md)
- [docs/ch5-exercise.md](docs/ch5-exercise.md)

其中报告文档已经明确覆盖：

- 与 AI 合作的实现过程
- 学习效果评估
- 交互方式
- 问题/bug/解决过程
- 验证过程
- 能力提升
- 与校内教程的对比

## 可复现方式

### 方式 1：从 crates.io 获取

```bash
cargo clone tg-rcore-tutorial-ch5-basic
cd tg-rcore-tutorial-ch5-basic
cargo run
```

默认 `cargo run` 会直接编译并运行 Chapter 5 base 测试对应内核。

运行 exercise：

```bash
cargo run --features exercise
```

### 方式 2：从 git 仓库获取

```bash
git clone https://github.com/Joshua912815/tg-rcore-tutorial.git
cd tg-rcore-tutorial/standalone/tg-rcore-tutorial-ch5-basic
cargo run
```

也可以使用：

```bash
make run
make run-exercise
make test-base
make test-exercise
```

## 测试结论

本 crate 发布前已在独立 Docker 环境中验证：

- `cargo check`
- `cargo check --features exercise`
- `cargo package`
- 从 `target/package/*.crate` 解压出的目录中重新执行构建
- `./test.sh base`
- `./test.sh exercise`

当前版本对应的实际结果：

- 工作树：`./test.sh base -> 14/14`
- 工作树：`./test.sh exercise -> 17/17`
- 解压包：`./test.sh base -> 14/14`
- 解压包：`./test.sh exercise -> 17/17`
