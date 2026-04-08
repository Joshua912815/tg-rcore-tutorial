# Chapter6 基础实验记录

## 一、实验任务与完成情况

本次完成的是 `tg-rcore-tutorial-ch6` 的基础实验。根据 `codex.md` 与 `tg-rcore-tutorial-ch6/exercise.md` 的要求，本章需要完成以下内容：

1. 实现 `linkat`、`unlinkat`、`fstat` 三个系统调用。
2. 修改 `easy-fs`，让单级目录文件系统支持硬链接、链接删除和 inode 回收。
3. 保持前向兼容，能够继续通过前面章节继承下来的关键测例，尤其是 chapter4/5 的 `mmap/munmap`、`spawn/wait/waitpid` 等行为。
4. 按 `codex.md` 的要求，记录“与 AI 合作的实现过程”和“学习效果评估”，并明确写清交互方式、问题/bug/解决过程、验证过程、能力提升、与校内教程的对比。
5. 在 `rcore-docker` 环境中完成验证，并在开始测试前清理旧容器，避免缓存和残留进程干扰。

本次实验已经完成。最终验证结果如下：

- 测试前已清理旧的 `rcore-docker` 实验容器：`codex-ch5-lab`、`codex-ch4-lab`、`rcore-lab-container`、`my-rcore`、`serene_robinson`、`rcore-container`。
- 使用全新容器 `codex-ch6-lab` 完成验证。
- `cargo check --features exercise` 通过。
- 手动运行 `cargo run --features exercise` 并在 shell 中执行 `ch6_usertest`，所有 chapter6 用户测例通过。
- `./test.sh exercise` 通过，checker 结果为 `Test PASSED: 33/33`。
- `./test.sh base` 通过，checker 结果为 `Test PASSED: 15/15`。

这说明本章新增的硬链接与文件状态功能已经完成，同时 chapter6 对 chapter3/4/5 的前向兼容也已经恢复并验证通过。

## 二、与 AI 合作的实现过程

### 1. 交互方式

本次没有采用 chapter3 那种“模拟聊天记录”写法，而是把 AI 当成一个真实的协作式编程助手，交互方式主要分成六类：

1. 先让 AI 阅读 `codex.md`、`tg-rcore-tutorial-ch6/exercise.md`、chapter6 源码和 checker，确认本章真正的任务边界。
2. 再让 AI 直接阅读 `tg-rcore-tutorial-user/src/bin/ch6_file1.rs`、`ch6_file2.rs`、`ch6_file3.rs`，根据测例反推 `fstat/link/unlink` 的精确语义。
3. 在真正改代码前，让 AI 判断最小改动路径，决定是“改磁盘 inode 格式存 nlink”，还是“扫描目录项动态计算链接数”。
4. 在发现 chapter6 仍残留 chapter5 的占位实现后，让 AI 对照 chapter5 已完成版本，补齐 `spawn/set_priority/mmap/munmap` 和调度循环修复，而不是只做本章新增 syscall。
5. 在 Docker 验证时，不只看 `ch6_usertest` 的表面结果，还继续查看运行日志和 checker 输出，防止被“包装器通过”误导。
6. 最后让 AI 把实现过程整理成可直接进入最终总报告的材料，而不是保留低质量、不可复用的对话体记录。

这种交互方式的核心分工是：

- AI 负责加速代码定位、跨章节比对、bug 归因、验证路径设计。
- 我负责判断哪些建议真正符合实验语义，尤其是文件系统回收逻辑和前向兼容要求，避免把看似合理但不自洽的实现直接写入内核。

### 2. 需求分析与代码定位

AI 先帮助我确认了 chapter6 的工作不只是“补三个 syscall”，而是至少包含三层内容：

1. syscall 层：`linkat`、`unlinkat`、`fstat` 的用户指针读取、错误返回和结构体回填。
2. 文件系统层：目录项复用、硬链接创建、删除最后一个链接时 inode/data block 回收。
3. 兼容性层：chapter6 必须仍然通过前面章节的关键用户测例，而当前 `ch6` 源码里实际上还保留了 chapter5 的占位实现。

为此，AI 协助我重点阅读了这些位置：

- `codex.md`
- `tg-rcore-tutorial-ch6/exercise.md`
- `tg-rcore-tutorial-ch6/src/main.rs`
- `tg-rcore-tutorial-ch6/src/fs.rs`
- `tg-rcore-tutorial-ch6/src/process.rs`
- `tg-rcore-tutorial-ch6/src/processor.rs`
- `tg-rcore-tutorial-easy-fs/src/vfs.rs`
- `tg-rcore-tutorial-easy-fs/src/efs.rs`
- `tg-rcore-tutorial-checker/src/cases/ch6.rs`
- `tg-rcore-tutorial-user/src/bin/ch6_file1.rs`
- `tg-rcore-tutorial-user/src/bin/ch6_file2.rs`
- `tg-rcore-tutorial-user/src/bin/ch6_file3.rs`
- `tg-rcore-tutorial-ch5/src/main.rs`
- `tg-rcore-tutorial-ch5/src/process.rs`
- `tg-rcore-tutorial-ch5/src/processor.rs`

这一步最重要的结论有两个：

- `easy-fs` 原实现只有“目录项 -> inode”的最基本路径，没有链接计数，也没有 inode 位图回收接口，因此硬链接功能不能只在 syscall 层补。
- chapter6 虽然主要主题是文件系统，但 checker 明确要求它继续满足 chapter5 exercise 中的 `spawn/wait/waitpid`、chapter4 的 `mmap/munmap` 等行为，因此必须把遗留的占位代码一并补齐。

### 3. 实现阶段

本次最终修改主要集中在两部分：`tg-rcore-tutorial-easy-fs` 和 `tg-rcore-tutorial-ch6`。

#### 3.1 `easy-fs` 的硬链接与回收支持

AI 帮我比较了两条实现路线：

- 路线 A：修改磁盘 inode 布局，新增持久化 `nlink` 字段。
- 路线 B：不改磁盘格式，直接扫描根目录目录项动态计算链接数。

最终选择了路线 B，原因是：

1. chapter6 只支持单级目录，目录项扫描成本很低。
2. 不修改磁盘 inode 布局，能避免破坏当前 `fs.img` 生成路径和已有文件系统镜像格式。
3. 对本章测例来说，动态统计链接数已经足够，并且更容易和当前 easy-fs 结构兼容。

具体实现包括：

- 在 `Inode` 中增加 `inode_id` 元数据，便于 `fstat` 返回 `ino`。
- 为目录 inode 增加 `link`、`unlink`、`count_links` 等方法。
- 让目录创建和硬链接插入都优先复用“已删除但仍占位”的空目录项，而不是无限向目录尾部追加。
- 在 `EasyFileSystem` 中增加 `dealloc_inode()`，支持最后一个硬链接删除后回收 inode 位图。
- 在 `FileSystem::unlink()` 中，在目录项删除后统计剩余链接数；若已归零，则清空文件数据块并回收 inode。

这样，`ch6_file2` 和 `ch6_file3` 所依赖的两个核心语义都被覆盖了：

- 多个文件名可以指向同一个 inode。
- 最后一个链接删除后，文件数据和 inode 资源都能真正回收，下一次同名创建不会因为陈旧 inode 或目录项残留失败。

#### 3.2 chapter6 syscall 与前向兼容补齐

在 `tg-rcore-tutorial-ch6/src/main.rs` 和 `src/fs.rs` 中，本次补齐了：

- `linkat`
- `unlinkat`
- `fstat`

其中 `fstat` 的实现重点是：

- 从当前进程的 fd_table 中取出文件句柄。
- 若 fd 对应普通文件，则根据 inode 号、文件类型和目录项统计结果构造 `Stat`。
- 通过地址翻译把 `Stat` 写回用户空间。

但真正影响验证结果的关键，不只是这三个 syscall。本次还把 chapter5 已经完成但 chapter6 尚未前移的实现一起补了回来，包括：

- `spawn`
- `set_priority`
- `mmap`
- `munmap`
- `Process` 中的 `stride/priority`
- `ProcManager` 的 ready queue 去重与调度步长推进
- 主调度循环里“不要长时间持有当前任务引用”的修复

这里有一个很重要的经验：  
如果我只按 `exercise.md` 字面要求去补 `linkat/unlinkat/fstat`，手动跑 `ch6_usertest` 甚至可能表面上“看起来通过”，但 checker 仍会因为 chapter5 相关输出缺失而失败。AI 在这一步的价值，主要体现在它能快速发现 chapter6 实际上有前向兼容缺口，而不只是功能缺口。

## 三、问题、bug 与解决过程

### 问题 1：`easy-fs` 原生不支持硬链接，也没有 inode 回收入口

**现象：**

- `tg-rcore-tutorial-ch6/src/fs.rs` 中的 `link/unlink` 直接是 `unimplemented!()`
- `easy-fs` 只能创建新 inode，不能统计链接数，也不能回收 inode 位图

**AI 的作用：**

- 帮我明确问题不在 syscall 层，而在文件系统层
- 对比“改磁盘格式”和“目录项扫描”两种方案后，给出更稳妥的最小实现路径

**解决办法：**

- 在 `vfs.rs` 中加入目录项读取、插入、删除和链接计数逻辑
- 在 `efs.rs` 中加入 `dealloc_inode()`
- 在 chapter6 的 `FileSystem::unlink()` 中实现“删除目录项 -> 统计剩余链接 -> 清空数据块并回收 inode”

### 问题 2：第一次手动跑 `ch6_usertest` 时，表面通过，但日志暴露 `spawn` 仍未实现

**现象：**

- `ch6_usertest` 最后打印了 `ch6 Usertests passed!`
- 但日志中反复出现 `spawn: parent pid = 3, not implemented`

这说明当时“总测例通过”其实只是包装器行为，并不能证明 chapter5 继承测例真的执行正确。

**AI 的作用：**

- 没有停留在表面结果，而是继续检查日志
- 立即判断出 chapter6 还残留 chapter5 的未完成代码，必须继续补齐

**解决办法：**

- 直接阅读 chapter5 已完成实现
- 将 `spawn`、`set_priority`、`mmap`、`munmap`、调度器和 trap 主循环修复前移到 chapter6
- 重新编译并再次手动运行 `ch6_usertest`

第二次运行后，`spawn` 已能正常生成子进程 PID，`Test spawn0 OK!` 和 `Test waitpid OK!` 等输出也真实出现。

### 问题 3：`fstat` 初版实现因为 `Stat` 私有字段导致编译失败

**现象：**

第一次 `cargo check --features exercise` 失败，报错原因是：

- `Stat` 的 `pad` 字段是私有的
- 不能用 `..Stat::new()` 的结构体更新语法直接构造

**AI 的作用：**

- 快速从编译错误里定位到具体字段封装问题，而不是去怀疑地址翻译或文件描述符表

**解决办法：**

- 改成 `let mut stat = Stat::new();`
- 然后逐字段填 `dev`、`ino`、`mode`、`nlink`

### 问题 4：`Range` 导入位置错误，导致编译不过

**现象：**

- 我把 `Range` 加在了根模块的 `use` 列表里
- 真正需要它的是 `impls` 子模块，因此会出现“根模块未使用、子模块找不到类型”的双重问题

**解决办法：**

- 把 `Range` 的导入移动到 `impls` 模块内部

这个问题虽然不大，但说明在 `no_std + 子模块` 的场景下，作用域细节会直接触发 `deny(warnings)` 级别的失败。

### 问题 5：主调度循环延续了 chapter5 的旧风险写法

**现象：**

- chapter6 主循环仍然把 `find_next()` 返回的 `&mut Process` 长时间保存在局部变量 `task` 中
- 但 syscall 处理中，`spawn/fork` 又会修改进程管理器内部结构

这和 chapter5 中已经出现过的风险一致：  
持有进程表内部引用时继续修改进程表，容易让后续返回值写回和当前任务状态处理出现不可靠行为。

**AI 的作用：**

- 通过对照 chapter5 已完成版本，直接指出这个问题在 chapter6 中也还存在

**解决办法：**

- 把 chapter5 已验证的修复方式前移到 chapter6
- Trap 返回后重新获取当前进程
- 不再跨越 syscall 分发过程长期持有旧的 `task` 引用

### 问题 6：Docker 环境验证前必须清理旧容器，否则容易混淆

**现象：**

测试前发现本地还残留多个旧实验容器，包括：

- `codex-ch5-lab`
- `codex-ch4-lab`
- `rcore-lab-container`
- `my-rcore`
- `serene_robinson`
- `rcore-container`

这些容器会增加“我到底是在新的环境里验证，还是在旧状态里重复利用缓存”的不确定性。

**解决办法：**

- 先统一删除旧的 `rcore-docker` 容器
- 再创建全新的 `codex-ch6-lab`
- 验证结束后，再把 `codex-ch6-lab` 本身也删掉，避免给下一轮实验留下新的冲突源

## 四、验证过程

本次实际采用的验证流程如下。

### 1. 清理旧容器

先删除旧的实验容器，确保验证发生在全新环境中：

```bash
docker rm -f codex-ch5-lab codex-ch4-lab rcore-lab-container my-rcore serene_robinson rcore-container
```

### 2. 启动新的 chapter6 容器

```bash
docker run -d --name codex-ch6-lab \
  -v /Users/joshua/Desktop/大二下课程资料/tg-rcore-tutorial:/mnt \
  -v /tmp/rcore-rustup:/usr/local/rustup \
  -w /mnt/tg-rcore-tutorial-ch6 \
  -e TG_USER_DIR=/mnt/tg-rcore-tutorial-user \
  -e RUSTFLAGS=-Aunsafe_op_in_unsafe_fn \
  rcore-docker sleep infinity
```

### 3. 编译检查

```bash
docker exec codex-ch6-lab bash -lc 'cd /mnt/tg-rcore-tutorial-ch6 && cargo check --features exercise'
```

结果：通过。

### 4. 手动运行总测例，检查运行时日志

```bash
docker exec -it codex-ch6-lab bash -lc 'cd /mnt/tg-rcore-tutorial-ch6 && cargo run --features exercise'
# 进入 shell 后执行
ch6_usertest
```

关键结果：

- `Test fstat OK!`
- `Test link OK!`
- `Test mass open/unlink OK!`
- `ch6 Usertests passed!`

这一步的作用主要是观察运行时细节，确认 `spawn` 已真实执行、硬链接行为正常，而不只看最后一行字符串。

### 5. 使用 checker 做正式验证

先让脚本自动安装 checker，再跑 exercise：

```bash
docker exec codex-ch6-lab bash -lc 'cd /mnt/tg-rcore-tutorial-ch6 && ./test.sh exercise'
```

结果：

- `Test PASSED: 33/33`
- `✓ ch6 练习测试通过`

随后补跑 base：

```bash
docker exec codex-ch6-lab bash -lc 'cd /mnt/tg-rcore-tutorial-ch6 && ./test.sh base'
```

结果：

- `Test PASSED: 15/15`
- `✓ ch6 基础测试通过`

### 6. 验证后清理本轮容器

```bash
docker rm -f codex-ch6-lab
```

这样本轮实验不会给后续 chapter8 或重复验证留下新的容器残留。

## 五、学习效果评估

### 1. 能力提升

这次实验对我的提升，主要体现在四个方面。

#### 1.1 对文件系统“名字”和“对象”分离的理解更具体

以前对硬链接的理解偏概念化，只知道“两个名字指向同一个文件”。  
这次实际修改 easy-fs 后，理解更具体了：

- 目录项保存的是“名字 -> inode 编号”的映射
- 真正的文件内容和元数据属于 inode
- `unlink` 删除的是目录项，而不是“按名字删文件内容”
- 只有最后一个链接删除后，inode 和数据块才应该回收

这比只读教材时的理解更接近真实实现。

#### 1.2 对“前向兼容”有了更工程化的认识

chapter6 这次最容易掉进去的坑，就是只盯着 `linkat/unlinkat/fstat` 本身。  
但真正通过 checker，必须把 chapter4/5 的行为一起维护住。

这让我更清楚地意识到：

- 操作系统实验不是一组彼此孤立的小题
- 每一章都是在上一章基础上的增量演进
- 新功能正确，不代表整体系统没有回归

这种“跨章节回归验证”的意识，是这次实验里比代码本身更重要的收获。

#### 1.3 对验证方法的判断更谨慎了

本次最有代表性的教训是：  
第一次手动运行 `ch6_usertest` 表面上显示通过，但日志里其实已经暴露出 `spawn` 仍未实现。

这让我明确认识到：

- 不能把“程序跑完了”直接等同于“实现正确”
- 需要同时看运行日志、用户测例输出和 checker 结果
- 发现异常日志时，必须继续追，而不是只看最后一行 `passed`

这比单纯“把代码写出来”更接近真实工程中的测试思维。

#### 1.4 对 Docker 化实验环境的控制更熟练

之前更容易把容器当作“只要能跑就行”的黑盒。  
这次则更清楚地意识到：

- 测试前清理旧容器是必要的
- 新容器命名要固定，便于追踪
- 验证结束后主动删除，也是一种实验环境管理能力

这部分虽然不是操作系统理论本身，但对后续实验稳定性影响很大。

### 2. 与校内教程的对比

这里先给出可直接用于总报告的定性比较；如果后续需要做更正式的定量对比，可以再结合 `/reports` 中材料补充。

#### 2.1 AI 协作方式的优势

和传统校内教程的“按步骤手动读文档、自己慢慢排查”相比，AI 协作方式的优势主要有三点：

1. **代码定位更快。**  
   这次 AI 能很快把范围从 chapter6 扩展到 `easy-fs`、checker、用户测例、chapter5 已完成实现，减少了我在仓库里盲目翻找的时间。

2. **跨章节比对更高效。**  
   chapter6 的真正难点之一，是发现它还残留 chapter5 的占位实现。单靠线性阅读当前章节文档，不一定会这么快发现；AI 在跨文件、跨章节检索上明显更高效。

3. **bug 归因更快收敛。**  
   比如“手动 `ch6_usertest` 看似通过但日志异常”这种问题，AI 能迅速把注意力从“是不是 chapter6 新 syscall 还有 bug”转到“是不是前向兼容没有补全”，帮助我少走弯路。

#### 2.2 校内教程方式仍然有价值的地方

和 AI 协作相比，校内教程的传统方式也有两个不可替代的优点：

1. **对基础原理的强制性更强。**  
   如果完全依赖 AI，很容易直接接受一个“能跑的方案”，而忽略 inode 生命周期、目录项复用、前向兼容这些背后的设计理由。

2. **更能暴露自己的知识短板。**  
   没有 AI 辅助时，很多地方必须自己从代码结构推出设计意图；这个过程更慢，但也更容易暴露自己哪一部分没有真正懂。

#### 2.3 本次实验后的判断

结合这次 chapter6 的实际体验，我认为更合理的方式不是二选一，而是：

- 用校内教程保证理论主线和章节目标不跑偏
- 用 AI 加速代码定位、跨章节对照、bug 排查和验证组织

如果只依赖校内教程，效率偏低，尤其在仓库规模变大、跨章节回归增多后更明显。  
如果只依赖 AI，又容易把“接受答案”误当成“真正理解”。  
这次最有效的做法，是把 AI 当成高效助手，而不是替代理解过程的黑箱。

## 六、本章结论

本次 chapter6 基础实验已经完成，且记录方式已改为可直接纳入最终总报告的材料，而不是模拟聊天记录。

从结果上看，本章不仅补齐了 `linkat/unlinkat/fstat` 和 easy-fs 的硬链接支持，还顺带修复了 chapter6 对 chapter5/4 的前向兼容缺口，最终在全新 Docker 环境中通过了：

- `./test.sh base`
- `./test.sh exercise`

从学习价值上看，这次实验最大的收获不是“又多实现了三个 syscall”，而是更清楚地理解了：

- 硬链接本质上是目录项与 inode 的关系问题
- 文件删除本质上是资源生命周期管理问题
- 新章节实验必须带着前向兼容意识来做
- AI 协作最有价值的地方，不是替我写代码，而是帮助我更快发现系统性问题
