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

## Impact

- 新增直接依赖 `regex`、`ignore`、`globset`（ripgrep 生态），会加宽间接依赖树；`software-supply-chain-security` 的 `Vulnerable dependency prevention` 与 `Automated dependency maintenance` 需求要求单列审查说明。
- 既有契约测试 `catalog_declares_exactly_shell_file_and_remember_tools` 断言 `catalog.len() == 3`，属预期内的契约更新。
- Web/mock：已核实前后端契约测试均不枚举原生工具名，mock 序列不同步不会导致测试失败；本次仅补一个 `grep` 示例以保持演示保真度。
