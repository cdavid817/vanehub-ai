# 个性化治理

自定义指令、长期记忆，以及会话层面对两者的限制，都住在同一个限界上下文 `personalization` 里。每个运行时——OnePiece 与每个 CLI 包装的 Agent——通过**一个适配器、一个快照**读取它，而不是各自从设置表里拼自己的提示词。

本章用于引导贡献者。权威需求位于 [unified-personalization-governance](../../../../openspec/specs/unified-personalization-governance/spec.md)；被它取代的两层分别见 [custom-instructions](../../../../openspec/specs/custom-instructions/spec.md) 与 [agent-cross-session-memory](../../../../openspec/specs/agent-cross-session-memory/spec.md)。

## 上下文边界

```text
src-tauri/src/contexts/personalization/
├─ domain/           作用域、合并状态机、记忆记录、快照
├─ application/      用例，以及它们需要的 port
├─ infrastructure/   记忆目录、SQLite 投影、迁移状态
└─ api/              PersonalizationApi —— 外部唯一可以持有的东西
```

两条规则撑住这条边界，且都由 `src-tauri/tests/architecture.rs` 强制：

- **`personalization` 只发布中立的 `PersonalizationApi`（以及它的兼容视图），绝不反向依赖 `agent_runtime` 的 port、基础设施或具体存储**。依赖是单向的。为了省几行而回头去拿一个运行时类型，就是把它倒过来；行数预算不构成理由。
- **上下文只定义 port；适配器与跨上下文装配放在 `bootstrap/`。**`RegistryAgentCapabilities` 与 `GovernedPersonalizationAdapter` 都在 `src-tauri/src/bootstrap/personalization*.rs`，两个都不在上下文内部。

前端镜像同一条边界：React 组件依赖 `src/services/personalization-service.ts`，Tauri 与 Web/mock 两个适配器同步实现它。组件永远不调用 `invoke()`。

## 运行时适配器契约

`agent_runtime` 把自己需要的东西声明成 `AgentPersonalizationSnapshotPort`（`contexts/agent_runtime/application/ports.rs`），由 `GovernedPersonalizationAdapter` 从治理后的策略满足它。

```rust,ignore
fn snapshot(&self, context: GenerationPersonalizationContext) -> AgentPersonalizationSnapshot;
fn pinned_bodies(&self, refs: &[AgentMemoryRef]) -> ...;
```

两条性质属于契约本身，不是实现细节：

- **`snapshot` 永不让一次生成失败**。读不出来的策略产出一个 fail-closed 快照——不注入自定义指令、不使用长期记忆——生成照常进行。没有个性化的回答仍然是回答，拒绝生成不是。只有**经过校验的** last-known-good 策略可以顶上；不存在放行式的默认值。
- **正文与索引分开取**。只有通过相关性筛选的少数几条需要正文，为了建索引而载入全部正文会毁掉这个索引本身要服务的预算。快照之后记录被改动过的 ref 会直接缺席，而不是悄悄换成新版本。

### 快照时序

```text
一轮开始
  └─ resolve_snapshot(agent, session, workspace, 运行时种类, 会话模式)
       ├─ 读策略各层：global → agent → workspace → workspace-agent
       ├─ 按字段跑合并状态机，逐段记录来源
       ├─ 与该运行时声明的能力取交集
       └─ 按当前修订号钉住记忆 ref
  └─ 提示词组装只读快照
  └─ 对通过筛选的少数 ref 调 pinned_bodies
一轮结束
```

快照**每次生成或每个席位轮次取一次，在开始时取**。这正是「轮次中途改设置只影响下一轮」的实现方式，而不是去改写一个已经按旧值规划好的轮次。

会话模式属于解析上下文，不是策略行：`standard`、`project-only`、`temporary` 在会话创建时决定并随会话记录存储——一个模式必须随它约束的那个会话一起消失。

## 作用域优先级与合并状态机

`PersonalizationPolicyScope`（`domain/scope.rs`）有四个变体，优先级固定：

| 作用域 | `precedence_rank` |
| --- | --- |
| `Global` | 0 |
| `Agent { agent_id }` | 1 |
| `Workspace { workspace_key }` | 2 |
| `WorkspaceAgent { workspace_key, agent_id }` | 3 |

workspace 优先于泛化的 Agent 覆盖，因为项目约定默认应该压过对某个 Agent 的个人偏好；workspace-agent 行是显式写下的例外。每层带一个 `InstructionMergeMode`（`inherit`、`append`、`replace`、`disabled`），按字段生效；每个存活的片段都记录了它由哪一层、经哪个动作产生。来源按字段而不是按层记录：一层替换了风格规则、放过了自我描述，会产出两个来源不同的片段，合并记录就会把这件事丢掉。

`scope_key` 由类型化 newtype 用 `/` 拼成，之所以安全，正是因为每个身份 newtype 都拒绝 `/`。用展示文本去拼，一个 workspace 名字就能伪造出另一个作用域的键。

### 哪些写法算同一个工作区

workspace key 由归一化后的路径派生，而「什么算同一个路径」是本机文件系统的事实。`LocalPathRules` 同时携带两条规则，且都以参数传入而不是在使用点读 `cfg!`，这样每条规则在任何平台上都能双向验证：

| 平台 | `fold_case` | `normalize_unicode` |
| --- | --- | --- |
| Windows | ✓ | — |
| macOS | ✓ | ✓ |
| Linux | — | — |

macOS 把同一个名字的合成写法与分解写法当作同一个文件打开，而路径来自文件对话框、shell 还是 git，写法可能不同；同一个目录派生出两个 key，会让这个工作区的记忆被钉在先被记录的那个写法上。在 Linux 上它们确实是两个文件，折叠会把两个真实目录并成一个作用域。归一化在大小写折叠**之前**执行——把分解写法小写化只折叠基字母、留下组合符，与把合成写法小写化得到的字符串并不相同。

远程路径两条规则都不用。远端文件系统的行为在这里无从知晓，套用本机规则会把服务器上互不相同的目录并到一起。

归一化刻意只做字符串处理：canonicalize 会让 key 取决于目录此刻是否存在、符号链接此刻指向哪里，于是重指一个链接或一块盘掉线就会改变工作区身份。

## 记忆：哪一面是权威

| 存储面 | 权威性 | 可重建 |
| --- | --- | --- |
| 记忆目录下的 Markdown 文件 | **权威** | 否 |
| SQLite 投影行 | 派生 | 是 |
| `MEMORY.md` 派生索引 | 派生 | 是 |
| 检索索引条目 | 派生 | 是 |

除文件外的一切都由 reconciliation 重新生成。这也是删除失败必须**逐面上报**的原因：一条记忆文件没了、检索条目还在，它依然会被召回；只报一个布尔值恰好会盖住用户最需要知道的那种情况。

动这块之前必须知道的几条推论：

- **通用文件工具不得写入记忆目录**。所有写入走 v2 应用服务，这是投影、索引与检索保持同步的唯一保证。
- **自动抽取只产生候选，绝不产生活动记录**。写入活动记忆这条路属于一次明确的人类决定。
- **展示名、用户填写的标题、模型字符串都不能作为稳定的记忆身份**。它们会变，id 不会。
- **策略与记忆的编辑一律使用 expected-revision CAS**。绝不 last-response-wins——基于已经前进过的修订号做的保存会被拒绝，并把两边都摆出来。

## 迁移与健康

`MigrationStatePort` 记录从 v2 之前的存储转换到了哪一步。`MemoryHealthPort` 回答**此刻**能否使用已存记忆，这不等于那一行持久状态：一个发现维护被别的进程占着的进程，知道那一行没说的事情。

只有存储处于 `Ready` 时读取才被放行。未完成、进行中或需修复的迁移一律**返回空而不是返回一部分**——调用方会把一部分当成全部，而半份数据比没有数据更危险。

已有的有效记忆无损迁移；解析不出来的条目进隔离而不是被丢弃。迁移、重置与修复都通过显式命名的维护查询枚举存储，绝不复用旧的 200 条上限扫描——那会静默截断。

## 它不接管什么

VaneHub 治理的是**它自己注入什么**，不治理 CLI 在自己进程里做什么：

- **不接管任何 CLI 的内部上下文压缩** —— OnePiece、Claude Code、Codex CLI、OpenCode、Gemini CLI、Antigravity CLI 各自的压缩是它们自己的事。
- **不接管任何 CLI 的原生记忆或指令文件** —— `CLAUDE.md`、`AGENTS.md` 及同类文件从不被写入或改写。

## 检查清单：新增一个由 VaneHub 管理的 Agent 或运行时

下面两条每个新 Agent 都必须做到。都不是可选的，也都不能用「我这边测着能用」来代替。

1. **声明能力**。能力由启动形态在 `RegistryAgentCapabilities::for_launch`（`bootstrap/personalization.rs`）中给出：

   | 启动种类 | 指令 | 记忆索引 | 选中正文 | 自动抽取 |
   | --- | --- | --- | --- | --- |
   | `api` | ✓ | ✓ | ✓ | ✓ |
   | `cli` | ✓ | ✓ | — | — |
   | 其他 | — | — | — | — |

   本次构建没见过的启动形态**什么都不声明**。这是刻意的：忘记声明的适配器必须 fail closed，而不是继承 OnePiece 的完整能力面。如果你的运行时是一种新形态，就在这里显式加上——别让它落进默认分支，然后再去纳闷为什么一条指令都没到。

   运行时不具备的能力压过说它具备的策略值。策略打开也不能让一个 CLI 接受它根本没地方放的注入机制。

2. **调用解析器**。每轮通过 `AgentPersonalizationSnapshotPort` 取一次快照，提示词只从快照组装。不要去读设置表，不要去读记忆目录，也不要再建第二条提示词组装路径。两条组装路径一定会漂移，而漂移的那条正是没人在测的那条。

3. **任何地方都不要写死 Agent 名单**。注册是动态的。`list_capabilities` 之所以去枚举注册表，正是因为：只显示内置 Agent 的界面，在用户添加一个 Agent 的那一刻就错了，而且是静默地错。覆盖这块的测试都会注册一个合成 Agent，用来证明这条路径不依赖任何已知 id。

## 设计所在之处

- [openspec/specs/unified-personalization-governance](../../../../openspec/specs/unified-personalization-governance/spec.md) —— 作用域、合并、记忆治理、会话模式、迁移。
- [openspec/specs/custom-instructions](../../../../openspec/specs/custom-instructions/spec.md) —— 被取代的指令层。
- [openspec/specs/agent-cross-session-memory](../../../../openspec/specs/agent-cross-session-memory/spec.md) —— 被取代的记忆池。
- [跨会话记忆](cross-session-memory.md) —— 共享池与这里的治理如何衔接。
- [Native bounded contexts](native-contexts.md) —— `personalization` 在其余上下文中的位置。
