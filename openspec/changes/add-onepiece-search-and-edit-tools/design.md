## Context

OnePiece 是 VaneHub 内置的原生 Agent（`launch_kind = api`），当前工具目录只有 `shell`、`file`（整文件 read/write）、`remember` 三个工具（完整动机见 proposal.md 的 Why）。本文档记录 `grep`、`glob`、`edit` 三个新工具与 `file` read 分页/上限能力的技术设计：工具契约、安全与执行边界、代码组织、集成点、依赖与供应链、测试策略，并在末尾记录关键取舍。实现方式为纯 Rust 内建，不引入外部进程（如系统 `grep`/`rg`）。

## 工具契约

| 工具 | 参数 | 风险层 | Plan Mode |
|---|---|---|---|
| `grep` | `pattern`(必填, 正则)、`glob?`、`path?`、`output_mode?`、`context?`、`case_insensitive?`、`head_limit?` | `AutoApprove` | 可用 |
| `glob` | `pattern`(必填)、`path?` | `AutoApprove` | 可用 |
| `edit` | `path`、`old_string`、`new_string`(均必填)、`replace_all?`(默认 `false`) | `RequiresApproval` | 硬拒 |
| `file` | 现有 `operation`/`path`/`content` **+ 新增** `offset?`、`limit?` | read 自动 / write 审批 | 仅 read |
| `shell` | 不变 | `RequiresApproval` | 硬拒（现状） |
| `remember` | 不变 | `AutoApprove` | 可用（现状） |

### `grep`

`output_mode` 三值：

- `files_with_matches`（**默认**）—— 只回文件路径列表
- `content` —— 回匹配行，配合 `context` 给前后文
- `count` —— 回每文件匹配数

默认给文件名而非内容，因为它最省 token：模型典型用法是先定位候选文件、再决定读哪个。

`glob` 参数用于限定文件集（如 `**/*.rs`），`path` 用于限定起始子目录，默认工作区根。

### `glob`

按文件名模式匹配，返回路径列表。与 `grep` 共用同一套遍历与过滤实现。

### `edit`

`old_string` 匹配语义（**唯一匹配优先**）：

| 匹配次数 | `replace_all` | 行为 |
|---|---|---|
| 0 | 任意 | 报错：未找到 |
| 1 | 任意 | 执行替换 |
| >1 | `false`(默认) | **报错，并回报实际匹配次数** |
| >1 | `true` | 全部替换，返回替换数 |

多匹配报错时必须告知实际次数，模型才知道该补多少上下文。默认拒绝多匹配是为了从根本上防止"改错位置"这类静默损坏 —— 这类错误一旦发生极难发现。

### `file` read 边界

新增 `offset` / `limit` 分页；输出加行号前缀。默认边界值：

| 边界 | 默认值 |
|---|---|
| 单次返回行数 | 2000 行 |
| 单行字符数 | 2000 字符（超出截断并标记） |
| 总字节 | 64KB（与 `SHELL_OUTPUT_LIMIT` 一致） |

三者取先触发者，均为**硬上限**：`limit` 参数只能把返回行数调得更少，不能调高。模型无法通过传一个大 `limit` 把自己的上下文撑爆 —— 上限保护的是上下文窗口，不是可协商的偏好。需要更多内容时用 `offset` 翻页。

## 安全与执行边界

新工具全部对齐 `shell_tool` 已有约束，同时回补 `file_tool` 缺失的部分：

- **取消**：`grep` / `glob` 遍历大仓库可能耗时较长，复用 `shell` 的 `Arc<AtomicBool>` 取消信号，在遍历循环每个条目处检查。`file` 的 read **不接取消** —— 单次 `std::fs::read` 没有可插入检查点的循环，加参数只是空摆设；该路径靠下面的三档上限约束，而非靠取消。
- **输出上限**：沿用 `SHELL_OUTPUT_LIMIT`（64KB）作为统一字节上限。`grep` / `glob` 额外加 200 条结果硬上限，`head_limit` 参数只能调低不能调高，取先触发者。**截断必须显式告知** —— 静默截断会让模型误以为已搜完。
- **输入上限**：`grep` / `edit` / `file read` 在 `std::fs::read` **之前**先用 `metadata().len()` 判断文件大小，超过 10MB 即放弃。否则一个未被 `.gitignore` 排除的超大日志会在任何输出上限生效前就分配等量内存 —— 输出边界保护上下文窗口，输入边界保护进程本身。`grep` 静默跳过超大文件（搜索不该因单个文件而失败），`file read` 与 `edit` 报明确错误（用户点名了这个文件，静默跳过等于骗人）。
- **二进制保护**：读到 NUL 字节即判定二进制，返回明确原因，而非抛 UTF-8 解码错误。
- **路径边界**：全部走现成的 `BoundedFilesystem`。遍历时**逐条目校验**，防止 symlink 指向工作区外。
- **默认过滤**：尊重 `.gitignore` / `.ignore`，跳过隐藏目录与二进制文件。不过滤的话，本仓一次 grep 会被 `node_modules` 与 `src-tauri/target` 淹没，工具等同不可用。

## 代码组织

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

## 集成点

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

## 依赖与供应链

新增两个直接依赖（当前 `Cargo.toml` 中 `regex` / `ignore` / `globset` / `walkdir` 均无）：

| Crate | 用途 | 间接引入 |
|---|---|---|
| `regex` | 正则引擎 | `regex-syntax`、`aho-corasick`、`memchr` |
| `ignore` | `.gitignore` 感知遍历 | `globset`、`walkdir`、`crossbeam-deque` 等 |

两者均属 ripgrep 生态（BurntSushi），是此类需求的事实标准。但会明显加宽间接依赖树。`software-supply-chain-security` spec 有 `Vulnerable dependency prevention` 与 `Automated dependency maintenance` 两条需求，proposal 中须单列一节说明引入理由与审查结论，不可夹带。

## 测试策略

按项目现有习惯，每个 tool 模块自带 `#[cfg(test)] mod tests`，用 `TempDirectory` 构造真实文件树。

- **纯函数层**：`risk_tier_for` / `requires_approval` / 两个 catalog 的断言，覆盖每个新工具名
- **工具行为层**：`grep` 三种 `output_mode`；`.gitignore` 确实被跳过；`edit` 的 0/1/多匹配三分支；`file` read 的分页与行号；二进制拒绝
- **边界层**（最关键）：symlink 指向工作区外时遍历逐条目拦截；超限截断显式告知；取消信号真正中断遍历
- **plan mode 层**：`grep`/`glob` 在 plan mode 可执行；`edit` 被硬拒（即使模型主动请求）

## Decisions

1. **`output_mode` 默认 `files_with_matches`。** 模型典型用法是先定位候选文件、再决定读哪个；回文件路径列表比回匹配内容省得多的 token。`content` 与 `count` 仍可通过显式传参取得，默认值只决定"不问就给什么"。
   - Alternative considered：默认 `content`（类似传统 grep 行为）。拒绝 —— 多数搜索场景模型并不需要立刻看到匹配行，默认回内容会让每次搜索都多付一份通常用不上的 token 成本。

2. **`edit` 多匹配默认报错，而非改第一处。** "改错位置"是静默损坏 —— 文件表面上改动成功，但改的不是模型想改的那一处，这类错误在后续对话中极难被发现和归因。报错时必须回报实际匹配次数，让模型知道该在 `old_string` 里补多少上下文才能唯一定位，而不是让模型盲目重试。
   - Alternative considered：改第一处匹配（类似简单字符串替换工具的默认行为）。拒绝 —— 省了一次报错往返，但用可能错误的编辑结果换这点效率，不划算。

3. **遍历跳过符号链接，而非跟随后校验。** 跟随符号链接需要对每个遍历到的条目做一次 `canonicalize` 系统调用来确认解析后路径仍在工作区内，在大仓库上这是显著的额外开销。直接跳过符号链接可以在遍历层面整体消除"符号链接指向工作区外"这一类越界风险，不需要逐条目验证。代价是极少数以符号链接组织代码的仓库会搜不全，这类情况可由用户改走 `shell` 工具兜底（`shell` 不受这条遍历策略约束）。
   - Alternative considered：跟随符号链接并对解析后的路径做 `BoundedFilesystem` 校验。拒绝 —— 安全上可行，但性能代价随仓库规模线性增长，且为一个小众场景（符号链接组织的仓库）优化，不值得让每个仓库、每次搜索都多付这份 canonicalize 开销。

4. **`grep` / `edit` / `file` read 在 `std::fs::read` 之前先用 `metadata().len()` 做 10MB 前置检查，且失败行为按工具区分。** 输出上限（字节/行数/条目数上限）只在数据已经读入内存之后才生效；一个未被 `.gitignore` 排除的超大文件（例如日志）会在触达任何输出上限之前就先把等量内存分配出去 —— 输出边界保护的是上下文窗口，输入边界保护的是进程本身，两者必须都有，且输入检查必须在真正读取之前完成，否则形同虚设。`grep` 静默跳过超限文件，因为遍历式搜索本就会跳过大量不相关文件，多跳过一个不改变模型对"搜索"这个操作的预期；`file` read 与 `edit` 报明确错误，因为用户/模型是点名要这一个文件，静默跳过等同于给出一个看似成功、实则未读到内容的假结果。
   - Alternative considered：只设输出上限，不做前置的输入大小检查。拒绝 —— 输出上限对已经分配出去的内存无能为力，一次性 `fs::read` 一个超大文件会在截断逻辑运行之前就已经付出完整的内存和 IO 代价。

5. **`limit` / `head_limit` 是硬上限，只能调低、不能调高。** 这两个参数保护的是模型自己的上下文窗口，而不是一个可与模型协商的偏好；如果模型可以传一个更大的值把默认上限往上抬，上限就不再是上限，等于形同虚设。需要更多内容时，正确的方式是用 `offset`（`file` read 翻页）或收窄查询范围（更具体的 `pattern`/`glob`/`path`），而不是加大单次返回量。
   - Alternative considered：允许 `limit` 在系统硬顶（如 64KB / 200 条）以下自由调节，包括调高默认值。拒绝 —— 会重新打开"模型不小心把自己上下文撑爆"这个 `file_tool` 边界补强本身要修的口子。
