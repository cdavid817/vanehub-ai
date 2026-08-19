# Git Worktree：让 Agent 在独立工作副本里改代码

## 什么是 Git worktree

**Git worktree 是同一个仓库的第二份工作副本**，有自己的目录和自己的分支，但共享同一份 Git 历史。

它不是复制一遍仓库。`git clone` 会复制全部历史，worktree 只是在另一个目录上再检出一个分支——磁盘代价小，且两边提交的东西在同一个仓库里。

正常情况下一个仓库对应一个工作目录，同一时刻只能停在一个分支上。worktree 打破了这个限制:主仓库(main worktree)之外可以再创建多个**关联工作目录**(linked worktree),每个有独立的工作区文件与 index(暂存区),但**共享同一个 `.git` 数据库**(对象库、引用、配置),所以体积小、创建是秒级的。

对应的原生 Git 命令(VaneHub AI 在后台替你执行的就是这一套):

```bash
# 从当前分支新建 worktree,检出到新分支
git worktree add ../feature-x -b feature-x
# 基于已有分支创建
git worktree add ../hotfix hotfix-branch
# 查看所有 worktree
git worktree list
# 用完移除
git worktree remove ../feature-x
# 清理已删目录但记录未同步的情况
git worktree prune
```

> **同一个分支不能被两个 worktree 同时检出**——Git 会直接报错阻止,因为那样容易让同一分支在两处被改而冲突。所以每个任务/会话必须有独立分支名。

## 传统场景下的用途

在 Agent 之外,worktree 本身也解决几类常见问题:

- **并行开发多分支**——主分支在跑测试/编译时想改紧急 hotfix,不必 stash 现有改动,直接开一个新目录处理。
- **避免频繁切分支的成本**——大仓库切分支会触发大量文件重写、依赖重装(`node_modules`、Rust 的 `target/`),worktree 让不同分支物理隔离。
- **多进程并行构建或测试**——不同 worktree 可同时跑 CI、跑测试,互不冲突。
- **review 时不打断当前工作**——临时把待 review 的分支检出到独立目录看代码,不影响正在进行的开发。

## 在 VaneHub AI 里它解决什么

**让 Agent 改代码，而不碰你正在用的分支。**

没有 worktree 时，Agent 和你在同一个目录上作业：它改到一半你想切分支就切不了，它跑测试时你保存文件会互相干扰，它改错了你的工作区就脏了。

有了 worktree：

- **你的主工作区保持不动**，随时可以切分支、跑自己的东西
- **多个会话可以并行改同一个仓库的不同任务**，各在自己的 worktree 里，文件改动不会冲突
- **改坏了直接丢掉那个 worktree** 就行，主工作区无影响

这就是[使用案例](use-cases.md)里「双 Agent 并行处理同一仓库的不同任务」能成立的原因。

## 为什么 Agent 时代它是刚需

多个 Agent 同时干活时,必须有**物理隔离的文件系统**,否则谁都不敢让 Agent 自主写代码。worktree 在多 Agent 编排里几乎成了基础设施:

**1. 并发 Agent 的天然隔离层**

多个 Agent 并行执行时,若共用一个工作目录会立刻遇到:文件写入互相覆盖、Git index/HEAD 状态冲突(一个在 checkout 另一个在 commit)、无法判断某次改动是哪个 Agent 干的。worktree 让每个 Agent 拿到**独立物理目录 + 独立分支**,天然做到任务级隔离,而且不必为每个 Agent clone 整份仓库(clone 慢、占空间;worktree 共享 `.git` 对象库,创建是秒级)。

```text
        共享 Git 数据库 (commits · blobs · refs)
              ▲            ▲            ▲
              │            │            │
          会话 A       会话 B       Loop 运行
        worktree-a   worktree-b   worktree-c
```

**2. 结果审查与回滚极简**

每个 Agent 的改动天然收敛成一个分支上的一组 commit。审查时 `git diff base..worktree-branch` 就能看清它到底改了什么;通过就合并,不通过就 `git worktree remove` + 删分支,一点痕迹不留——不像"共享目录 + 事后 diff 整个仓库"那样分不清哪些改动来自谁。

**3. 权限边界与文件系统边界重合**

给每个任务分配独立 worktree + 独立分支作为"信封",Agent 只在自己的信封里活动。这样**不需要额外写沙箱逻辑**去限制 Agent 只能碰某些文件——操作系统层面的目录隔离本身就是边界。

**4. 支撑"失败可丢弃"的乐观并发**

Agent 生成的代码不一定对。可以让多个 Agent 对同一问题给出不同方案,各跑在自己的 worktree 里互不干扰,最后挑一个合并、其余整个丢弃。没有 worktree 的话这种"多方案探索、择优合并"代价会高很多(要么串行跑,要么每个方案 clone 一份)。

## 什么时候能用

创建会话时选目录，界面会先检视这个目录是不是 Git 仓库，并标出来：

| 标记 | 含义 |
| --- | --- |
| **Git** | Git 项目，可创建 worktree |
| **文件夹** | 普通文件夹，**worktree 选项被隐藏或禁用** |

**这次检视不会启动任何 Agent、不会开交互会话**——它只是看一眼目录。

非 Git 项目仍然可以正常创建会话，只是没有 worktree 这个选项。

## 路径与分支怎么定

勾选**创建新 Git worktree** 后填一个 **Worktree 名称**，路径和分支按固定规则派生：

| | 规则 | 例子（项目 `C:\code\app`，名称 `feature-a`）|
| --- | --- | --- |
| **路径** | 项目**同级目录** + `项目名-worktree名` | `C:\code\app-feature-a` |
| **分支** | `vanehub/worktree名` | `vanehub/feature-a` |

会话创建成功后，**这个 worktree 路径就是该会话的有效工作目录**——Agent 看到的、命令执行的、文件浏览的，都是它。

## 三种会被提前拒绝的情况

都在**执行任何 Git 命令之前**就拒绝，不会留下半个 worktree：

| 情况 | 结果 |
| --- | --- |
| **名称为空或不安全** | 拒绝创建会话 |
| **目标路径已存在** | 在 `git worktree add` 之前拒绝 |
| **Git 不可执行** | 界面收到简洁的不可用提示 |

**失败信息是分层的**：界面上只给一句简洁说明，`git worktree add` 的完整 stdout、stderr 和诊断写进统一日志。所以界面不会糊你一屏 Git 输出，但排查时信息一条不少——见[可观测性](observability.md)。

## Loop 的 worktree 是另一套

[Loop Engineering](loop-engineering.md) 每次启动运行都会**自己建一个专属 worktree 和分支**，且在创建角色会话或改动任何项目文件**之前**就建好。

它与你手工建的 worktree 有三点不同：

- **分支名是防冲突的**，不是固定的 `vanehub/名称`
- 运行会持久化规范项目路径、worktree 路径、worktree 名和分支
- **执行者与验证者的所有会话、以及全部验证命令，都以这个 worktree 为有界根目录**

目标路径或分支与已有目标冲突时，**准备阶段就失败**，在创建角色会话或改动文件之前，并保留简洁的失败上下文与脱敏诊断。

### Loop 的 worktree 不会被自动清理

**运行成功、失败、取消、被拒绝或重启恢复之后，worktree 一律保留**，直到你自己在这个功能之外处理它。

系统**不会自动执行 `git worktree remove`、不会删分支、不会合并、不会提交**。它只把路径暴露给你去检查。

这是有意的：一次自动运行的产物，在你看过之前不该被自动清掉。

## 注意事项与限制

- **仅桌面端可用**，依赖本机 Git 可执行文件。
- **仅对 Git 项目可用**；普通文件夹没有这个选项。
- **远端工作区不支持 worktree**——只能指向远端已存在的路径。因此 [Loop Engineering](loop-engineering.md) 也**不适用于远程工作区**。
- **目标路径已存在就会被拒绝**，不会覆盖或复用。
- **Loop 的 worktree 永不自动清理**，累积的目录需要你自己管理。
- **VaneHub AI 不会替你提交、合并或推送** worktree 里的改动。

### 用多了要注意的几点

| 注意事项 | 说明 |
| --- | --- |
| **数量要有上限** | 磁盘 I/O 与文件句柄是有限资源。仓库含 `node_modules`/`target` 这类重目录时,每个 worktree 默认要各装一份依赖——建议用 pnpm/cargo 的全局 cache,或对依赖目录做符号链接共享 |
| **命名要可追溯** | worktree 目录名最好能直接映射到任务/会话,出问题时好定位是哪次留下的。VaneHub AI 的 `项目名-worktree名` 与 `vanehub/worktree名` 就是这个用意 |
| **清理要主动做** | Loop 的 worktree 不会自动清理,跑久了会堆积。定期 `git worktree list` 看一眼,清掉的目录记得 `git worktree prune` 同步记录 |
| **分支不能复用** | 同一分支不能被两个 worktree 同时检出,所以每个任务必须有独立分支名 |

## 相关

- 创建会话时怎么勾选 → [创建第一个会话](first-session.md)
- 依赖 worktree 的自动循环 → [Loop Engineering 工程](loop-engineering.md)
- 并行改同一仓库的完整走法 → [使用案例](use-cases.md)
- Git 失败详情去哪看 → [可观测性](observability.md)
- 执行隔离在多 Agent 编排里的位置 → [多 Agent 系统技术架构](../../../agent-infrastructure/multi-agent-architecture.md)
