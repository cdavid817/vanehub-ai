# OnePiece 工具目录补强 — 设计文档

- 日期：2026-08-05
- 状态：已确认，待转 OpenSpec proposal
- 分支：`worktree-onepiece-agent-enhancement`

> 本文是 superpowers brainstorming 产出的设计文档，**不是 OpenSpec 提案**。按 `AGENTS.md`，动代码前仍需在 `openspec/changes/` 下起 proposal 并通过 `openspec validate --strict`。本文是该 proposal 的输入。

## 1. 背景与动机

OnePiece 是 VaneHub 内置的原生 Agent（`launch_kind = api`），自 `add-onepiece-native-agent` 落地后未做过功能补充。其 Provider Profile 体系（25 家 catalog 供应商、模型发现、凭证隔离、API key 校验）已完备，但**内置工具只有三个**：`shell`、`file`（整文件 read/write）、`remember`。

四个具体问题：

1. **搜索代码必须弹审批。** 没有 grep/glob，找代码只能走 `shell`，而 `shell` 在 `risk_tier_for` 中固定为 `RequiresApproval` —— 每一次搜索都打断用户。
2. **`file` 没有任何执行边界。** `shell_tool` 有 60s 超时、64KB 输出上限、取消信号、`audit_command` 审计；`file_tool` 的 read 是裸 `read_to_string`，无大小上限、无超时、无取消。读大文件会直接撑爆上下文。
3. **改一行要重写整个文件。** `write` 只有全量覆写，大文件上 token 消耗与正确性双输。
4. **跨平台缺口。** 即使退而求其次走 `shell`，Windows 下是 `cmd /C`，`grep`/`rg` 通常不存在。

Plan mode 下问题更严重：`plan_mode_tool_catalog()` 只提供 `file(read)` + `remember`，模型**完全无法搜索**，只能猜路径盲读。

## 2. 范围

**本次做：** `grep`、`glob`、`edit` 三个新工具 + `file` read 的边界补强。实现方式为纯 Rust 内建。

**本次不做**（各自独立成 change，依赖顺序见附录 A）：`api_process_adapter` 拆解与崩溃恢复、任务分解 / todo 规划、子 Agent 树、外部信息接入、插件注册自定义 mode、配置与会话体验。

## 3. 工具契约

| 工具 | 参数 | 风险层 | Plan Mode |
|---|---|---|---|
| `grep` | `pattern`(必填, 正则)、`glob?`、`path?`、`output_mode?`、`context?`、`case_insensitive?`、`head_limit?` | `AutoApprove` | 可用 |
| `glob` | `pattern`(必填)、`path?` | `AutoApprove` | 可用 |
| `edit` | `path`、`old_string`、`new_string`(均必填)、`replace_all?`(默认 `false`) | `RequiresApproval` | 硬拒 |
| `file` | 现有 `operation`/`path`/`content` **+ 新增** `offset?`、`limit?` | read 自动 / write 审批 | 仅 read |
| `shell` | 不变 | `RequiresApproval` | 硬拒（现状） |
| `remember` | 不变 | `AutoApprove` | 可用（现状） |

### 3.1 `grep`

`output_mode` 三值：

- `files_with_matches`（**默认**）—— 只回文件路径列表
- `content` —— 回匹配行，配合 `context` 给前后文
- `count` —— 回每文件匹配数

默认给文件名而非内容，因为它最省 token：模型典型用法是先定位候选文件、再决定读哪个。

`glob` 参数用于限定文件集（如 `**/*.rs`），`path` 用于限定起始子目录，默认工作区根。

### 3.2 `glob`

按文件名模式匹配，返回路径列表。与 `grep` 共用同一套遍历与过滤实现。

### 3.3 `edit`

`old_string` 匹配语义（**唯一匹配优先**）：

| 匹配次数 | `replace_all` | 行为 |
|---|---|---|
| 0 | 任意 | 报错：未找到 |
| 1 | 任意 | 执行替换 |
| >1 | `false`（默认） | **报错，并回报实际匹配次数** |
| >1 | `true` | 全部替换，返回替换数 |

多匹配报错时必须告知实际次数，模型才知道该补多少上下文。默认拒绝多匹配是为了从根本上防止"改错位置"这类静默损坏 —— 这类错误一旦发生极难发现。

### 3.4 `file` read 边界

新增 `offset` / `limit` 分页；输出加行号前缀。默认边界值：

| 边界 | 默认值 |
|---|---|
| 单次返回行数 | 2000 行 |
| 单行字符数 | 2000 字符（超出截断并标记） |
| 总字节 | 64KB（与 `SHELL_OUTPUT_LIMIT` 一致） |

三者取先触发者。这些是**默认值而非硬上限** —— `limit` 参数可在总字节约束内上调。

## 4. 安全与执行边界

新工具全部对齐 `shell_tool` 已有约束，同时回补 `file_tool` 缺失的部分：

- **取消**：`grep`/`glob` 遍历大仓库可能耗时较长，复用 `shell` 的 `Arc<AtomicBool>` 取消信号。`file_tool` 目前完全未接入，一并补上。
- **输出上限**：沿用 `SHELL_OUTPUT_LIMIT`（64KB）作为统一字节上限。`grep` / `glob` 额外加默认 200 条结果上限（`grep` 可经 `head_limit` 参数在字节约束内上调），取先触发者。**截断必须显式告知** —— 静默截断会让模型误以为已搜完。
- **二进制保护**：读到 NUL 字节即判定二进制，返回明确原因，而非抛 UTF-8 解码错误。
- **路径边界**：全部走现成的 `BoundedFilesystem`。遍历时**逐条目校验**，防止 symlink 指向工作区外。
- **默认过滤**：尊重 `.gitignore` / `.ignore`，跳过隐藏目录与二进制文件。不过滤的话，本仓一次 grep 会被 `node_modules` 与 `src-tauri/target` 淹没，工具等同不可用。

## 5. 代码组织

`api_process_adapter.rs` 已 3801 行，**不往其中加逻辑**：

```
src-tauri/src/contexts/agent_runtime/infrastructure/tools/
├─ mod.rs          现有，加导出
├─ file_tool.rs    改：offset/limit、行号、上限、二进制、取消
├─ shell_tool.rs   不动
├─ edit_tool.rs    新增
├─ grep_tool.rs    新增
├─ glob_tool.rs    新增
└─ walk.rs         新增：共享受限遍历
```

`walk.rs` 是关键抽象 —— `grep` 与 `glob` 共用同一套"安全遍历"（`BoundedFilesystem` 边界 + `ignore` 过滤 + 取消信号 + 上限），边界逻辑只实现一次。

`api_process_adapter.rs` 中仅在 `execute_tool_call` 的 `match` 上新增三个分支，纯路由。

## 6. 集成点

新增工具需穿过 5 处已有机制。每一处遗漏都会产生用户可感知的错误行为：

| 位置 | 改动 | 遗漏后果 |
|---|---|---|
| `tool_catalog()` | 3 → 6 个工具 | — |
| `plan_mode_tool_catalog()` | 2 → 4 个（+`grep`/`glob`） | plan mode 无法搜索，只能盲读 |
| `risk_tier_for()` | `grep`/`glob` → `AutoApprove`；`edit` 显式 → `RequiresApproval` | `grep` 落入默认分支变成弹审批，**本变更核心价值归零** |
| `requires_approval()` 白名单 | 加入 `edit` | 已授信用户每次改动仍弹审批 |
| `execute_tool_call()` plan 硬拒 | 现只拒 `shell` 与 file write，加拒 `edit` | plan mode 下可改文件 —— **安全缺口** |

`requires_approval()` 的白名单是硬编码工具名：

```rust
if auto_approve_tools && (tool_name == SHELL_TOOL_NAME || tool_name == FILE_TOOL_NAME) {
    return false;
}
```

新工具名不在其中即无法被信任授权覆盖。这是必须同步处理的集成点，不是可选优化。

已有契约测试 `catalog_declares_exactly_shell_file_and_remember_tools` 断言 `catalog.len() == 3`，必然失败，属预期内的契约更新。

## 7. 依赖与供应链

新增两个直接依赖（当前 `Cargo.toml` 中 `regex` / `ignore` / `globset` / `walkdir` 均无）：

| Crate | 用途 | 间接引入 |
|---|---|---|
| `regex` | 正则引擎 | `regex-syntax`、`aho-corasick`、`memchr` |
| `ignore` | `.gitignore` 感知遍历 | `globset`、`walkdir`、`crossbeam-deque` 等 |

两者均属 ripgrep 生态（BurntSushi），是此类需求的事实标准。但会明显加宽间接依赖树。`software-supply-chain-security` spec 有 `Vulnerable dependency prevention` 与 `Automated dependency maintenance` 两条需求，**proposal 中须单列一节**说明引入理由与审查结论，不可夹带。

## 8. 测试策略

按项目现有习惯，每个 tool 模块自带 `#[cfg(test)] mod tests`，用 `TempDirectory` 构造真实文件树。

- **纯函数层**：`risk_tier_for` / `requires_approval` / 两个 catalog 的断言，覆盖每个新工具名
- **工具行为层**：`grep` 三种 `output_mode`；`.gitignore` 确实被跳过；`edit` 的 0/1/多匹配三分支；`file` read 的分页与行号；二进制拒绝
- **边界层**（最关键）：symlink 指向工作区外时遍历逐条目拦截；超限截断显式告知；取消信号真正中断遍历
- **plan mode 层**：`grep`/`glob` 在 plan mode 可执行；`edit` 被硬拒（即使模型主动请求）

## 9. Spec 影响面

### 9.1 已发现的既有漂移

`agent-tool-execution` 的 `Native agent tool catalog` 需求写着：

> "SHALL provide **exactly two tools** … a shell tool and a file read/write tool"

但代码实际提供 3 个（多一个 `remember`）。`add-agent-cross-session-memory` 当时更新了 `agent-chat-configuration` 与 `agent-cross-session-memory`，**遗漏了这份**。本次既然要改同一条需求，一并校正。

### 9.2 需写 delta 的 spec

| Spec | 受影响需求 |
|---|---|
| `agent-tool-execution` | Native agent tool catalog（含 9.1 漂移）、Risk-tiered tool approval、Sandboxed tool execution |
| `agent-tool-trust` | A trusted agent's shell and file-write calls skip approval → 需覆盖 `edit` |
| `agent-chat-configuration` | Plan mode restricts…（需求措辞已足够通用，补 `grep`/`glob` 的 scenario 即可） |
| `onepiece-native-agent` | Safe OnePiece tool defaults |

### 9.3 Web/mock 同步

`agent-tool-execution` 有 `Web runtime tool-use parity` 需求，且 `AGENTS.md` 要求 `tauri-agent-client.ts` 与 `web-agent-client.ts` 接口一致。须同步 `web-agent-client.ts` 的 mock 工具序列，否则契约测试失败。

## 附录 A：OnePiece 能力补强路线图

本设计是路线图第 1 项。其余子项各自独立成 change：

| # | 子项目 | 依赖 | 说明 |
|---|---|---|---|
| **1** | **工具目录补强**（本文） | 无 | 零依赖，边界清晰，为后续"加工具"类变更立模式 |
| 2 | `api_process_adapter` 拆解 + 生成崩溃恢复 | 无（宜早做） | 3801 行拆分；补普通会话中断/恢复。越晚做，后续冲突越大 |
| 3 | 任务分解 / todo 规划 | 宜在 2 之后 | 可参考 Loop 的 Worker/Verifier 角色拆分 |
| 4 | 子 Agent 树 | 依赖 3 | 复用 `multi-agent-coordination` 的 DAG 调度器 |
| 5 | 外部信息接入（联网搜索/抓取） | 依赖 1 | 本质是加工具，但涉及网络出口，安全审查需单独进行 |
| 6 | 插件注册自定义 mode | 最后 | 扩展点设计；前置能力不稳定时定扩展点必然出错 |

配置与会话体验类补充与上述六项无依赖关系，可在任意时点插入，但需先明确具体痛点才能定义范围。

### 现状核实记录

立项时对能力缺口的核实结论：

| 子能力 | 核实结果 |
|---|---|
| 任务分解(Plan Agent) | 确认无。Loop 引擎有 Worker/Verifier 双角色模式可参考 |
| 子 Agent 树 | 确认无。`multi-agent-coordination` 已有 DAG 调度器/执行器/仓储，但那是**跨已注册 Agent 的编排**，非 OnePiece 自行派发子 Agent；基础设施可复用 |
| Agent 崩溃恢复 | **部分已有**。Loop 路径有完整 pause/cancel/restart recovery；OnePiece 普通会话生成路径 `api_process_adapter.rs` 中 `interrupt`/`resume`/`crash`/`restart` 零命中 |
| 插件注册自定义 mode | 确认无。`plugin-integration-management` 仅为 GitHub CLI 一类外部集成检测目录，非可扩展 mode 注册点 |
| 上下文压缩 | **已有**。`agent-context-compaction` 已接入 `api_process_adapter`，无需重做 |
