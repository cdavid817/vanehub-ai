# 跨会话记忆

存储的记忆是一个主机级共享池，OnePiece 和所有 CLI 包装的 Agent（`claude-code`、`codex-cli`、`gemini-cli`、`opencode`、`antigravity-cli`）都从中读取——但共享现在是**默认值，不是规则**：每条记忆自带作用域与受众。两者的治理见[个性化治理](personalization-governance.md)，本章讲持久化这一半；检索/搜索路径见 [Retrieval and vector search](retrieval.md)。

## 默认共享，可按条收窄

每条记忆都记录了产生它的 Agent 与 workspace。其中两项现在是有效约束，而不只是备注：

- **作用域** —— `global` 或某一个 workspace。workspace 作用域的记忆，对没有指明 workspace 的调用方不会被解析出来。
- **受众** —— 默认全部 Agent，也可以只给记录上列出的那几个 Agent id。

来源元数据仍与这两者分开记录：**「由谁记录」不等于「谁可读取」**。一个 Agent 产生的记忆可以只让另一个读到，两个字段都不会作为检索工具的输入暴露。

在没有 workspace 文件夹的会话中保存的记忆按 `global` 存入而不是被拒绝，可从任何 workspace 或无 workspace 的情况下读取。

## 保存记忆

- **OnePiece** 在其自身的 API tool-calling loop 中暴露一个记忆工具。该工具产生的是**待审候选**，不是活动记录——自动路径只提议，由人来决定。
- **CLI 包装的 Agent** 不暴露该工具，因为 VaneHub 不控制 CLI 包装 Agent 自身的内部工具系统。它们通过单独的自动抽取机制产生候选，该机制依附于上下文压缩。
- **用户自己写下的记忆**直接生效：作者就是人本身，没有第二个人需要复核。

CLI 包装 Agent 的抽取走 OnePiece 的 provider,因为这些 CLI 不暴露可复用的模型凭据。OnePiece 没有配置 provider 时,它们完全不产生抽取。

## 记忆存储与产生机制

记忆以文件形式落盘到一个主机级共享的 `memory/` 目录,并由一个 `MEMORY.md` 索引文件汇总条目。**文件是权威面**,SQLite 投影行、`MEMORY.md` 索引与检索条目都是派生的、可重建的。

生产写入路径现在只有一条:`personalization` 上下文的 v2 应用服务。旧的 `FileAgentMemoryStore` 已不再挂在任何写入端口上,目录只有一个主人;它留下的那一个 `list_all` 是显式命名的维护枚举,供行存储转换读取它正在转换的源。通用文件工具**不得**直接写这个目录:绕过应用服务就等于让投影、索引与检索三面各自漂移。

下面的流程图展示了三条产生路径如何汇入同一个存储——注意自动的两条**停在待审队列**,只有人的决定才会写出活动记录。

```mermaid
flowchart LR
    subgraph 存储[主机级 memory/ 目录与派生面]
        MEM[memory/*.md 权威文件]
        IDX[MEMORY.md 派生索引]
        PROJ[SQLite 投影行]
    end

    SERVICE[personalization v2 应用服务<br/>唯一写入路径]
    QUEUE[待审候选队列]

    subgraph 产生路径
        P1[OnePiece 记忆工具<br/>tool-calling loop 中]
        P2[OnePiece 自动抽取<br/>随对话压缩触发]
        P3[CLI Agent<br/>由 OnePiece 代做]
        P4[用户自己写下]
    end

    P1 --> QUEUE
    P2 --> QUEUE
    P3 --> QUEUE
    QUEUE -->|人做出决定| SERVICE
    P4 --> SERVICE
    SERVICE --> MEM
    SERVICE --> IDX
    SERVICE --> PROJ
```

CLI 包装的 Agent(`claude-code`、`codex-cli`、`gemini-cli`、`opencode`、`antigravity-cli`)本身不暴露保存记忆的工具,VaneHub 不控制它们内部的工具系统。它们的记忆由 OnePiece 在 generation 结束时代为抽取,复用 OnePiece 已有的 provider 凭据与抽取逻辑,整个过程对 CLI Agent 透明。下面的时序图展示了这条路径,两条关键约束是**抽取只提交候选**,以及**所有失败只记录日志,绝不阻塞生成**。

```mermaid
sequenceDiagram
    participant Gen as CLI Agent generation
    participant OpLoop as OnePiece 代做循环
    participant Extract as extract_and_save_memory
    participant Queue as 待审候选队列

    Gen->>OpLoop: generation 完成
    OpLoop->>OpLoop: is_cli_kind 判定为 CLI Agent
    OpLoop->>Extract: 复用 OnePiece 凭据<br/>调用 extract_and_save_memory
    Note over Extract: 不发起任何工具调用<br/>直接从对话文本抽取
    alt 抽取成功
        Extract->>Queue: 提交候选
        Note over Queue: 在人做出决定之前<br/>一条都没有存入
    else 抽取或提交失败
        Extract-->>OpLoop: 仅 log,不抛错
        Note over OpLoop: 不阻塞后续生成
    end
```

每条记忆记录上的 `provenance` 字段(`agent_id`、`folder`、`source`、`created_at`)承载的是**来源**:这条记忆由哪个 Agent、在哪个 workspace 文件夹、经由哪条路径、何时产生。它用于追溯与筛选展示,**不决定谁能读到它**——那由记录上单独的作用域与受众决定。把"谁记的"当成"谁能读"是这次治理改造要终结的那个等价关系:同一个 Agent 记下的两条记忆完全可以有不同的受众。

## 关键类型与常量

### 存储模型

记忆存储是主机级共享的 `memory/` 目录,不是数据库行;**文件是权威面**,每条记忆对应一个 `{id}.md` 文件。id 由存储生成而不是从名字派生——v2 允许重名,而把名字当文件名会让两条同名记忆互相覆盖。`MEMORY.md` 索引、SQLite 投影行与检索条目都是从文件重建出来的派生面。

### MemoryMetadata frontmatter

每条记忆文件的 frontmatter 解析为 `MemoryMetadata`,字段包括 `name`(可读的展示名称,可修改;**不是文件名**——文件名是上一节说的不可变 `{id}.md`)、`description`(概述)、`memory_type`(闭集四值 `user`/`feedback`/`project`/`reference`;缺失或未知值降级为 `untyped`,不拒绝写入也不拒绝读取),以及 `provenance` 来源元数据(`agent_id`、`folder`、`source`、`created_at`,迁移场景下另带 `migrated_from`)。frontmatter 只读前 `MAX_FRONTMATTER_LINES=30` 行的窗口,避免把整份正文当 frontmatter 解析。

### 枚举与产生路径

治理后的枚举是分页的,并且**不复用旧的 `MAX_SCANNED_FILES=200` 扫描**——那个上限只剩在旧目录读取器上,迁移、重置与修复都走显式命名的维护查询,否则一个超过 200 条的存储会被静默截断。三条自动产生路径都只提交候选:

- **OnePiece 记忆工具** —— 工具名常量 `REMEMBER_TOOL_NAME="remember"`,在 OnePiece 自身的 API tool-calling loop 中暴露。它产生的是待审候选,不是活动记录。
- **OnePiece 自动抽取** —— 随对话压缩触发(`extract_memories_accounted`),单次压缩最多产生 `MAX_MEMORY_ACTIONS=10` 条记忆动作,超过即截断。
- **CLI Agent 由 OnePiece 代做** —— `extract_and_save_memory` 复用 `ONEPIECE_AGENT_ID="onepiece"` 的凭据与抽取逻辑,不发起任何工具调用,直接从对话文本抽取,对 CLI Agent 透明。

第四条是用户自己写下的那条:作者是人本身,直接进入活动记忆。

### 注入边界

记忆注入按调用方分两套预算。`ONEPIECE_MEMORY_INDEX_BOUNDS` 为 `lines: 200, bytes: 12000`,OnePiece 调用方还会注入记忆 `body` 正文;`CLI_MEMORY_INDEX_BOUNDS` 为 `lines: 40, bytes: 3000`,CLI 调用方只注入索引行,不注入 `body`。注入时单次最多选取 `MAX_SELECTED_MEMORIES=5` 条记忆。所有失败只 `log`,不阻塞已交付的生成结果——记忆是增强而非必需,抽取或注入失败都不应影响主路径。

## 设计所在之处

本章用于引导贡献者。权威需求位于 spec 中。

- [openspec/specs/unified-personalization-governance](../../../../openspec/specs/unified-personalization-governance/spec.md) —— 作用域、受众、候选审查与迁移。
- [openspec/specs/agent-cross-session-memory](../../../../openspec/specs/agent-cross-session-memory/spec.md) —— 共享池、来源元数据和保存路径。
- [openspec/specs/retrieval-vector-search](../../../../openspec/specs/retrieval-vector-search/spec.md) —— 检索工具与降级。

记忆持久化与治理位于 `personalization` 限界上下文,召回位于 `retrieval`;见 [Native bounded contexts](native-contexts.md)。
