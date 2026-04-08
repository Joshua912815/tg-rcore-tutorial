# joshua912815-rcore-tutorial-t2l9-smp

`joshua912815-rcore-tutorial-t2l9-smp` 是一个面向 `TASK2.md` 中 `T2L9` 的独立实验 crate。  
它把“`ch1~ch2` 扩展多核能力”做成了一个可发布、可复现、单 crate 交付的教学实验：

- `t2l9-ch1-smp`：基于 `ch1` 的多核启动实验
- `t2l9-ch2-smp`：基于 `ch2` 的多核启动 + 单核执行批处理实验

这个 crate 的设计重点是“一个 crates.io 包即可复现实验”，因此：

- 主包内直接包含两个内核二进制目标
- `ch2` 需要的用户程序源码内置在 `user-src/` 模板目录中，由主包 `build.rs` 动态生成临时 user crate 后自动编译
- QEMU runner 固定为 `-smp 4`
- 关键词按任务要求设置为：`ai` `ai4ose` `kernel` `learning` `os`

## 运行方式

### 运行 ch1 多核实验

```bash
cargo run --bin t2l9-ch1-smp
```

预期现象：

- OpenSBI 选出的 boot hart 通过 SBI HSM 启动其余 3 个 hart
- 4 个 hart 都会输出自己的 hart id
- 所有 hart 到达 S-mode 后系统关机

### 运行 ch2 多核实验

```bash
cargo run
# 等价于 cargo run --bin t2l9-ch2-smp
```

预期现象：

- OpenSBI 选出的 boot hart 负责初始化批处理系统并依次执行 4 个用户程序
- secondary harts 仅完成启动、打印状态并进入 `wfi` 停车
- 输出中能看到：
  - secondary hart parked
  - `Hello, world from user mode program!`
  - `Test power_3 OK!`
  - `Test power_5 OK!`
  - `Test power_7 OK!`

## 测试

```bash
./test.sh
```

测试脚本会依次运行 `ch1` 和 `ch2` 两个目标，并检查核心输出。

## 目录结构

```text
tg-rcore-tutorial-t2l9-smp/
├── .cargo/config.toml
├── build.rs
├── src/
│   ├── lib.rs
│   └── bin/
│       ├── t2l9-ch1-smp.rs
│       └── t2l9-ch2-smp.rs
├── user-src/
│   └── src/
│       ├── lib.rs
│       ├── heap.rs
│       └── bin/
│           ├── 00hello_world.rs
│           ├── 08power_3.rs
│           ├── 09power_5.rs
│           └── 10power_7.rs
└── test.sh
```

## 发布前检查

在真正 `cargo publish` 之前，建议先做三件事：

1. 将当前仓库推到你自己的远程仓库。
2. 为本 crate 打一个与版本一致的 tag，例如 `joshua912815-rcore-tutorial-t2l9-smp-v0.1.0-preview.1`。
3. 根据你的远程仓库地址补全 `Cargo.toml` 中的 `repository` / `homepage`。
