## Why

OnePiece 的原生工具目录只有 `shell`、`file`（整文件 read/write）、`remember`。这带来四个具体问题：

1. 没有内容/文件名搜索，找代码只能走 `shell`，而 `shell` 在 `risk_tier_for` 中固定为 `RequiresApproval` —— 每次搜索都打断用户审批。
2. `file_tool` 的 read 是裸 `read_to_string`，无大小上限、无超时、无取消，与 `shell_tool` 已有的 60s 超时 / 64KB 上限 / 取消信号严重不对称。
3. `file` 只有整文件覆写，改一行需重写整个文件。
4. Plan mode 下 `plan_mode_tool_catalog()` 仅提供 `file(read)` + `remember`，模型无法搜索，只能猜路径盲读。

## What Changes

- 新增 `grep`（内容搜索）、`glob`（文件名搜索）、`edit`（唯一匹配的串替换）三个工具。
- 为 `file` 的 read 操作增加 `offset` / `limit` 分页、行号前缀、行数/单行/字节三档上限、二进制拒绝、取消信号接入。
- `grep` / `glob` 归入 `AutoApprove`；`edit` 归入 `RequiresApproval` 并纳入信任授权白名单。
- Plan mode 目录增加 `grep` / `glob`；`execute_tool_call` 增加对 `edit` 的 plan mode 硬拒。
- 校正 `agent-tool-execution` 中「exactly two tools」的既有漂移 —— 该措辞在 `remember` 工具落地时未同步更新。

## Capabilities

### New Capabilities

（无新增 capability；全部为既有 capability 的修改）

### Modified Capabilities

- `agent-tool-execution`：工具目录、风险分级、沙箱执行边界
- `agent-tool-trust`：受信任 Agent 跳过审批的工具集需覆盖 `edit`
- `agent-chat-configuration`：plan mode 只读工具集需覆盖 `grep` / `glob`
- `onepiece-native-agent`：Safe OnePiece tool defaults
- `agent-context-compaction`：修正摘要调用「不声明工具」场景随工具目录扩张而复发的既有枚举漂移

## Dependencies

本变更新增三个直接依赖，当前 `Cargo.toml` 的 `regex` / `ignore` / `globset` / `walkdir` 均未出现：

| Crate | 为何需要 | 主要间接引入 |
|---|---|---|
| `regex` | `grep` 工具的内容匹配核心 —— 编译并执行 `pattern` 参数指定的正则表达式，逐行匹配文件内容 | `regex-syntax`、`aho-corasick`、`memchr` |
| `ignore` | `grep`/`glob` 共享的受限遍历（`walk.rs`）用它做 `.gitignore`/`.ignore` 感知的目录遍历；没有它，一次搜索会被 `node_modules`、`src-tauri/target` 等目录淹没，工具等同不可用 | `walkdir`、`crossbeam-deque` 等 |
| `globset` | `grep` 的 `glob` 参数与 `glob` 工具本身都直接构造 `GlobBuilder` 编译文件名匹配模式（如 `**/*.rs`），是代码里直接 `use` 的依赖；同时也是 `ignore` 内部使用的库，两处引用会被 Cargo 统一到同一份 | — |

三者均属 ripgrep 生态（BurntSushi 维护的同一组库；ripgrep 本身就是它们的组合），是"忽略规则感知的内容/文件名搜索"这类需求的事实标准 —— 选择复用这套已被广泛使用和审计的实现，而不是自行重写遍历、忽略规则解析或 glob 匹配逻辑。

三者的引入会加宽间接依赖树，触发 `software-supply-chain-security` 的 `Vulnerable dependency prevention` 与 `Automated dependency maintenance` 两条既有需求。这两条需求依赖的是仓库既有的自动化门禁，而非本提案的一次性人工审查：`Automated dependency maintenance` 让 Dependabot 持续扫描 `src-tauri` 的 Cargo 项目并按周开出分组更新 PR；`Vulnerable dependency prevention` 让默认分支的依赖审查在任何引入版本命中已公开漏洞时直接阻断合并。三者引入后即自动落入这两条既有门禁的持续覆盖范围，不需要为本次改动新增例外流程或单独的一次性人工安全评审。

## Impact

- 既有契约测试 `catalog_declares_exactly_shell_file_and_remember_tools` 断言 `catalog.len() == 3`，属预期内的契约更新。
- Web/mock：已核实前后端契约测试均不枚举原生工具名，mock 序列不同步不会导致测试失败；本次仅补一个 `grep` 示例以保持演示保真度。
