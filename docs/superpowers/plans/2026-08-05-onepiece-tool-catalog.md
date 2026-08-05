# OnePiece 工具目录补强 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 为 OnePiece 原生 Agent 增加 `grep` / `glob` / `edit` 三个工具，并为既有 `file` read 补上分页与执行边界。

**Architecture:** 三个新工具各自一个模块，放在 `src-tauri/src/contexts/agent_runtime/infrastructure/tools/` 下；`grep` 与 `glob` 共用新增的 `walk.rs`（工作区受限 + `.gitignore` 感知 + 可取消 + 有上限的单遍遍历）。已达 3801 行的 `api_process_adapter.rs` 只增加纯路由分支，不承载逻辑。工具目录、风险分级、信任白名单、plan mode 硬拒四处接线分两步落地：先落不可见管线，再单独一次提交"上线"目录。

**Tech Stack:** Rust 2021 / Tauri 2.x；新增 crate `regex`、`ignore`、`globset`；测试用项目既有的 `TempDirectory` 助手 + `#[cfg(test)] mod tests` 内联单测。

## Global Constraints

- 设计依据：`docs/superpowers/specs/2026-08-05-onepiece-tool-catalog-design.md`。与本计划冲突时以设计文档为准。
- 变更流程：动代码前必须先完成 Task 1 的 OpenSpec proposal，并通过 `openspec validate <change-id> --strict`（`AGENTS.md`「变更流程」）。
- Rust 错误处理：跨 Tauri command 边界的错误转 `Result<T, String>` 或自定义 error enum；`unwrap()` / `expect()` 仅限测试代码（`AGENTS.md`）。
- 注释只写「为什么这样做」，不写代码翻译式注释（`AGENTS.md`）。
- 统一输出字节上限沿用既有 `SHELL_OUTPUT_LIMIT = 64 * 1024`。
- `grep` / `glob` 结果条数上限 200；`file` read 2000 行、单行 2000 字符。三档取先触发者，**截断必须在输出中显式告知**。以上均为**硬上限** —— `limit` / `head_limit` 参数只能调低，不能调高。
- 单文件大小上限 10MB，在 `std::fs::read` **之前**用 `metadata().len()` 检查。`grep` 静默跳过超限文件；`file read` 与 `edit` 报明确错误。
- 新工具风险分级：`grep` / `glob` → `AutoApprove`；`edit` → `RequiresApproval`。
- Plan mode：`grep` / `glob` 可用；`edit` 硬拒。
- 提交前必须通过：`npm run lint`、`npm run test`、`cargo clippy --manifest-path src-tauri/Cargo.toml`（`AGENTS.md`）。
- **`cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check` 必须通过。** `.github/workflows/ci.yml:212` 在每个 PR 上跑它，且它是 Rust 步骤里的第一个 —— 不过它，后面的测试根本不会执行。本计划中的示例代码是按可读性排版的，不保证符合 rustfmt；**每个 Rust 任务提交前先跑一次 `cargo fmt --all`**。
- CI 的 clippy 是 `--all-targets -- -D warnings`（`ci.yml:218`），警告即错误。Task 2-5 期间会有若干 `dead_code` 警告，因为常量与函数要到后续任务才被消费 —— 这是增量落地的正常产物，到 Task 6 自然消解。**不要**用 `#![allow(dead_code)]` 掩盖（它会活过 Task 6 并永久遮蔽真正的死代码）。代价是：本分支在 Task 6 完成前不可开 PR。
- 符号链接一律跳过，不跟随（见 Task 2 说明）。符号链接测试仅在 `#[cfg(unix)]` 下运行 —— Windows 创建符号链接需要开发者模式或管理员权限。

---

### Task 1: OpenSpec proposal 起草

**Files:**
- Create: `openspec/changes/add-onepiece-search-and-edit-tools/.openspec.yaml`
- Create: `openspec/changes/add-onepiece-search-and-edit-tools/proposal.md`
- Create: `openspec/changes/add-onepiece-search-and-edit-tools/design.md`
- Create: `openspec/changes/add-onepiece-search-and-edit-tools/tasks.md`
- Create: `openspec/changes/add-onepiece-search-and-edit-tools/specs/agent-tool-execution/spec.md`
- Create: `openspec/changes/add-onepiece-search-and-edit-tools/specs/agent-tool-trust/spec.md`
- Create: `openspec/changes/add-onepiece-search-and-edit-tools/specs/agent-chat-configuration/spec.md`
- Create: `openspec/changes/add-onepiece-search-and-edit-tools/specs/onepiece-native-agent/spec.md`

**Interfaces:**
- Consumes: `docs/superpowers/specs/2026-08-05-onepiece-tool-catalog-design.md` 全文
- Produces: change-id `add-onepiece-search-and-edit-tools`，后续所有任务的 tasks.md 回填目标

- [ ] **Step 1: 建目录与 `.openspec.yaml`**

```bash
mkdir -p openspec/changes/add-onepiece-search-and-edit-tools/specs/agent-tool-execution
mkdir -p openspec/changes/add-onepiece-search-and-edit-tools/specs/agent-tool-trust
mkdir -p openspec/changes/add-onepiece-search-and-edit-tools/specs/agent-chat-configuration
mkdir -p openspec/changes/add-onepiece-search-and-edit-tools/specs/onepiece-native-agent
```

`.openspec.yaml`（`created` 用当天日期）：

```yaml
schema: spec-driven
created: 2026-08-05
```

- [ ] **Step 2: 写 `proposal.md`**

结构照搬 `openspec/changes/add-gemini-cli-terminal-usage-tracking/proposal.md`（`## Why` / `## What Changes` / `## Capabilities` 含 `### New Capabilities` `### Modified Capabilities` / `## Impact`）。

```markdown
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
```

- [ ] **Step 3: 写 `design.md`**

把 `docs/superpowers/specs/2026-08-05-onepiece-tool-catalog-design.md` 的第 3–8 节（工具契约、安全边界、代码组织、集成点、依赖供应链、测试策略）迁入，并补一节 Decisions 记录三个取舍及其理由：

1. `output_mode` 默认 `files_with_matches` —— 模型典型用法是先定位候选、再决定读哪个，回文件名最省 token。
2. `edit` 多匹配默认报错而非改第一处 —— 「改错位置」是静默损坏，事后极难发现；报错时回报实际匹配次数，让模型知道该补多少上下文。
3. 遍历跳过符号链接而非跟随后校验 —— 跟随符号链接需对每个条目做 canonicalize 系统调用，在大仓库上代价显著；直接跳过可消除整类越界风险，代价是极少数以符号链接组织的仓库搜不全，可由用户改走 `shell` 兜底。

- [ ] **Step 4: 写四份 spec delta**

每份用 OpenSpec delta 语法（`## MODIFIED Requirements` + 完整需求体 + 场景）。四份的目标需求：

| 文件 | 需求 |
|---|---|
| `specs/agent-tool-execution/spec.md` | `Native agent tool catalog`、`Risk-tiered tool approval`、`Sandboxed tool execution` |
| `specs/agent-tool-trust/spec.md` | `A trusted agent's shell and file-write calls skip approval` |
| `specs/agent-chat-configuration/spec.md` | `Plan mode restricts a native API agent to read-only tools` |
| `specs/onepiece-native-agent/spec.md` | `Safe OnePiece tool defaults` |

`agent-tool-execution` 的 `Native agent tool catalog` 需求体改为（注意同时修掉「exactly two」漂移）：

```markdown
## MODIFIED Requirements

### Requirement: Native agent tool catalog
The system SHALL provide a fixed, provider-agnostic tool catalog to a native API-based agent's generation, comprising a shell/command-execution tool, a file read/write tool, a content-search tool, a filename-search tool, a scoped file-edit tool, and a cross-session memory tool. Each tool SHALL be defined once and translated into the request shape required by the session's `interface_format`.

#### Scenario: Tools included in every native generation request
- **WHEN** a chat generation starts for an agent with `launch_kind = api`
- **THEN** the outgoing provider request SHALL declare the shell, file, content-search, filename-search, file-edit, and memory tools

#### Scenario: Tool definitions translated per interface format
- **WHEN** the session's `interface_format` is `anthropic`
- **THEN** each tool SHALL be declared using Anthropic's `{name, description, input_schema}` shape
- **WHEN** the session's `interface_format` is `openai-compatible`
- **THEN** each tool SHALL be declared using OpenAI's `{type: "function", function: {name, description, parameters}}` shape
```

`agent-tool-execution` 的另两条需求 —— `Risk-tiered tool approval` 需在既有「file-read 免审批 / file-write 与 shell 必审批」之上加入「内容搜索与文件名搜索免审批；文件编辑必审批」；`Sandboxed tool execution` 需加入「搜索遍历 SHALL 尊重仓库忽略规则、跳过符号链接与二进制文件、可被取消，且结果 SHALL 有显式声明的上限」。

`specs/agent-tool-trust/spec.md`（注意需求标题本身要改 —— 它把工具集写进了标题）：

```markdown
## MODIFIED Requirements

### Requirement: A trusted agent's shell, file-write, and edit calls skip approval
The system SHALL execute a trusted native API agent's `shell` calls, file tool `write` operations, and file-edit calls immediately, without prompting for approval, while leaving every other tool's approval behavior unchanged.

#### Scenario: Trusted agent runs a shell command without a prompt
- **WHEN** a native API agent with the trust setting enabled requests a shell tool call
- **THEN** the system SHALL execute it immediately without an approval prompt

#### Scenario: Trusted agent writes a file without a prompt
- **WHEN** a native API agent with the trust setting enabled requests a file tool call with a write operation
- **THEN** the system SHALL execute it immediately without an approval prompt

#### Scenario: Trusted agent edits a file without a prompt
- **WHEN** a native API agent with the trust setting enabled requests a file-edit tool call
- **THEN** the system SHALL execute it immediately without an approval prompt

#### Scenario: Untrusted agent is unaffected
- **WHEN** a native API agent without the trust setting enabled requests a shell call, a file write, or a file edit
- **THEN** the system SHALL require approval exactly as it did before this capability existed
```

`specs/agent-chat-configuration/spec.md`：

```markdown
## MODIFIED Requirements

### Requirement: Plan mode restricts a native API agent to read-only tools
The system SHALL, when the session's permission mode is plan mode, offer a native API agent only tools that cannot modify the user's system or call an arbitrary external server, and SHALL reject any attempt to use a tool or tool operation outside that restricted set regardless of what the model requests.

#### Scenario: Plan mode excludes shell, edit, and MCP-sourced tools from the catalog
- **WHEN** a generation starts in plan mode
- **THEN** the tool catalog offered to the model SHALL NOT include the shell tool, the file-edit tool, or any MCP-sourced tool

#### Scenario: Plan mode narrows the file tool to read-only
- **WHEN** a generation starts in plan mode
- **THEN** the tool catalog offered to the model SHALL only allow the file tool's read operation, not its write operation

#### Scenario: Plan mode retains read-only search tools
- **WHEN** a generation starts in plan mode
- **THEN** the tool catalog offered to the model SHALL include the content-search and filename-search tools

#### Scenario: Plan mode retains the remember tool
- **WHEN** a generation starts in plan mode
- **THEN** the tool catalog offered to the model SHALL still include the remember tool

#### Scenario: A disallowed tool call is rejected even if requested
- **WHEN** the model requests the shell tool, the file-edit tool, an MCP-sourced tool, or a file write operation while the session is in plan mode
- **THEN** the system SHALL reject the call as an error outcome without executing it, regardless of whether the tool appeared in the offered catalog

#### Scenario: Non-plan modes are unaffected
- **WHEN** a generation starts with a permission mode other than plan mode
- **THEN** the tool catalog and tool execution behavior SHALL be exactly what they were before this capability existed
```

`specs/onepiece-native-agent/spec.md`：

```markdown
## MODIFIED Requirements

### Requirement: Safe OnePiece tool defaults
The system SHALL initialize and reset OnePiece with automatic shell, file-write, and file-edit approval disabled and SHALL continue applying the existing MCP approval and plan-mode restrictions. Read-only content-search and filename-search calls SHALL NOT require approval.

#### Scenario: First configuration retains approval prompts
- **WHEN** OnePiece is configured for the first time
- **THEN** shell, file-write, and file-edit calls SHALL require approval until the user explicitly enables the existing trust setting

#### Scenario: Read-only search does not prompt
- **WHEN** OnePiece requests a content-search or filename-search tool call
- **THEN** the system SHALL execute it without an approval prompt regardless of the trust setting

#### Scenario: Trust does not bypass existing hard gates
- **WHEN** a trusted OnePiece requests an MCP tool or runs in plan mode
- **THEN** the existing MCP approval and plan-mode restrictions SHALL remain in force
```

- [ ] **Step 5: 写 `tasks.md` 骨架**

按本计划的 Task 2–10 建立编号分组，全部留 `- [ ]`，最后加一节 `## N. Verification`。后续每个任务完成后回填实际结论（照 `add-gemini-cli-terminal-usage-tracking/tasks.md` 的详尽风格 —— 写清实际做了什么、测试数量、为何某项未做）。

- [ ] **Step 6: 校验**

Run: `openspec validate add-onepiece-search-and-edit-tools --strict`
Expected: `valid`

Run: `openspec validate --specs --strict`
Expected: 全部 passed，0 failed

- [ ] **Step 7: Commit**

```bash
git add openspec/changes/add-onepiece-search-and-edit-tools
git commit -m "spec: propose OnePiece search and edit tools"
```

---

### Task 2: 依赖引入与 `walk.rs` 共享受限遍历

**Files:**
- Modify: `src-tauri/Cargo.toml`（`[dependencies]` 段）
- Create: `src-tauri/src/contexts/agent_runtime/infrastructure/tools/walk.rs`
- Modify: `src-tauri/src/contexts/agent_runtime/infrastructure/tools/mod.rs`

**Interfaces:**
- Consumes: `crate::platform::filesystem::BoundedFilesystem`（`new`、`validate_relative`、`resolve_existing`）
- Produces:
  - `pub(crate) const MAX_SEARCH_RESULTS: usize = 200;` 与 `pub(crate) const MAX_TOOL_OUTPUT_BYTES: usize = 64 * 1024;` —— **放在 `tools/mod.rs` 而非 `walk.rs`**。它们是工具的输出预算，与「怎么遍历文件」无关；更要紧的是 `shell_tool.rs` 已有一个同值的 `SHELL_OUTPUT_LIMIT = 64 * 1024`，同一模块里两个名字指同一个 64KB，改一个另一个会静默走偏。`shell_tool.rs` 改用 `MAX_TOOL_OUTPUT_BYTES`，删掉 `SHELL_OUTPUT_LIMIT`。后续任务从 `super::` 而非 `super::walk::` 导入这两个常量。
  - `pub(crate) enum Visit { Continue, Stop }`
  - `pub(crate) fn visit_workspace_files(workspace_folder: &str, relative_root: Option<&str>, cancelled: &AtomicBool, visit: &mut dyn FnMut(&Path, &str) -> Visit) -> Result<(), String>`
    —— 回调参数为 (绝对路径, 工作区相对路径，正斜杠分隔)。**取 `&str` 而非 `&BoundedFilesystem`**：工作区根需要一个绝对路径起点，而 `BoundedFilesystem` 的 `root` 字段是私有的，唯一的取法是 `resolve_existing(".")` —— 那要求 `validate_relative` 接受 `Component::CurDir`，是个未经验证的假设，赌错就是每次搜索必失败。直接 canonicalize 传入的 `workspace_folder` 没有这个风险，边界检查仍由内部构造的 boundary 负责。
  - `pub(crate) fn is_binary(bytes: &[u8]) -> bool`
  - `pub(crate) const MAX_FILE_BYTES: u64 = 10 * 1024 * 1024;`
  - `pub(crate) fn exceeds_size_limit(path: &Path) -> bool`

- [ ] **Step 1: 加依赖**

在 `src-tauri/Cargo.toml` 的 `[dependencies]` 中按字母序插入：

```toml
globset = "0.4"
ignore = "0.4"
regex = "1"
```

- [ ] **Step 2: 确认依赖可解析**

Run: `cargo check --manifest-path src-tauri/Cargo.toml`
Expected: 编译通过（此时尚无代码使用这三个 crate，可能出现 unused dependency 提示，属正常）

- [ ] **Step 3: 写失败的测试**

新建 `src-tauri/src/contexts/agent_runtime/infrastructure/tools/walk.rs`，先只写测试模块：

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TempDirectory;

    fn not_cancelled() -> Arc<AtomicBool> {
        Arc::new(AtomicBool::new(false))
    }

    fn collect(directory: &TempDirectory, root: Option<&str>) -> Vec<String> {
        let folder = directory.path().to_string_lossy().to_string();
        let mut seen = Vec::new();
        visit_workspace_files(&folder, root, &not_cancelled(), &mut |_absolute, relative| {
            seen.push(relative.to_string());
            Visit::Continue
        })
        .expect("walk succeeds");
        seen.sort();
        seen
    }

    #[test]
    fn visits_plain_files_under_the_workspace() {
        let directory = TempDirectory::new("walk-plain");
        std::fs::write(directory.path().join("a.txt"), "a").expect("write a");
        std::fs::create_dir(directory.path().join("sub")).expect("mkdir sub");
        std::fs::write(directory.path().join("sub/b.txt"), "b").expect("write b");
        assert_eq!(collect(&directory, None), vec!["a.txt", "sub/b.txt"]);
    }

    #[test]
    fn skips_gitignored_paths_even_outside_a_git_repository() {
        let directory = TempDirectory::new("walk-gitignore");
        std::fs::write(directory.path().join(".gitignore"), "ignored.txt\nnode_modules/\n")
            .expect("write gitignore");
        std::fs::write(directory.path().join("kept.txt"), "keep").expect("write kept");
        std::fs::write(directory.path().join("ignored.txt"), "drop").expect("write ignored");
        std::fs::create_dir(directory.path().join("node_modules")).expect("mkdir node_modules");
        std::fs::write(directory.path().join("node_modules/pkg.js"), "drop").expect("write pkg");
        let seen = collect(&directory, None);
        assert!(seen.contains(&"kept.txt".to_string()));
        assert!(!seen.iter().any(|path| path.contains("ignored.txt")));
        assert!(!seen.iter().any(|path| path.contains("node_modules")));
    }

    #[test]
    fn skips_hidden_entries() {
        let directory = TempDirectory::new("walk-hidden");
        std::fs::write(directory.path().join("visible.txt"), "v").expect("write visible");
        std::fs::create_dir(directory.path().join(".secret")).expect("mkdir .secret");
        std::fs::write(directory.path().join(".secret/key.txt"), "k").expect("write key");
        let seen = collect(&directory, None);
        assert_eq!(seen, vec!["visible.txt"]);
    }

    #[test]
    fn a_relative_root_narrows_the_walk() {
        let directory = TempDirectory::new("walk-root");
        std::fs::write(directory.path().join("top.txt"), "t").expect("write top");
        std::fs::create_dir(directory.path().join("sub")).expect("mkdir sub");
        std::fs::write(directory.path().join("sub/inner.txt"), "i").expect("write inner");
        assert_eq!(collect(&directory, Some("sub")), vec!["sub/inner.txt"]);
    }

    #[test]
    fn a_relative_root_that_escapes_the_workspace_is_rejected() {
        let directory = TempDirectory::new("walk-escape-root");
        let outcome = visit_workspace_files(
            &directory.path().to_string_lossy(),
            Some("../"),
            &not_cancelled(),
            &mut |_absolute, _relative| Visit::Continue,
        );
        assert!(outcome.is_err());
    }

    #[test]
    fn a_cancelled_walk_stops_and_reports_an_error() {
        let directory = TempDirectory::new("walk-cancel");
        std::fs::write(directory.path().join("a.txt"), "a").expect("write a");
        let cancelled = Arc::new(AtomicBool::new(true));
        let outcome = visit_workspace_files(
            &directory.path().to_string_lossy(),
            None,
            &cancelled,
            &mut |_absolute, _relative| Visit::Continue,
        );
        assert!(outcome.is_err());
    }

    #[test]
    fn a_missing_workspace_folder_is_reported_as_an_error() {
        let outcome = visit_workspace_files(
            "Z:/definitely/does/not/exist",
            None,
            &not_cancelled(),
            &mut |_absolute, _relative| Visit::Continue,
        );
        assert!(outcome.is_err());
    }

    #[test]
    fn a_visitor_returning_stop_ends_the_walk_early() {
        let directory = TempDirectory::new("walk-stop");
        for index in 0..10 {
            std::fs::write(directory.path().join(format!("f{index}.txt")), "x")
                .expect("write fixture");
        }
        let mut count = 0usize;
        visit_workspace_files(
            &directory.path().to_string_lossy(),
            None,
            &not_cancelled(),
            &mut |_absolute, _relative| {
                count += 1;
                Visit::Stop
            },
        )
        .expect("walk succeeds");
        assert_eq!(count, 1);
    }

    // 符号链接在 Windows 上需要开发者模式或管理员权限才能创建，故仅在 unix 下验证。
    // 跳过符号链接的逻辑本身与平台无关。
    #[cfg(unix)]
    #[test]
    fn a_symlink_pointing_outside_the_workspace_is_not_visited() {
        let outside = TempDirectory::new("walk-symlink-outside");
        std::fs::write(outside.path().join("secret.txt"), "secret").expect("write secret");
        let directory = TempDirectory::new("walk-symlink");
        std::fs::write(directory.path().join("normal.txt"), "n").expect("write normal");
        std::os::unix::fs::symlink(
            outside.path().join("secret.txt"),
            directory.path().join("leak.txt"),
        )
        .expect("create symlink");
        assert_eq!(collect(&directory, None), vec!["normal.txt"]);
    }

    #[test]
    fn binary_content_is_detected_by_a_nul_byte() {
        assert!(is_binary(b"abc\0def"));
        assert!(!is_binary(b"plain text"));
        assert!(!is_binary(b""));
    }

    #[test]
    fn a_small_file_is_within_the_size_limit() {
        let directory = TempDirectory::new("walk-size-small");
        let path = directory.path().join("small.txt");
        std::fs::write(&path, "tiny").expect("write fixture");
        assert!(!exceeds_size_limit(&path));
    }

    #[test]
    fn a_file_over_the_size_limit_is_rejected() {
        let directory = TempDirectory::new("walk-size-large");
        let path = directory.path().join("large.bin");
        std::fs::write(&path, vec![b'x'; (MAX_FILE_BYTES + 1) as usize]).expect("write fixture");
        assert!(exceeds_size_limit(&path));
    }

    #[test]
    fn an_unreadable_path_is_treated_as_over_the_limit() {
        // 拿不到 metadata 时保守判定为超限：调用方会跳过或报错，而不是继续去 read 一个
        // 大小未知的文件。
        assert!(exceeds_size_limit(Path::new("Z:/definitely/does/not/exist")));
    }
}
```

- [ ] **Step 4: 运行测试确认失败**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --lib tools::walk`
Expected: 编译失败 —— `cannot find function visit_workspace_files` / `cannot find type Visit`

- [ ] **Step 5: 实现 `walk.rs`**

在测试模块之上写入：

```rust
//! `grep` 与 `glob` 共用的工作区受限遍历。边界（路径越界、符号链接、取消、上限）只在这里
//! 实现一次，两个工具都不重复处理。

use crate::platform::filesystem::BoundedFilesystem;
use ignore::WalkBuilder;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// 单文件读取上限。输出上限保护的是模型的上下文窗口，这一条保护的是进程本身 —— 没有它，
/// 一个未被 `.gitignore` 排除的大日志会在任何输出截断生效前就先分配等量内存。
pub(crate) const MAX_FILE_BYTES: u64 = 10 * 1024 * 1024;

/// 判定二进制的嗅探窗口。整文件扫描对大文件不划算，而文本文件的 NUL 字节几乎总在开头出现。
const BINARY_SNIFF_BYTES: usize = 8 * 1024;

/// 访问者对每个文件的处置：继续遍历，或提前终止（用于结果条数/字节到顶）。
pub(crate) enum Visit {
    Continue,
    Stop,
}

/// 遍历 `boundary` 根下（可选 `relative_root` 子目录）的常规文件，对每个文件调用 `visit`。
///
/// 符号链接一律跳过而非跟随后校验：跟随需要对每个条目做 canonicalize 系统调用，大仓库上代价
/// 显著；直接跳过可消除整类越界读取（例如仓库内指向 `~/.ssh/` 的链接）。
///
/// `require_git(false)` 是刻意的 —— 工作区未必是 git 仓库，但其中的 `.gitignore` 依然表达了
/// 「这些内容不值得看」，默认的 `require_git(true)` 会让非仓库工作区退化成搜索全部内容。
pub(crate) fn visit_workspace_files(
    workspace_folder: &str,
    relative_root: Option<&str>,
    cancelled: &AtomicBool,
    visit: &mut dyn FnMut(&Path, &str) -> Visit,
) -> Result<(), String> {
    let boundary = BoundedFilesystem::new(Path::new(workspace_folder))
        .map_err(|error| format!("Workspace folder is unavailable: {error}"))?;
    // 直接 canonicalize 而不走 `boundary.resolve_existing(".")` —— 后者依赖
    // `validate_relative` 接受 `Component::CurDir`，那是个未经验证的假设。
    let workspace_root = Path::new(workspace_folder)
        .canonicalize()
        .map_err(|error| format!("Workspace folder is unavailable: {error}"))?;
    let root = match relative_root.map(str::trim).filter(|root| !root.is_empty()) {
        Some(relative) => boundary
            .resolve_existing(relative)
            .map_err(|error| format!("Path \"{relative}\" is not accessible: {error}"))?,
        None => workspace_root.clone(),
    };

    let walker = WalkBuilder::new(&root)
        .hidden(true)
        .git_ignore(true)
        .git_global(true)
        .git_exclude(true)
        .ignore(true)
        .parents(true)
        .require_git(false)
        .follow_links(false)
        .build();

    for entry in walker {
        if cancelled.load(Ordering::Relaxed) {
            return Err("Search was cancelled.".to_string());
        }
        let Ok(entry) = entry else {
            // 单个条目不可读（权限、竞态删除）不应让整次搜索失败。
            continue;
        };
        let Some(file_type) = entry.file_type() else {
            continue;
        };
        if !file_type.is_file() || file_type.is_symlink() {
            continue;
        }
        let absolute = entry.path();
        let Ok(relative) = absolute.strip_prefix(&workspace_root) else {
            continue;
        };
        let display = relative.to_string_lossy().replace('\\', "/");
        if let Visit::Stop = visit(absolute, &display) {
            return Ok(());
        }
    }
    Ok(())
}

/// 二进制判定：嗅探窗口内出现 NUL 字节即认定为二进制。比向模型抛一个 UTF-8 解码错误
/// 更可解释 —— 模型据此知道该换一个文件，而不是重试同一个。
pub(crate) fn is_binary(bytes: &[u8]) -> bool {
    bytes
        .iter()
        .take(BINARY_SNIFF_BYTES)
        .any(|byte| *byte == 0)
}

/// 在 `std::fs::read` 之前判断文件是否超过 `MAX_FILE_BYTES`。拿不到 metadata 时返回 `true`
/// —— 失败方向选择「不读」而非「读一个大小未知的文件」。
pub(crate) fn exceeds_size_limit(path: &Path) -> bool {
    match std::fs::metadata(path) {
        Ok(metadata) => metadata.len() > MAX_FILE_BYTES,
        Err(_) => true,
    }
}
```

- [ ] **Step 6: 注册模块**

`tools/mod.rs` 中加入：

```rust
mod walk;
```

- [ ] **Step 7: 运行测试确认通过**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --lib tools::walk`
Expected: unix 上 13 passed，Windows 上 12 passed（符号链接测试被 `#[cfg(unix)]` 排除）

- [ ] **Step 8: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/Cargo.lock \
        src-tauri/src/contexts/agent_runtime/infrastructure/tools/walk.rs \
        src-tauri/src/contexts/agent_runtime/infrastructure/tools/mod.rs
git commit -m "feat(agent-runtime): add workspace-bounded file walk for search tools"
```

---

### Task 3: `glob` 工具

**Files:**
- Create: `src-tauri/src/contexts/agent_runtime/infrastructure/tools/glob_tool.rs`
- Modify: `src-tauri/src/contexts/agent_runtime/infrastructure/tools/mod.rs`

**Interfaces:**
- Consumes: `walk::{visit_workspace_files, Visit}`、`super::{ToolExecutionOutcome, MAX_SEARCH_RESULTS}`
- Produces: `pub(crate) fn execute_glob(pattern: &str, path: Option<&str>, workspace_folder: &str, cancelled: Arc<AtomicBool>) -> ToolExecutionOutcome`

- [ ] **Step 1: 写失败的测试**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TempDirectory;

    fn not_cancelled() -> Arc<AtomicBool> {
        Arc::new(AtomicBool::new(false))
    }

    fn workspace(name: &str) -> TempDirectory {
        let directory = TempDirectory::new(name);
        std::fs::create_dir(directory.path().join("src")).expect("mkdir src");
        std::fs::write(directory.path().join("src/main.rs"), "fn main() {}").expect("write main");
        std::fs::write(directory.path().join("src/lib.rs"), "pub fn go() {}").expect("write lib");
        std::fs::write(directory.path().join("README.md"), "# hi").expect("write readme");
        directory
    }

    #[test]
    fn matches_files_by_extension() {
        let directory = workspace("glob-extension");
        let outcome = execute_glob(
            "**/*.rs",
            None,
            &directory.path().to_string_lossy(),
            not_cancelled(),
        );
        assert!(!outcome.is_error);
        assert!(outcome.output.contains("src/main.rs"));
        assert!(outcome.output.contains("src/lib.rs"));
        assert!(!outcome.output.contains("README.md"));
    }

    #[test]
    fn reports_no_matches_without_an_error() {
        let directory = workspace("glob-none");
        let outcome = execute_glob(
            "**/*.py",
            None,
            &directory.path().to_string_lossy(),
            not_cancelled(),
        );
        assert!(!outcome.is_error);
        assert!(outcome.output.contains("No files matched"));
    }

    #[test]
    fn an_invalid_pattern_is_reported_as_an_error() {
        let directory = workspace("glob-invalid");
        let outcome = execute_glob(
            "[unclosed",
            None,
            &directory.path().to_string_lossy(),
            not_cancelled(),
        );
        assert!(outcome.is_error);
        assert!(outcome.output.contains("Invalid glob pattern"));
    }

    #[test]
    fn an_empty_pattern_is_rejected() {
        let directory = workspace("glob-empty");
        let outcome = execute_glob(
            "   ",
            None,
            &directory.path().to_string_lossy(),
            not_cancelled(),
        );
        assert!(outcome.is_error);
    }

    #[test]
    fn a_path_scope_narrows_the_search() {
        let directory = workspace("glob-scope");
        std::fs::create_dir(directory.path().join("docs")).expect("mkdir docs");
        std::fs::write(directory.path().join("docs/guide.md"), "g").expect("write guide");
        let outcome = execute_glob(
            "**/*.md",
            Some("docs"),
            &directory.path().to_string_lossy(),
            not_cancelled(),
        );
        assert!(!outcome.is_error);
        assert!(outcome.output.contains("docs/guide.md"));
        assert!(!outcome.output.contains("README.md"));
    }

    #[test]
    fn exceeding_the_result_limit_reports_truncation() {
        let directory = TempDirectory::new("glob-truncate");
        for index in 0..(MAX_SEARCH_RESULTS + 10) {
            std::fs::write(directory.path().join(format!("f{index}.txt")), "x")
                .expect("write fixture");
        }
        let outcome = execute_glob(
            "**/*.txt",
            None,
            &directory.path().to_string_lossy(),
            not_cancelled(),
        );
        assert!(!outcome.is_error);
        assert!(outcome.output.contains("truncated"));
    }

    #[test]
    fn a_missing_workspace_folder_is_reported_as_an_error() {
        let outcome = execute_glob("**/*", None, "Z:/definitely/does/not/exist", not_cancelled());
        assert!(outcome.is_error);
    }
}
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --lib tools::glob_tool`
Expected: 编译失败 —— `cannot find function execute_glob`

- [ ] **Step 3: 实现**

```rust
//! 按文件名模式搜索工作区。遍历、过滤与边界全部委托给 `walk`，本模块只负责模式匹配与
//! 输出成形。

use super::walk::{visit_workspace_files, Visit};
use super::{ToolExecutionOutcome, MAX_SEARCH_RESULTS};
use globset::GlobBuilder;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

pub(crate) fn execute_glob(
    pattern: &str,
    path: Option<&str>,
    workspace_folder: &str,
    cancelled: Arc<AtomicBool>,
) -> ToolExecutionOutcome {
    let pattern = pattern.trim();
    if pattern.is_empty() {
        return ToolExecutionOutcome {
            output: "No glob pattern was provided.".to_string(),
            is_error: true,
        };
    }
    // `literal_separator(true)` 让 `*` 不跨目录分隔符，`**` 才跨 —— 与用户对 glob 的通常预期一致。
    let matcher = match GlobBuilder::new(pattern)
        .literal_separator(true)
        .build()
        .map(|glob| glob.compile_matcher())
    {
        Ok(matcher) => matcher,
        Err(error) => {
            return ToolExecutionOutcome {
                output: format!("Invalid glob pattern \"{pattern}\": {error}"),
                is_error: true,
            }
        }
    };

    let mut matches: Vec<String> = Vec::new();
    let mut truncated = false;
    let outcome = visit_workspace_files(workspace_folder, path, &cancelled, &mut |_absolute, relative| {
        if matcher.is_match(relative) {
            matches.push(relative.to_string());
            if matches.len() >= MAX_SEARCH_RESULTS {
                truncated = true;
                return Visit::Stop;
            }
        }
        Visit::Continue
    });
    if let Err(error) = outcome {
        return ToolExecutionOutcome {
            output: error,
            is_error: true,
        };
    }
    if matches.is_empty() {
        return ToolExecutionOutcome {
            output: format!("No files matched \"{pattern}\"."),
            is_error: false,
        };
    }
    matches.sort();
    let mut output = matches.join("\n");
    if truncated {
        output.push_str(&format!(
            "\n\n[Results truncated at {MAX_SEARCH_RESULTS} files. Narrow the pattern or use the path argument.]"
        ));
    }
    ToolExecutionOutcome {
        output,
        is_error: false,
    }
}
```

- [ ] **Step 4: 注册模块**

`tools/mod.rs`：

```rust
mod glob_tool;

pub(crate) use glob_tool::execute_glob;
```

- [ ] **Step 5: 运行测试确认通过**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --lib tools::glob_tool`
Expected: 7 passed

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/contexts/agent_runtime/infrastructure/tools/glob_tool.rs \
        src-tauri/src/contexts/agent_runtime/infrastructure/tools/mod.rs
git commit -m "feat(agent-runtime): add glob filename search tool"
```

---

### Task 4: `grep` 工具

**Files:**
- Create: `src-tauri/src/contexts/agent_runtime/infrastructure/tools/grep_tool.rs`
- Modify: `src-tauri/src/contexts/agent_runtime/infrastructure/tools/mod.rs`

**Interfaces:**
- Consumes: `walk::{visit_workspace_files, is_binary, exceeds_size_limit, Visit}`、`super::{MAX_SEARCH_RESULTS, MAX_TOOL_OUTPUT_BYTES}`
- Produces: `pub(crate) fn execute_grep(request: GrepRequest<'_>, workspace_folder: &str, cancelled: Arc<AtomicBool>) -> ToolExecutionOutcome`，以及
  ```rust
  pub(crate) struct GrepRequest<'a> {
      pub(crate) pattern: &'a str,
      pub(crate) glob: Option<&'a str>,
      pub(crate) path: Option<&'a str>,
      pub(crate) output_mode: &'a str,
      pub(crate) context: usize,
      pub(crate) case_insensitive: bool,
      pub(crate) head_limit: Option<usize>,
  }
  ```
  用结构体而非 7 个位置参数，避免 `clippy::too_many_arguments`，也避免调用方把同类型参数传串位。

- [ ] **Step 1: 写失败的测试**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TempDirectory;

    fn not_cancelled() -> Arc<AtomicBool> {
        Arc::new(AtomicBool::new(false))
    }

    fn request(pattern: &str, output_mode: &str) -> GrepRequest<'_> {
        GrepRequest {
            pattern,
            glob: None,
            path: None,
            output_mode,
            context: 0,
            case_insensitive: false,
            head_limit: None,
        }
    }

    fn workspace(name: &str) -> TempDirectory {
        let directory = TempDirectory::new(name);
        std::fs::create_dir(directory.path().join("src")).expect("mkdir src");
        std::fs::write(
            directory.path().join("src/alpha.rs"),
            "fn alpha() {}\nlet needle = 1;\n",
        )
        .expect("write alpha");
        std::fs::write(directory.path().join("src/beta.rs"), "fn beta() {}\n").expect("write beta");
        std::fs::write(directory.path().join("notes.md"), "needle in markdown\n")
            .expect("write notes");
        directory
    }

    #[test]
    fn files_with_matches_is_the_default_shape() {
        let directory = workspace("grep-files");
        let outcome = execute_grep(
            request("needle", "files_with_matches"),
            &directory.path().to_string_lossy(),
            not_cancelled(),
        );
        assert!(!outcome.is_error);
        assert!(outcome.output.contains("src/alpha.rs"));
        assert!(outcome.output.contains("notes.md"));
        assert!(!outcome.output.contains("src/beta.rs"));
        // 文件名模式不应回内容
        assert!(!outcome.output.contains("let needle = 1;"));
    }

    #[test]
    fn content_mode_returns_matching_lines_with_line_numbers() {
        let directory = workspace("grep-content");
        let outcome = execute_grep(
            request("needle", "content"),
            &directory.path().to_string_lossy(),
            not_cancelled(),
        );
        assert!(!outcome.is_error);
        assert!(outcome.output.contains("src/alpha.rs:2:let needle = 1;"));
    }

    #[test]
    fn count_mode_returns_per_file_counts() {
        let directory = workspace("grep-count");
        let outcome = execute_grep(
            request("needle", "count"),
            &directory.path().to_string_lossy(),
            not_cancelled(),
        );
        assert!(!outcome.is_error);
        assert!(outcome.output.contains("src/alpha.rs:1"));
    }

    #[test]
    fn a_glob_filter_narrows_the_file_set() {
        let directory = workspace("grep-glob");
        let mut input = request("needle", "files_with_matches");
        input.glob = Some("**/*.rs");
        let outcome = execute_grep(input, &directory.path().to_string_lossy(), not_cancelled());
        assert!(!outcome.is_error);
        assert!(outcome.output.contains("src/alpha.rs"));
        assert!(!outcome.output.contains("notes.md"));
    }

    #[test]
    fn case_insensitive_matching_is_opt_in() {
        let directory = workspace("grep-case");
        let sensitive = execute_grep(
            request("NEEDLE", "files_with_matches"),
            &directory.path().to_string_lossy(),
            not_cancelled(),
        );
        assert!(sensitive.output.contains("No matches"));

        let mut input = request("NEEDLE", "files_with_matches");
        input.case_insensitive = true;
        let insensitive =
            execute_grep(input, &directory.path().to_string_lossy(), not_cancelled());
        assert!(insensitive.output.contains("src/alpha.rs"));
    }

    #[test]
    fn context_lines_are_included_in_content_mode() {
        let directory = workspace("grep-context");
        let mut input = request("needle", "content");
        input.context = 1;
        let outcome = execute_grep(input, &directory.path().to_string_lossy(), not_cancelled());
        assert!(!outcome.is_error);
        assert!(outcome.output.contains("fn alpha() {}"));
    }

    #[test]
    fn an_invalid_regular_expression_is_reported_as_an_error() {
        let directory = workspace("grep-invalid");
        let outcome = execute_grep(
            request("(unclosed", "files_with_matches"),
            &directory.path().to_string_lossy(),
            not_cancelled(),
        );
        assert!(outcome.is_error);
        assert!(outcome.output.contains("Invalid regular expression"));
    }

    #[test]
    fn an_unknown_output_mode_is_rejected() {
        let directory = workspace("grep-mode");
        let outcome = execute_grep(
            request("needle", "sideways"),
            &directory.path().to_string_lossy(),
            not_cancelled(),
        );
        assert!(outcome.is_error);
    }

    #[test]
    fn binary_files_are_skipped_without_failing_the_search() {
        let directory = workspace("grep-binary");
        std::fs::write(directory.path().join("blob.bin"), b"needle\0\0binary").expect("write blob");
        let outcome = execute_grep(
            request("needle", "files_with_matches"),
            &directory.path().to_string_lossy(),
            not_cancelled(),
        );
        assert!(!outcome.is_error);
        assert!(!outcome.output.contains("blob.bin"));
    }

    #[test]
    fn gitignored_files_are_not_searched() {
        let directory = workspace("grep-gitignore");
        std::fs::write(directory.path().join(".gitignore"), "hidden.txt\n")
            .expect("write gitignore");
        std::fs::write(directory.path().join("hidden.txt"), "needle here").expect("write hidden");
        let outcome = execute_grep(
            request("needle", "files_with_matches"),
            &directory.path().to_string_lossy(),
            not_cancelled(),
        );
        assert!(!outcome.output.contains("hidden.txt"));
    }

    #[test]
    fn reports_no_matches_without_an_error() {
        let directory = workspace("grep-empty");
        let outcome = execute_grep(
            request("zzz-absent-zzz", "files_with_matches"),
            &directory.path().to_string_lossy(),
            not_cancelled(),
        );
        assert!(!outcome.is_error);
        assert!(outcome.output.contains("No matches"));
    }

    #[test]
    fn head_limit_truncates_and_says_so() {
        let directory = TempDirectory::new("grep-head-limit");
        for index in 0..20 {
            std::fs::write(directory.path().join(format!("f{index}.txt")), "needle")
                .expect("write fixture");
        }
        let mut input = request("needle", "files_with_matches");
        input.head_limit = Some(5);
        let outcome = execute_grep(input, &directory.path().to_string_lossy(), not_cancelled());
        assert!(!outcome.is_error);
        assert!(outcome.output.contains("truncated"));
        assert_eq!(
            outcome
                .output
                .lines()
                .filter(|line| line.starts_with('f'))
                .count(),
            5
        );
    }

    #[test]
    fn a_cancelled_search_is_reported_as_an_error() {
        let directory = workspace("grep-cancel");
        let outcome = execute_grep(
            request("needle", "files_with_matches"),
            &directory.path().to_string_lossy(),
            Arc::new(AtomicBool::new(true)),
        );
        assert!(outcome.is_error);
    }

    #[test]
    fn a_missing_workspace_folder_is_reported_as_an_error() {
        let outcome = execute_grep(
            request("needle", "files_with_matches"),
            "Z:/definitely/does/not/exist",
            not_cancelled(),
        );
        assert!(outcome.is_error);
    }
}
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --lib tools::grep_tool`
Expected: 编译失败 —— `cannot find function execute_grep` / `cannot find type GrepRequest`

- [ ] **Step 3: 实现**

```rust
//! 按正则搜索工作区文件内容。遍历与过滤委托给 `walk`；本模块负责正则匹配、三种输出形态与
//! 上限处理。

use super::walk::{exceeds_size_limit, is_binary, visit_workspace_files, Visit};
use super::{ToolExecutionOutcome, MAX_SEARCH_RESULTS, MAX_TOOL_OUTPUT_BYTES};
use globset::GlobBuilder;
use regex::RegexBuilder;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

pub(crate) const OUTPUT_MODE_FILES: &str = "files_with_matches";
pub(crate) const OUTPUT_MODE_CONTENT: &str = "content";
pub(crate) const OUTPUT_MODE_COUNT: &str = "count";

/// 用结构体承载 grep 的七个输入，避免长参数列表把同类型参数传串位。
pub(crate) struct GrepRequest<'a> {
    pub(crate) pattern: &'a str,
    pub(crate) glob: Option<&'a str>,
    pub(crate) path: Option<&'a str>,
    pub(crate) output_mode: &'a str,
    pub(crate) context: usize,
    pub(crate) case_insensitive: bool,
    pub(crate) head_limit: Option<usize>,
}

pub(crate) fn execute_grep(
    request: GrepRequest<'_>,
    workspace_folder: &str,
    cancelled: Arc<AtomicBool>,
) -> ToolExecutionOutcome {
    let pattern = request.pattern.trim();
    if pattern.is_empty() {
        return error("No search pattern was provided.");
    }
    if !matches!(
        request.output_mode,
        OUTPUT_MODE_FILES | OUTPUT_MODE_CONTENT | OUTPUT_MODE_COUNT
    ) {
        return error(&format!(
            "Unknown output_mode \"{}\". Expected one of: {OUTPUT_MODE_FILES}, {OUTPUT_MODE_CONTENT}, {OUTPUT_MODE_COUNT}.",
            request.output_mode
        ));
    }
    let matcher = match RegexBuilder::new(pattern)
        .case_insensitive(request.case_insensitive)
        .build()
    {
        Ok(matcher) => matcher,
        Err(failure) => {
            return error(&format!(
                "Invalid regular expression \"{pattern}\": {failure}"
            ));
        }
    };
    let file_filter = match request.glob.map(str::trim).filter(|glob| !glob.is_empty()) {
        Some(glob) => match GlobBuilder::new(glob)
            .literal_separator(true)
            .build()
            .map(|compiled| compiled.compile_matcher())
        {
            Ok(compiled) => Some(compiled),
            Err(failure) => {
                return error(&format!("Invalid glob pattern \"{glob}\": {failure}"));
            }
        },
        None => None,
    };

    let limit = request.head_limit.unwrap_or(MAX_SEARCH_RESULTS).min(MAX_SEARCH_RESULTS);
    let mut lines: Vec<String> = Vec::new();
    let mut bytes = 0usize;
    let mut truncated = false;

    let walk = visit_workspace_files(workspace_folder, request.path, &cancelled, &mut |absolute, relative| {
        if let Some(filter) = &file_filter {
            if !filter.is_match(relative) {
                return Visit::Continue;
            }
        }
        // 大小检查必须在 read 之前 —— 否则超大文件在被判定「跳过」之前就已经进了内存。
        // 搜索静默跳过而非报错：用户要的是「搜整个仓库」，不该因某一个大文件而整体失败。
        if exceeds_size_limit(absolute) {
            return Visit::Continue;
        }
        let Ok(raw) = std::fs::read(absolute) else {
            // 读不了的单个文件不该让整次搜索失败。
            return Visit::Continue;
        };
        if is_binary(&raw) {
            return Visit::Continue;
        }
        let Ok(text) = String::from_utf8(raw) else {
            return Visit::Continue;
        };
        let rendered = render_file(&matcher, relative, &text, request.output_mode, request.context);
        for line in rendered {
            bytes += line.len() + 1;
            lines.push(line);
            if lines.len() >= limit || bytes >= MAX_TOOL_OUTPUT_BYTES {
                truncated = true;
                return Visit::Stop;
            }
        }
        Visit::Continue
    });
    if let Err(failure) = walk {
        return error(&failure);
    }
    if lines.is_empty() {
        return ToolExecutionOutcome {
            output: format!("No matches for \"{pattern}\"."),
            is_error: false,
        };
    }
    let mut output = lines.join("\n");
    if truncated {
        output.push_str(
            "\n\n[Results truncated. Narrow the pattern, add a glob filter, or scope with path.]",
        );
    }
    ToolExecutionOutcome {
        output,
        is_error: false,
    }
}

/// 把单个文件的匹配结果渲染成该 output_mode 下的若干输出行。无匹配时返回空 Vec。
fn render_file(
    matcher: &regex::Regex,
    relative: &str,
    text: &str,
    output_mode: &str,
    context: usize,
) -> Vec<String> {
    let all: Vec<&str> = text.lines().collect();
    let hits: Vec<usize> = all
        .iter()
        .enumerate()
        .filter(|(_, line)| matcher.is_match(line))
        .map(|(index, _)| index)
        .collect();
    if hits.is_empty() {
        return Vec::new();
    }
    match output_mode {
        OUTPUT_MODE_FILES => vec![relative.to_string()],
        OUTPUT_MODE_COUNT => vec![format!("{relative}:{}", hits.len())],
        _ => {
            let mut wanted: Vec<usize> = Vec::new();
            for hit in &hits {
                let start = hit.saturating_sub(context);
                let end = (hit + context).min(all.len().saturating_sub(1));
                for index in start..=end {
                    if !wanted.contains(&index) {
                        wanted.push(index);
                    }
                }
            }
            wanted.sort_unstable();
            wanted
                .into_iter()
                .map(|index| format!("{relative}:{}:{}", index + 1, all[index]))
                .collect()
        }
    }
}

fn error(message: &str) -> ToolExecutionOutcome {
    ToolExecutionOutcome {
        output: message.to_string(),
        is_error: true,
    }
}
```

- [ ] **Step 4: 注册模块**

`tools/mod.rs`：

```rust
mod grep_tool;

pub(crate) use grep_tool::{execute_grep, GrepRequest};
```

- [ ] **Step 5: 运行测试确认通过**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --lib tools::grep_tool`
Expected: 14 passed

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/contexts/agent_runtime/infrastructure/tools/grep_tool.rs \
        src-tauri/src/contexts/agent_runtime/infrastructure/tools/mod.rs
git commit -m "feat(agent-runtime): add grep content search tool"
```

---

### Task 5: `edit` 工具

**Files:**
- Create: `src-tauri/src/contexts/agent_runtime/infrastructure/tools/edit_tool.rs`
- Modify: `src-tauri/src/contexts/agent_runtime/infrastructure/tools/mod.rs`

**Interfaces:**
- Consumes: `crate::platform::filesystem::BoundedFilesystem`、`walk::is_binary`
- Produces: `pub(crate) fn execute_edit(path: &str, old_string: &str, new_string: &str, replace_all: bool, workspace_folder: &str) -> ToolExecutionOutcome`

- [ ] **Step 1: 写失败的测试**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TempDirectory;

    fn workspace(name: &str, contents: &str) -> TempDirectory {
        let directory = TempDirectory::new(name);
        std::fs::write(directory.path().join("code.rs"), contents).expect("write fixture");
        directory
    }

    #[test]
    fn replaces_a_unique_match() {
        let directory = workspace("edit-unique", "let a = 1;\nlet b = 2;\n");
        let outcome = execute_edit(
            "code.rs",
            "let a = 1;",
            "let a = 42;",
            false,
            &directory.path().to_string_lossy(),
        );
        assert!(!outcome.is_error);
        assert_eq!(
            std::fs::read_to_string(directory.path().join("code.rs")).expect("read back"),
            "let a = 42;\nlet b = 2;\n"
        );
    }

    #[test]
    fn a_missing_match_is_reported_as_an_error_without_writing() {
        let directory = workspace("edit-missing", "let a = 1;\n");
        let outcome = execute_edit(
            "code.rs",
            "let z = 9;",
            "let z = 0;",
            false,
            &directory.path().to_string_lossy(),
        );
        assert!(outcome.is_error);
        assert!(outcome.output.contains("was not found"));
        assert_eq!(
            std::fs::read_to_string(directory.path().join("code.rs")).expect("read back"),
            "let a = 1;\n"
        );
    }

    #[test]
    fn multiple_matches_are_rejected_and_the_count_is_reported() {
        let directory = workspace("edit-multiple", "x = 1;\nx = 1;\nx = 1;\n");
        let outcome = execute_edit(
            "code.rs",
            "x = 1;",
            "x = 2;",
            false,
            &directory.path().to_string_lossy(),
        );
        assert!(outcome.is_error);
        assert!(outcome.output.contains('3'));
        assert_eq!(
            std::fs::read_to_string(directory.path().join("code.rs")).expect("read back"),
            "x = 1;\nx = 1;\nx = 1;\n"
        );
    }

    #[test]
    fn replace_all_replaces_every_match_and_reports_the_count() {
        let directory = workspace("edit-replace-all", "x = 1;\nx = 1;\n");
        let outcome = execute_edit(
            "code.rs",
            "x = 1;",
            "x = 2;",
            true,
            &directory.path().to_string_lossy(),
        );
        assert!(!outcome.is_error);
        assert!(outcome.output.contains('2'));
        assert_eq!(
            std::fs::read_to_string(directory.path().join("code.rs")).expect("read back"),
            "x = 2;\nx = 2;\n"
        );
    }

    #[test]
    fn an_identical_old_and_new_string_is_rejected() {
        let directory = workspace("edit-noop", "same\n");
        let outcome = execute_edit(
            "code.rs",
            "same",
            "same",
            false,
            &directory.path().to_string_lossy(),
        );
        assert!(outcome.is_error);
    }

    #[test]
    fn an_empty_old_string_is_rejected() {
        let directory = workspace("edit-empty-old", "content\n");
        let outcome = execute_edit(
            "code.rs",
            "",
            "new",
            false,
            &directory.path().to_string_lossy(),
        );
        assert!(outcome.is_error);
    }

    #[test]
    fn a_path_that_escapes_the_workspace_is_rejected() {
        let directory = workspace("edit-escape", "content\n");
        let outcome = execute_edit(
            "../outside.rs",
            "content",
            "new",
            false,
            &directory.path().to_string_lossy(),
        );
        assert!(outcome.is_error);
    }

    #[test]
    fn a_missing_file_is_reported_as_an_error() {
        let directory = workspace("edit-no-file", "content\n");
        let outcome = execute_edit(
            "absent.rs",
            "content",
            "new",
            false,
            &directory.path().to_string_lossy(),
        );
        assert!(outcome.is_error);
    }

    #[test]
    fn a_binary_file_is_refused() {
        let directory = TempDirectory::new("edit-binary");
        std::fs::write(directory.path().join("blob.bin"), b"abc\0def").expect("write blob");
        let outcome = execute_edit(
            "blob.bin",
            "abc",
            "xyz",
            false,
            &directory.path().to_string_lossy(),
        );
        assert!(outcome.is_error);
        assert!(outcome.output.contains("binary"));
    }
}
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --lib tools::edit_tool`
Expected: 编译失败 —— `cannot find function execute_edit`

- [ ] **Step 3: 实现**

```rust
//! 工作区内的定点串替换。默认要求 `old_string` 唯一匹配 —— 「改错位置」属于静默损坏，
//! 事后极难发现；报错时回报实际匹配次数，模型据此知道该补多少上下文。

use super::walk::{exceeds_size_limit, is_binary, MAX_FILE_BYTES};
use super::ToolExecutionOutcome;
use crate::platform::filesystem::BoundedFilesystem;
use std::path::Path;

pub(crate) fn execute_edit(
    path: &str,
    old_string: &str,
    new_string: &str,
    replace_all: bool,
    workspace_folder: &str,
) -> ToolExecutionOutcome {
    if old_string.is_empty() {
        return error("The old_string argument must not be empty.");
    }
    if old_string == new_string {
        return error("The old_string and new_string arguments are identical; nothing to change.");
    }
    let boundary = match BoundedFilesystem::new(Path::new(workspace_folder)) {
        Ok(boundary) => boundary,
        Err(failure) => return error(&format!("Workspace folder is unavailable: {failure}")),
    };
    let resolved = match boundary.resolve_existing(path) {
        Ok(resolved) => resolved,
        Err(failure) => return error(&format!("Path \"{path}\" is not accessible: {failure}")),
    };
    // 用户点名了这个文件，所以超限时报错而非静默跳过 —— 静默跳过等于骗人。
    if exceeds_size_limit(&resolved) {
        return error(&format!(
            "\"{path}\" is larger than the {} MB edit limit.",
            MAX_FILE_BYTES / (1024 * 1024)
        ));
    }
    let raw = match std::fs::read(&resolved) {
        Ok(raw) => raw,
        Err(failure) => return error(&format!("Failed to read \"{path}\": {failure}")),
    };
    if is_binary(&raw) {
        return error(&format!("\"{path}\" appears to be a binary file and cannot be edited as text."));
    }
    let text = match String::from_utf8(raw) {
        Ok(text) => text,
        Err(_) => return error(&format!("\"{path}\" is not valid UTF-8 text.")),
    };

    let occurrences = text.matches(old_string).count();
    if occurrences == 0 {
        return error(&format!("The old_string was not found in \"{path}\"."));
    }
    if occurrences > 1 && !replace_all {
        return error(&format!(
            "The old_string matches {occurrences} times in \"{path}\". Provide more surrounding context to make it unique, or set replace_all to true."
        ));
    }
    let updated = if replace_all {
        text.replace(old_string, new_string)
    } else {
        text.replacen(old_string, new_string, 1)
    };
    match std::fs::write(&resolved, &updated) {
        Ok(()) => ToolExecutionOutcome {
            output: format!("Replaced {occurrences} occurrence(s) in \"{path}\"."),
            is_error: false,
        },
        Err(failure) => error(&format!("Failed to write \"{path}\": {failure}")),
    }
}

fn error(message: &str) -> ToolExecutionOutcome {
    ToolExecutionOutcome {
        output: message.to_string(),
        is_error: true,
    }
}
```

- [ ] **Step 4: 注册模块**

`tools/mod.rs`：

```rust
mod edit_tool;

pub(crate) use edit_tool::execute_edit;
```

- [ ] **Step 5: 运行测试确认通过**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --lib tools::edit_tool`
Expected: 9 passed

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/contexts/agent_runtime/infrastructure/tools/edit_tool.rs \
        src-tauri/src/contexts/agent_runtime/infrastructure/tools/mod.rs
git commit -m "feat(agent-runtime): add scoped file edit tool"
```

---

### Task 6: `file` read 边界补强

**Files:**
- Modify: `src-tauri/src/contexts/agent_runtime/infrastructure/tools/file_tool.rs`

**Interfaces:**
- Consumes: `walk::{is_binary, exceeds_size_limit, MAX_FILE_BYTES}`、`super::MAX_TOOL_OUTPUT_BYTES`
- Produces: `execute_file` 签名变为
  `pub(crate) fn execute_file(operation: &str, path: &str, content: Option<&str>, offset: Option<usize>, limit: Option<usize>, workspace_folder: &str) -> ToolExecutionOutcome`
  —— Task 7 的路由需按此新签名调用。

- [ ] **Step 1: 写失败的测试**

在 `file_tool.rs` 既有 `mod tests` 中**追加**（既有 6 个测试保留，但调用处需补两个 `None` 参数）：

```rust
    #[test]
    fn read_output_is_prefixed_with_line_numbers() {
        let directory = TempDirectory::new("file-tool-line-numbers");
        std::fs::write(directory.path().join("a.txt"), "first\nsecond\n").expect("write fixture");
        let outcome = execute_file(
            "read",
            "a.txt",
            None,
            None,
            None,
            &directory.path().to_string_lossy(),
        );
        assert!(!outcome.is_error);
        assert!(outcome.output.contains("1\tfirst"));
        assert!(outcome.output.contains("2\tsecond"));
    }

    #[test]
    fn offset_and_limit_page_through_a_file() {
        let directory = TempDirectory::new("file-tool-paging");
        let body: String = (1..=10).map(|index| format!("line{index}\n")).collect();
        std::fs::write(directory.path().join("a.txt"), body).expect("write fixture");
        let outcome = execute_file(
            "read",
            "a.txt",
            None,
            Some(5),
            Some(2),
            &directory.path().to_string_lossy(),
        );
        assert!(!outcome.is_error);
        assert!(outcome.output.contains("6\tline6"));
        assert!(outcome.output.contains("7\tline7"));
        assert!(!outcome.output.contains("line8"));
        assert!(!outcome.output.contains("line5"));
    }

    #[test]
    fn exceeding_the_default_line_cap_reports_truncation() {
        let directory = TempDirectory::new("file-tool-line-cap");
        let body: String = (0..(MAX_READ_LINES + 50))
            .map(|index| format!("line{index}\n"))
            .collect();
        std::fs::write(directory.path().join("a.txt"), body).expect("write fixture");
        let outcome = execute_file(
            "read",
            "a.txt",
            None,
            None,
            None,
            &directory.path().to_string_lossy(),
        );
        assert!(!outcome.is_error);
        assert!(outcome.output.contains("truncated"));
    }

    #[test]
    fn an_overlong_line_is_truncated_and_marked() {
        let directory = TempDirectory::new("file-tool-long-line");
        let body = format!("{}\n", "x".repeat(MAX_READ_LINE_CHARS + 100));
        std::fs::write(directory.path().join("a.txt"), body).expect("write fixture");
        let outcome = execute_file(
            "read",
            "a.txt",
            None,
            None,
            None,
            &directory.path().to_string_lossy(),
        );
        assert!(!outcome.is_error);
        assert!(outcome.output.contains("… [line truncated]"));
    }

    #[test]
    fn a_binary_file_is_refused_with_a_clear_reason() {
        let directory = TempDirectory::new("file-tool-binary");
        std::fs::write(directory.path().join("blob.bin"), b"abc\0def").expect("write blob");
        let outcome = execute_file(
            "read",
            "blob.bin",
            None,
            None,
            None,
            &directory.path().to_string_lossy(),
        );
        assert!(outcome.is_error);
        assert!(outcome.output.contains("binary"));
    }

    #[test]
    fn an_offset_past_the_end_of_the_file_is_reported_without_an_error() {
        let directory = TempDirectory::new("file-tool-offset-past-end");
        std::fs::write(directory.path().join("a.txt"), "only\n").expect("write fixture");
        let outcome = execute_file(
            "read",
            "a.txt",
            None,
            Some(500),
            None,
            &directory.path().to_string_lossy(),
        );
        assert!(!outcome.is_error);
        assert!(outcome.output.contains("beyond the end"));
    }
```

同时把既有 6 个测试的 `execute_file(...)` 调用补齐两个 `None` 参数，例如：

```rust
        let outcome = execute_file("read", "a.txt", None, None, None, &directory.path().to_string_lossy());
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --lib tools::file_tool`
Expected: 编译失败 —— `this function takes 4 arguments but 6 arguments were supplied` / `cannot find value MAX_READ_LINES`

- [ ] **Step 3: 实现**

替换 `file_tool.rs` 中 `execute_file` 与 `read_file`（`write_file` 与既有 import 不动，另加两个 import）：

```rust
use super::walk::{exceeds_size_limit, is_binary, MAX_FILE_BYTES};
use super::MAX_TOOL_OUTPUT_BYTES;

/// read 的边界，三者取先触发者，均为硬上限。`limit` 参数只能把返回行数调得更少 ——
/// 上限保护的是上下文窗口，不是可协商的偏好，模型不能通过传一个大 `limit` 把自己撑爆。
/// 需要更多内容时用 `offset` 翻页。
pub(crate) const MAX_READ_LINES: usize = 2000;
pub(crate) const MAX_READ_LINE_CHARS: usize = 2000;

pub(crate) fn execute_file(
    operation: &str,
    path: &str,
    content: Option<&str>,
    offset: Option<usize>,
    limit: Option<usize>,
    workspace_folder: &str,
) -> ToolExecutionOutcome {
    let boundary = match BoundedFilesystem::new(Path::new(workspace_folder)) {
        Ok(boundary) => boundary,
        Err(error) => {
            return ToolExecutionOutcome {
                output: format!("Workspace folder is unavailable: {error}"),
                is_error: true,
            }
        }
    };
    match operation {
        "read" => read_file(&boundary, path, offset.unwrap_or(0), limit),
        "write" => write_file(&boundary, path, content.unwrap_or_default()),
        other => ToolExecutionOutcome {
            output: format!("Unknown file operation \"{other}\"."),
            is_error: true,
        },
    }
}

fn read_file(
    boundary: &BoundedFilesystem,
    path: &str,
    offset: usize,
    limit: Option<usize>,
) -> ToolExecutionOutcome {
    let resolved = match boundary.resolve_existing(path) {
        Ok(resolved) => resolved,
        Err(error) => {
            return ToolExecutionOutcome {
                output: format!("Path \"{path}\" is not accessible: {error}"),
                is_error: true,
            }
        }
    };
    // 用户点名了这个文件，所以超限时报错而非静默返回空 —— 大小检查必须在 read 之前，
    // 否则「保护内存」这个目的本身就落空了。
    if exceeds_size_limit(&resolved) {
        return ToolExecutionOutcome {
            output: format!(
                "\"{path}\" is larger than the {} MB read limit.",
                MAX_FILE_BYTES / (1024 * 1024)
            ),
            is_error: true,
        };
    }
    let raw = match std::fs::read(&resolved) {
        Ok(raw) => raw,
        Err(error) => {
            return ToolExecutionOutcome {
                output: format!("Failed to read \"{path}\": {error}"),
                is_error: true,
            }
        }
    };
    if is_binary(&raw) {
        return ToolExecutionOutcome {
            output: format!("\"{path}\" appears to be a binary file and cannot be read as text."),
            is_error: true,
        };
    }
    let text = match String::from_utf8(raw) {
        Ok(text) => text,
        Err(_) => {
            return ToolExecutionOutcome {
                output: format!("\"{path}\" is not valid UTF-8 text."),
                is_error: true,
            }
        }
    };

    let all: Vec<&str> = text.lines().collect();
    if offset >= all.len() && !all.is_empty() {
        return ToolExecutionOutcome {
            output: format!(
                "Offset {offset} is beyond the end of \"{path}\" ({} lines).",
                all.len()
            ),
            is_error: false,
        };
    }
    let requested = limit.unwrap_or(MAX_READ_LINES).min(MAX_READ_LINES);
    let mut rendered = Vec::new();
    let mut bytes = 0usize;
    let mut truncated = false;
    for (index, line) in all.iter().enumerate().skip(offset).take(requested) {
        let (body, clipped) = if line.chars().count() > MAX_READ_LINE_CHARS {
            (
                line.chars().take(MAX_READ_LINE_CHARS).collect::<String>(),
                true,
            )
        } else {
            ((*line).to_string(), false)
        };
        let entry = if clipped {
            format!("{}\t{body}… [line truncated]", index + 1)
        } else {
            format!("{}\t{body}", index + 1)
        };
        bytes += entry.len() + 1;
        rendered.push(entry);
        if bytes >= MAX_TOOL_OUTPUT_BYTES {
            truncated = true;
            break;
        }
    }
    if offset + rendered.len() < all.len() {
        truncated = true;
    }
    let mut output = rendered.join("\n");
    if truncated {
        output.push_str(&format!(
            "\n\n[Output truncated at line {}. The file has {} lines; continue with offset.]",
            offset + rendered.len(),
            all.len()
        ));
    }
    ToolExecutionOutcome {
        output,
        is_error: false,
    }
}
```

- [ ] **Step 4: 运行测试确认通过**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --lib tools::file_tool`
Expected: 12 passed（既有 6 个 + 新增 6 个）

> 注意：既有测试 `reads_an_existing_file_within_the_workspace` 原先断言 `outcome.output == "hello"`，加行号后应为 `"1\thello"`。修正该断言，不要放宽成 `contains`。

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/contexts/agent_runtime/infrastructure/tools/file_tool.rs
git commit -m "feat(agent-runtime): bound file reads with paging, line numbers, and limits"
```

---

### Task 7: 不可见管线（风险分级 + 信任白名单 + 路由 + plan mode 硬拒）

本任务只铺管线，**不改工具目录** —— 新工具此时尚未被提供给模型，因此以上改动对运行时行为无可见影响。Task 8 才是「上线」那一次提交，可被独立评审与回退。

**Files:**
- Modify: `src-tauri/src/contexts/agent_runtime/application/tool_catalog.rs`（新增常量、`risk_tier_for`、`requires_approval`）
- Modify: `src-tauri/src/contexts/agent_runtime/infrastructure/api_process_adapter.rs`（import 行 2 与 12-13、`execute_tool_call` 1258-1285）

**Interfaces:**
- Consumes: `execute_glob`、`execute_grep`、`GrepRequest`、`execute_edit`、`execute_file`（新签名）
- Produces: `GREP_TOOL_NAME`、`GLOB_TOOL_NAME`、`EDIT_TOOL_NAME` 三个 `pub(crate) const &str`，供 Task 8 的目录构建使用

- [ ] **Step 1: 写失败的测试**

在 `tool_catalog.rs` 的 `mod tests` 中追加：

```rust
    #[test]
    fn search_tools_do_not_require_approval() {
        assert_eq!(
            risk_tier_for(GREP_TOOL_NAME, &json!({"pattern": "needle"})),
            ToolRiskTier::AutoApprove
        );
        assert_eq!(
            risk_tier_for(GLOB_TOOL_NAME, &json!({"pattern": "**/*.rs"})),
            ToolRiskTier::AutoApprove
        );
    }

    #[test]
    fn edit_always_requires_approval() {
        assert_eq!(
            risk_tier_for(EDIT_TOOL_NAME, &json!({"path": "a.rs", "old_string": "a", "new_string": "b"})),
            ToolRiskTier::RequiresApproval
        );
    }

    #[test]
    fn a_trusted_agent_skips_approval_for_edit() {
        let input = json!({"path": "a.rs", "old_string": "a", "new_string": "b"});
        assert!(requires_approval(EDIT_TOOL_NAME, &input, false));
        assert!(!requires_approval(EDIT_TOOL_NAME, &input, true));
    }

    #[test]
    fn trust_does_not_widen_to_mcp_tools() {
        let input = json!({});
        assert!(requires_approval("mcp__server__tool", &input, true));
    }
```

在 `api_process_adapter.rs` 的 `mod tests` 中追加：

```rust
    #[test]
    fn execute_tool_call_routes_the_search_and_edit_tools_by_name() {
        let directory = TempDirectory::new("adapter-route-search");
        std::fs::write(directory.path().join("a.rs"), "let needle = 1;\n").expect("write fixture");
        let folder = directory.path().to_string_lossy().to_string();

        let grep = execute_tool_call(
            GREP_TOOL_NAME,
            &json!({"pattern": "needle"}),
            Some(&folder),
            Arc::new(AtomicBool::new(false)),
            "onepiece",
            &NoopMemories,
            &NoopMcp,
            false,
        );
        assert!(!grep.is_error);
        assert!(grep.output.contains("a.rs"));

        let glob = execute_tool_call(
            GLOB_TOOL_NAME,
            &json!({"pattern": "**/*.rs"}),
            Some(&folder),
            Arc::new(AtomicBool::new(false)),
            "onepiece",
            &NoopMemories,
            &NoopMcp,
            false,
        );
        assert!(!glob.is_error);
        assert!(glob.output.contains("a.rs"));

        let edit = execute_tool_call(
            EDIT_TOOL_NAME,
            &json!({"path": "a.rs", "old_string": "needle = 1", "new_string": "needle = 2"}),
            Some(&folder),
            Arc::new(AtomicBool::new(false)),
            "onepiece",
            &NoopMemories,
            &NoopMcp,
            false,
        );
        assert!(!edit.is_error);
    }

    #[test]
    fn execute_tool_call_rejects_edit_in_plan_mode() {
        let directory = TempDirectory::new("adapter-plan-edit");
        std::fs::write(directory.path().join("a.rs"), "let a = 1;\n").expect("write fixture");
        let outcome = execute_tool_call(
            EDIT_TOOL_NAME,
            &json!({"path": "a.rs", "old_string": "a = 1", "new_string": "a = 2"}),
            Some(&directory.path().to_string_lossy()),
            Arc::new(AtomicBool::new(false)),
            "onepiece",
            &NoopMemories,
            &NoopMcp,
            true,
        );
        assert!(outcome.is_error);
        assert!(outcome.output.contains("plan mode"));
        // 硬拒必须发生在触碰文件系统之前
        assert_eq!(
            std::fs::read_to_string(directory.path().join("a.rs")).expect("read back"),
            "let a = 1;\n"
        );
    }

    #[test]
    fn execute_tool_call_still_allows_search_tools_in_plan_mode() {
        let directory = TempDirectory::new("adapter-plan-search");
        std::fs::write(directory.path().join("a.rs"), "let needle = 1;\n").expect("write fixture");
        let outcome = execute_tool_call(
            GREP_TOOL_NAME,
            &json!({"pattern": "needle"}),
            Some(&directory.path().to_string_lossy()),
            Arc::new(AtomicBool::new(false)),
            "onepiece",
            &NoopMemories,
            &NoopMcp,
            true,
        );
        assert!(!outcome.is_error);
        assert!(outcome.output.contains("a.rs"));
    }
```

> `NoopMemories` / `NoopMcp` 是该测试模块中既有的测试替身。实现前先在 `api_process_adapter.rs` 的 `mod tests` 中确认其实际名称（搜索 `execute_tool_call_routes_shell_and_file_by_name` 这个既有测试，照抄它使用的替身与构造方式），不要新造。

- [ ] **Step 2: 运行测试确认失败**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --lib tool_catalog`
Expected: 编译失败 —— `cannot find value GREP_TOOL_NAME`

- [ ] **Step 3: 加常量与风险分级**

`tool_catalog.rs` 中，在既有常量旁加：

```rust
pub(crate) const GREP_TOOL_NAME: &str = "grep";
pub(crate) const GLOB_TOOL_NAME: &str = "glob";
pub(crate) const EDIT_TOOL_NAME: &str = "edit";
```

`risk_tier_for` 的 `match` 中加两个分支（`edit` 落入既有 `_` 默认分支即为 `RequiresApproval`，但显式列出可让契约测试锁住这一意图，避免日后有人改默认分支时静默放宽）：

```rust
        // 两者都只读，且都被工作区边界与 .gitignore 约束。让它们免审批正是本能力的目的 ——
        // 每次搜索都打断用户，模型就会退回用 shell 猜，反而更危险。
        GREP_TOOL_NAME | GLOB_TOOL_NAME => ToolRiskTier::AutoApprove,
        EDIT_TOOL_NAME => ToolRiskTier::RequiresApproval,
```

- [ ] **Step 4: 扩信任白名单**

`requires_approval` 的白名单条件改为：

```rust
    if auto_approve_tools
        && (tool_name == SHELL_TOOL_NAME
            || tool_name == FILE_TOOL_NAME
            || tool_name == EDIT_TOOL_NAME)
    {
        return false;
    }
```

并把该函数上方文档注释中的 "`auto_approve_tools` can only ever skip approval for `shell` and `file` calls" 改为 "for `shell`, `file`, and `edit` calls" —— 注释若与代码不符，下一个读它的人会被误导。

- [ ] **Step 5: 加路由与 plan mode 硬拒**

`api_process_adapter.rs` 第 2 行 import 改为：

```rust
use super::tools::{execute_edit, execute_file, execute_glob, execute_grep, execute_shell, GrepRequest, ToolExecutionOutcome};
```

第 12-13 行的 `use` 列表中补入 `EDIT_TOOL_NAME`、`GLOB_TOOL_NAME`、`GREP_TOOL_NAME`。

在 `execute_tool_call` 中，紧接既有 `if plan_mode && name == SHELL_TOOL_NAME` 块之后加：

```rust
    if plan_mode && name == EDIT_TOOL_NAME {
        return plan_mode_denial("Editing files");
    }
```

`match name` 中，`FILE_TOOL_NAME` 分支的 `execute_file` 调用改为新签名，并加三个新分支：

```rust
        FILE_TOOL_NAME => {
            let operation = input
                .get("operation")
                .and_then(Value::as_str)
                .unwrap_or_default();
            if plan_mode && operation != "read" {
                return plan_mode_denial("Writing files");
            }
            let path = input
                .get("path")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let content = input.get("content").and_then(Value::as_str);
            let offset = input.get("offset").and_then(Value::as_u64).map(|v| v as usize);
            let limit = input.get("limit").and_then(Value::as_u64).map(|v| v as usize);
            execute_file(operation, path, content, offset, limit, folder)
        }
        GREP_TOOL_NAME => execute_grep(
            GrepRequest {
                pattern: input.get("pattern").and_then(Value::as_str).unwrap_or_default(),
                glob: input.get("glob").and_then(Value::as_str),
                path: input.get("path").and_then(Value::as_str),
                output_mode: input
                    .get("output_mode")
                    .and_then(Value::as_str)
                    .unwrap_or(OUTPUT_MODE_FILES),
                context: input
                    .get("context")
                    .and_then(Value::as_u64)
                    .unwrap_or(0) as usize,
                case_insensitive: input
                    .get("case_insensitive")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
                head_limit: input.get("head_limit").and_then(Value::as_u64).map(|v| v as usize),
            },
            folder,
            cancelled,
        ),
        GLOB_TOOL_NAME => execute_glob(
            input.get("pattern").and_then(Value::as_str).unwrap_or_default(),
            input.get("path").and_then(Value::as_str),
            folder,
            cancelled,
        ),
        EDIT_TOOL_NAME => execute_edit(
            input.get("path").and_then(Value::as_str).unwrap_or_default(),
            input.get("old_string").and_then(Value::as_str).unwrap_or_default(),
            input.get("new_string").and_then(Value::as_str).unwrap_or_default(),
            input.get("replace_all").and_then(Value::as_bool).unwrap_or(false),
            folder,
        ),
```

`OUTPUT_MODE_FILES` 需从 `super::tools::grep_tool` 导出并 import —— 在 `tools/mod.rs` 中补 `pub(crate) use grep_tool::OUTPUT_MODE_FILES;`。

- [ ] **Step 6: 运行测试确认通过**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --lib tool_catalog`
Expected: 既有测试 + 4 个新测试全部 passed

Run: `cargo test --manifest-path src-tauri/Cargo.toml --lib api_process_adapter`
Expected: 既有测试 + 3 个新测试全部 passed

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/contexts/agent_runtime/application/tool_catalog.rs \
        src-tauri/src/contexts/agent_runtime/infrastructure/api_process_adapter.rs \
        src-tauri/src/contexts/agent_runtime/infrastructure/tools/mod.rs
git commit -m "feat(agent-runtime): wire search and edit tool routing, risk tiers, and trust"
```

---

### Task 8: 工具目录上线

这是把新能力真正暴露给模型的那一次提交。前序任务全部落地后再做，可被独立回退。

**Files:**
- Modify: `src-tauri/src/contexts/agent_runtime/application/tool_catalog.rs`（`tool_catalog`、`plan_mode_tool_catalog`、既有契约测试）

**Interfaces:**
- Consumes: Task 7 的三个工具名常量
- Produces: 6 个工具的完整目录；plan mode 目录 4 个

- [ ] **Step 1: 改失败的契约测试**

把既有的 `catalog_declares_exactly_shell_file_and_remember_tools` 与 `plan_mode_catalog_offers_only_read_only_file_and_remember` 替换为：

```rust
    #[test]
    fn catalog_declares_the_six_native_tools_in_a_stable_order() {
        let catalog = tool_catalog();
        let names: Vec<&str> = catalog.iter().map(|tool| tool.name.as_str()).collect();
        assert_eq!(
            names,
            vec![
                SHELL_TOOL_NAME,
                FILE_TOOL_NAME,
                GREP_TOOL_NAME,
                GLOB_TOOL_NAME,
                EDIT_TOOL_NAME,
                REMEMBER_TOOL_NAME,
            ]
        );
    }

    #[test]
    fn plan_mode_catalog_offers_only_read_only_tools() {
        let catalog = plan_mode_tool_catalog();
        let names: Vec<&str> = catalog.iter().map(|tool| tool.name.as_str()).collect();
        assert_eq!(
            names,
            vec![
                FILE_TOOL_NAME,
                GREP_TOOL_NAME,
                GLOB_TOOL_NAME,
                REMEMBER_TOOL_NAME,
            ]
        );
        assert_eq!(
            catalog[0].input_schema["properties"]["operation"]["enum"],
            json!(["read"])
        );
    }

    #[test]
    fn plan_mode_catalog_never_offers_shell_or_edit() {
        let names: Vec<String> = plan_mode_tool_catalog()
            .into_iter()
            .map(|tool| tool.name)
            .collect();
        assert!(!names.contains(&SHELL_TOOL_NAME.to_string()));
        assert!(!names.contains(&EDIT_TOOL_NAME.to_string()));
    }
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --lib tool_catalog`
Expected: FAIL —— 目录仍为 3 个工具，断言不匹配

- [ ] **Step 3: 加三个定义构造函数**

`grep` 与 `glob` 会同时出现在完整目录与 plan mode 目录中，两处重复维护迟早漂移。照既有 `remember_tool_definition()` 的做法，三个新工具各写一个私有构造函数（`edit` 只用于完整目录，但保持一致更好读）：

```rust
fn grep_tool_definition() -> ToolDefinition {
    ToolDefinition {
            name: GREP_TOOL_NAME.to_string(),
            description: "Search file contents in the session's workspace folder with a regular expression. Respects .gitignore and skips binary files. Prefer this over running grep through the shell.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "pattern": {
                        "type": "string",
                        "description": "Regular expression to search for."
                    },
                    "glob": {
                        "type": "string",
                        "description": "Optional glob limiting which files are searched, e.g. \"**/*.rs\"."
                    },
                    "path": {
                        "type": "string",
                        "description": "Optional subdirectory (relative to the workspace root) to search within."
                    },
                    "output_mode": {
                        "type": "string",
                        "enum": ["files_with_matches", "content", "count"],
                        "description": "\"files_with_matches\" (default) lists matching file paths; \"content\" returns matching lines with line numbers; \"count\" returns per-file match counts."
                    },
                    "context": {
                        "type": "integer",
                        "description": "Lines of context around each match. Only used when output_mode is \"content\"."
                    },
                    "case_insensitive": {
                        "type": "boolean",
                        "description": "Match case-insensitively. Defaults to false."
                    },
                    "head_limit": {
                        "type": "integer",
                        "description": "Maximum number of result lines to return."
                    }
                },
                "required": ["pattern"]
            }),
    }
}

fn glob_tool_definition() -> ToolDefinition {
    ToolDefinition {
            name: GLOB_TOOL_NAME.to_string(),
            description: "Find files by name pattern in the session's workspace folder. Respects .gitignore. Prefer this over listing files through the shell.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "pattern": {
                        "type": "string",
                        "description": "Glob pattern matched against workspace-relative paths, e.g. \"**/*.test.ts\"."
                    },
                    "path": {
                        "type": "string",
                        "description": "Optional subdirectory (relative to the workspace root) to search within."
                    }
                },
                "required": ["pattern"]
            }),
    }
}

fn edit_tool_definition() -> ToolDefinition {
    ToolDefinition {
            name: EDIT_TOOL_NAME.to_string(),
            description: "Replace an exact string in a file relative to the session's workspace folder. old_string must match exactly once unless replace_all is true. Prefer this over rewriting a whole file with the file tool.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Path relative to the workspace root."
                    },
                    "old_string": {
                        "type": "string",
                        "description": "Exact text to replace. Include enough surrounding context to match exactly once."
                    },
                    "new_string": {
                        "type": "string",
                        "description": "Replacement text."
                    },
                    "replace_all": {
                        "type": "boolean",
                        "description": "Replace every occurrence instead of requiring a unique match. Defaults to false."
                    }
                },
                "required": ["path", "old_string", "new_string"]
            }),
    }
}
```

- [ ] **Step 4: 两个目录都改用构造函数**

`tool_catalog()` 的 `vec![]` 改为六项，顺序与 Step 1 的契约测试一致：

```rust
    vec![
        /* 既有的 shell ToolDefinition，不动 */
        /* 既有的 file ToolDefinition，schema 补 offset/limit，见下 */
        grep_tool_definition(),
        glob_tool_definition(),
        edit_tool_definition(),
        remember_tool_definition(),
    ]
```

`plan_mode_tool_catalog()` 的 `vec![]` 改为四项：

```rust
    vec![
        /* 既有的只读 file ToolDefinition，schema 补 offset/limit */
        grep_tool_definition(),
        glob_tool_definition(),
        remember_tool_definition(),
    ]
```

两个目录中的 `file` 定义都要在 `input_schema.properties` 中补上分页属性（完整目录与只读版本各补一份）：

```rust
                    "offset": {
                        "type": "integer",
                        "description": "Line number to start reading from (0-based). Ignored when writing."
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Maximum number of lines to return. Ignored when writing."
                    }
```

- [ ] **Step 5: 运行测试确认通过**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --lib tool_catalog`
Expected: 全部 passed

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/contexts/agent_runtime/application/tool_catalog.rs
git commit -m "feat(agent-runtime): offer search and edit tools to native generations"
```

---

### Task 9: Web/mock 保真度

**Files:**
- Modify: `src/services/web-agent-client.ts`（约 3206-3270 行的模拟工具序列附近）

**Interfaces:**
- Consumes: 既有的 `pendingMockToolApprovals` / `resolveToolApproval` 流程
- Produces: 无（仅演示保真度）

> 已核实：前端 `contract-conformance.test.ts` 与 Rust `contract_tests.rs` 均不枚举原生工具名，因此本任务**不会**因遗漏而导致测试失败。它的目的是让 Web 演示模式与桌面能力不脱节。若 Task 1 的 spec delta 中写入了针对 Web runtime 的 grep/glob 场景，则本任务从「保真度改善」升级为「必做」。

- [ ] **Step 1: 补一个 grep 模拟调用**

在既有 `const rememberTimeoutId = setTimeout(...)` 那一段**之前**插入下面这段。`grep` 是 `AutoApprove`，因此走 `remember` 的无审批路径，而**不是** `shell` 的 `pendingMockToolApprovals` 门控路径 —— 若接错路径，Web 演示会表现出与桌面相反的审批行为：

```ts
      // Read-only search (`add-onepiece-search-and-edit-tools`): `grep` is classified
      // `AutoApprove`, so it follows `remember`'s no-approval path rather than `shell`'s gated
      // one. Output is a fixed fake result — the Web runtime never touches a real filesystem.
      const grepTimeoutId = setTimeout(() => {
        publishChatEvent({
          type: "tool_use",
          sessionId: input.sessionId,
          messageId: assistantMessage.id,
          toolUse: {
            id: `web-grep-${assistantMessage.id}`,
            name: "grep",
            input: { pattern: "export function", output_mode: "files_with_matches" },
            output: "src/App.tsx\nsrc/main.tsx",
            status: "completed",
          },
        });
      }, 233);
      timeoutIds.push(grepTimeoutId);
```

`233` 这个延迟把 grep 排在 shell 审批（230）之后、`remember`（235）之前，与真实运行时中「先搜索定位、再记忆」的顺序一致。

- [ ] **Step 2: 验证**

Run: `npm run test`
Expected: 全部 passed

Run: `npm run lint`
Expected: 无错误

- [ ] **Step 3: Commit**

```bash
git add src/services/web-agent-client.ts
git commit -m "feat(web): simulate a grep tool call in the mock agent runtime"
```

---

### Task 10: 全量验证与提案回填

**Files:**
- Modify: `openspec/changes/add-onepiece-search-and-edit-tools/tasks.md`

- [ ] **Step 1: 跑全量校验**

逐条执行 `AGENTS.md`「校验命令」，记录实际输出：

```bash
npm run test
npm run build
npm run lint
cargo test --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
openspec validate --specs --strict
openspec validate add-onepiece-search-and-edit-tools --strict
```

Expected: 全部通过。任一失败则修复后重跑，**不要**在 tasks.md 里记「大致通过」。

> 若遇到 `relay_tests.rs` / `relay_streamable_http*.rs` 的 socket 或 timing 失败，先重跑一次再判断 —— 这些用例在本仓已知间歇性失败，与本变更无关。

- [ ] **Step 2: 回填 tasks.md**

按 `add-gemini-cli-terminal-usage-tracking/tasks.md` 的详尽风格逐条回填：实际改了什么、测试数量变化（如「`cargo test --lib` — 从 N 增至 N+M」）、以及任何**未做项及其原因**（例如符号链接测试在 Windows 上被 `#[cfg(unix)]` 排除）。不要把未做项标成已做。

- [ ] **Step 3: Commit**

```bash
git add openspec/changes/add-onepiece-search-and-edit-tools/tasks.md
git commit -m "spec: record implementation results for OnePiece search and edit tools"
```

- [ ] **Step 4: 归档（仅在用户确认后执行）**

归档会改动主 specs，属不可逆的治理动作，**必须先征得用户同意**：

```bash
openspec archive add-onepiece-search-and-edit-tools
powershell -ExecutionPolicy Bypass -File scripts/Update-OpenSpecArchiveIndex.ps1
```

然后把主 specs、归档目录、索引一起提交（`AGENTS.md`「OpenSpec 归档治理」）。

---

## 附：本计划未覆盖的范围

以下属 OnePiece 能力补强路线图的其余子项，各自独立成 change，**不在本计划内**（依赖顺序见设计文档附录 A）：`api_process_adapter` 拆解与崩溃恢复、任务分解 / todo 规划、子 Agent 树、外部信息接入、插件注册自定义 mode、配置与会话体验。
