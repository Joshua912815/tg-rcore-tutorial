# joshua912815-tg-rcore-tutorial-ch4-basic

这是 `tg-rcore-tutorial` Chapter 4 基础实验的独立可复现 crate，只包含本章基础实验所需的实现与资源，不包含其他章节扩展版、附加题或无关功能。

## 基本信息

- crate 名称：`joshua912815-tg-rcore-tutorial-ch4-basic`
- git 仓库：[Joshua912815/tg-rcore-tutorial](https://github.com/Joshua912815/tg-rcore-tutorial)
- 对应 tag：`ch4-basic-crate-v0.4.0-preview.1`
- 文档路径：`docs/ch4-report.md`
- 版本：`0.4.0-preview.1`

## 复现方式

### 方式一：从 crates.io 复现

```bash
cargo clone joshua912815-tg-rcore-tutorial-ch4-basic
cd joshua912815-tg-rcore-tutorial-ch4-basic
cargo run
./test.sh all
```

### 方式二：从 git tag 复现

```bash
git clone https://github.com/Joshua912815/tg-rcore-tutorial.git
cd tg-rcore-tutorial
git checkout ch4-basic-crate-v0.4.0-preview.1
cd standalone/joshua912815-tg-rcore-tutorial-ch4-basic
cargo run
./test.sh all
```

## 实验内容

本 crate 对应 Chapter 4 基础实验，完成了以下功能：

- 基于地址翻译与权限检查重写 `trace` 系统调用
- 实现匿名 `mmap/munmap`
- 保持 chapter4 base 与 exercise 两组测试可通过

## 测试结论

在 `rcore-docker` 环境中，本 crate 已完成以下验证：

- `cargo check`
- `cargo package`
- 从 `target/package/*.crate` 解压后的目录执行 `cargo check --features exercise`
- `./test.sh base`
- `./test.sh exercise`
- `./test.sh all`

最终结果：

- `./test.sh base -> Test PASSED: 6/6`
- `./test.sh exercise -> Test PASSED: 16/16`

## 文档

提交材料中的中文实验记录位于：

- [docs/ch4-report.md](docs/ch4-report.md)

该文档包含：

- 与 AI 合作的实现过程
- 学习效果评估
- 交互方式
- 问题 / bug / 解决过程
- 验证过程
- 能力提升
- 与校内教程的对比
