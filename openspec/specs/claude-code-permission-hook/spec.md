# claude-code-permission-hook Specification

## Purpose
TBD - created by archiving change add-claude-code-permission-callback. Update Purpose after archive.
## Requirements
### Requirement: PreToolUse requests resolve through the existing decision pipeline
The system SHALL translate every `PreToolUse` request the hook wrapper forwards into an `Action`/`Resource` pair and resolve it through the same `evaluate()`/`ApprovalBroker` pipeline used for native API agents, without a separate decision engine.

#### Scenario: Mapped tool call is evaluated identically to a native agent's
- **WHEN** the loopback server receives a `PreToolUse` request for a tool that maps to an existing `Action`
- **THEN** the system SHALL resolve it to `Allow`, `Deny`, or `Ask` using the same policy templates, grants, and audit trail a native agent's equivalent action would use

#### Scenario: Ask resolution blocks until a human decision or timeout
- **WHEN** the resolved effect is `Ask`
- **THEN** the system SHALL create a pending approval visible in the existing `ApprovalCard` UI and hold the HTTP response until it is resolved by a human decision or by the existing timeout sweep

### Requirement: Tool-to-Action mapping is explicit and partial
The system SHALL match only `Bash`, `Edit`, `Write`, `Read`, `Glob`, `Grep`, and MCP tool names (`mcp__*`) in the hook's matcher configuration, mapping them respectively to `shell.exec`, `file.write`, `file.write`, `file.read`, `file.read`, `file.read`, and `mcp.tool`, and SHALL NOT match any other tool name.

#### Scenario: Unmapped tool is not intercepted
- **WHEN** Claude Code invokes a tool with no configured mapping (for example `WebFetch`)
- **THEN** the hook SHALL NOT fire for that call
- **AND** Claude Code's native behavior for that tool SHALL be unaffected

#### Scenario: MCP tool call is floored at Ask
- **WHEN** a mapped MCP tool call reaches the decision pipeline
- **THEN** it SHALL resolve to `Ask` regardless of the `claude-code` principal's assigned template, matching `permissions-core`'s existing MCP floor

### Requirement: Loopback server is authenticated and bound to localhost only
The system SHALL bind the permission-hook HTTP server to a loopback address only, SHALL generate a random bearer token once per application launch, SHALL require every request to present that token, and SHALL reject any request missing or presenting an incorrect token.

#### Scenario: Request without a valid token is rejected
- **WHEN** a request to the loopback server does not present the current launch's token
- **THEN** the system SHALL reject the request without evaluating any permission decision

#### Scenario: Token and port are regenerated each launch
- **WHEN** the desktop application starts
- **THEN** the system SHALL generate a new random port and token for the permission-hook server and SHALL write both to a local discovery location the hook wrapper reads

### Requirement: Unreachable server falls back to a risk-tiered, asymmetric default
The system SHALL, when the hook wrapper cannot reach the loopback server for any reason (not running, connection refused, timeout, or missing discovery data), resolve only a fixed allowlist of read-only tools (`Read`, `Glob`, `Grep`) to `Allow`, and SHALL resolve every other tool to `Deny` in that condition.

#### Scenario: Server unreachable and tool is on the read-only allowlist
- **WHEN** the hook wrapper cannot reach the loopback server and the requested tool is `Read`, `Glob`, or `Grep`
- **THEN** the wrapper SHALL resolve the request to `Allow` without contacting VaneHub

#### Scenario: Server unreachable and tool is not on the allowlist
- **WHEN** the hook wrapper cannot reach the loopback server and the requested tool is not on the read-only allowlist, including tools the wrapper does not recognize
- **THEN** the wrapper SHALL resolve the request to `Deny`

### Requirement: Malformed hook payloads fail closed
The system SHALL resolve a `PreToolUse` request to `Deny` if the hook wrapper cannot parse the request or response payload, and SHALL NOT crash or hang the calling Claude Code process as a result.

#### Scenario: Unparseable stdin payload
- **WHEN** the hook wrapper receives a stdin payload it cannot parse
- **THEN** the wrapper SHALL exit denying the tool call
- **AND** SHALL NOT hang waiting for further input

#### Scenario: Unparseable server response
- **WHEN** the loopback server returns a response the wrapper cannot parse
- **THEN** the wrapper SHALL treat the request as denied

#### Scenario: Malformed response does not use the offline allowlist
- **WHEN** the loopback server responds but the wrapper cannot parse the response body
- **THEN** the wrapper SHALL resolve the request to `Deny` regardless of whether the requested tool is on the read-only offline-fallback allowlist

### Requirement: Wrapper timeout is bounded between VaneHub's approval timeout and Claude Code's hook ceiling
The system SHALL bound the hook wrapper's own client-side wait to a duration longer than the pending-approval timeout the native decision pipeline already enforces, and shorter than Claude Code's own hook timeout ceiling.

#### Scenario: Legitimate pending approval is not cut off early
- **WHEN** a mapped action resolves to `Ask` and a human has not yet responded
- **THEN** the wrapper SHALL continue waiting at least until the native pending-approval timeout has had a chance to resolve the request

#### Scenario: Wrapper always resolves before Claude Code's own ceiling
- **WHEN** the wrapper's own wait duration elapses without a response
- **THEN** the wrapper SHALL resolve the request as denied rather than waiting indefinitely

### Requirement: Web runtime has no permission-hook surface
The Web/mock runtime SHALL NOT start a loopback server, write a discovery file, or otherwise simulate this capability, since it has no user-facing surface of its own.

#### Scenario: Web/mock runtime is unaffected
- **WHEN** the application runs in Web/mock mode
- **THEN** no loopback server, discovery file, or hook wrapper behavior SHALL be present or simulated

### Requirement: Hook installation requires the wrapper binary to exist
VaneHub SHALL NOT write a permission-hook entry into Claude Code's global settings unless the wrapper binary named by that entry is present on disk. When it is absent, enabling hook management SHALL fail with an error identifying the expected location, and SHALL leave the user's Claude Code settings unmodified.

#### Scenario: Wrapper binary is absent
- **WHEN** hook management is enabled and the wrapper binary is not present at its resolved path
- **THEN** the operation SHALL fail with an error naming that path
- **AND** Claude Code's global settings SHALL NOT be modified

#### Scenario: Wrapper binary is present
- **WHEN** hook management is enabled and the wrapper binary exists at its resolved path
- **THEN** the hook entries SHALL be written as before

#### Scenario: Removing a hook installed by an earlier build
- **WHEN** hook management is disabled while the wrapper binary is absent
- **THEN** the removal SHALL still clear VaneHub's entries from Claude Code's settings

### Requirement: Packaged applications carry the permission-hook wrapper
Every supported Tauri package SHALL include the permission-hook wrapper as a target-specific external binary installed beside the main application executable. Runtime hook configuration SHALL resolve that installed location before any resource-directory fallback.

#### Scenario: A supported target is packaged
- **WHEN** a package command runs for Windows x64, macOS arm64, macOS x64, or Linux x64
- **THEN** it SHALL build and stage the wrapper for the same Rust target before Tauri bundling
- **AND** the resulting package SHALL contain the wrapper beside the main executable

#### Scenario: Packaged hook management is enabled
- **WHEN** the installed wrapper is present beside the main executable
- **THEN** hook configuration SHALL name that executable

#### Scenario: Install a hook from a Windows path

- **WHEN** the permission hook executable has a drive-qualified Windows path, including a directory whose name contains spaces
- **THEN** the projected command SHALL use shell-compatible separators and quoting
- **AND** Claude Code's hook shell SHALL preserve the drive, every path segment, and the executable name as one command token
- **AND** reinstalling the hook SHALL replace VaneHub's earlier raw-path entry while preserving non-owned hook entries

#### Scenario: An installation is incomplete or damaged
- **WHEN** the resolved packaged wrapper is absent
- **THEN** enabling hook management SHALL fail without modifying Claude Code's global settings

### Requirement: Wrapper provides a standalone uninstall escape hatch

The hook wrapper SHALL support a `--uninstall` invocation that removes only VaneHub-owned `PreToolUse` entries — identified by the wrapper binary's name appearing in an entry's hook command — from Claude Code's global settings file, SHALL preserve every other key and every non-owned hook entry, SHALL work without any running VaneHub instance or discovery data, and SHALL NOT modify the file when it cannot be parsed.

#### Scenario: Owned entries are removed and everything else survives

- **WHEN** `--uninstall` runs against a settings file containing VaneHub-owned `PreToolUse` entries alongside other keys and non-owned hook entries
- **THEN** the owned entries SHALL be removed
- **AND** every other key and non-owned hook entry SHALL be preserved
- **AND** the process SHALL exit successfully

#### Scenario: Nothing to remove is success, not failure

- **WHEN** `--uninstall` runs and the settings file is absent or contains no VaneHub-owned entries
- **THEN** the process SHALL exit successfully without modifying the file

#### Scenario: An unparseable settings file is left untouched

- **WHEN** `--uninstall` runs against a settings file that is not valid JSON
- **THEN** the file SHALL NOT be modified
- **AND** the process SHALL exit with a nonzero status and an error message

### Requirement: Offline denial names its recovery paths

The hook wrapper SHALL, when denying a tool call because the loopback server cannot be reached, produce a deny reason that distinguishes the absence of discovery data from a present-but-unreachable instance, and SHALL name both recovery actions — starting VaneHub, and running the wrapper's `--uninstall` escape hatch — in each variant.

#### Scenario: Discovery data exists but the instance is unreachable

- **WHEN** discovery data is present and the connection to the loopback server fails
- **THEN** the deny reason SHALL state that the VaneHub instance is not reachable
- **AND** SHALL name both recovery actions

#### Scenario: No discovery data exists

- **WHEN** no discovery data can be read at all
- **THEN** the deny reason SHALL state that no VaneHub instance has registered on this machine
- **AND** SHALL name both recovery actions

### Requirement: Hook registration reconverges at desktop startup

The desktop application SHALL, at startup, re-project the permission-hook entries into Claude Code's global settings when and only when the `claude-code` principal has a previously assigned template row, so that the entries name the current wrapper binary path after an application update. A reconvergence failure SHALL NOT block the rest of permissions bootstrap.

#### Scenario: Hook management was previously enabled

- **WHEN** the desktop application starts and the `claude-code` principal has an assigned template row
- **THEN** the hook entries SHALL be re-projected naming the currently resolved wrapper path

#### Scenario: Hook management was never enabled

- **WHEN** the desktop application starts and the `claude-code` principal has no assigned template row
- **THEN** Claude Code's global settings SHALL NOT be modified

#### Scenario: Reconvergence fails

- **WHEN** the startup re-projection fails for any reason
- **THEN** permissions bootstrap SHALL continue
- **AND** CLI sessions SHALL remain governed by the existing offline fallback behavior

### Requirement: Permission decisions are scoped to VaneHub-managed Claude Code sessions
The hook wrapper SHALL produce a VaneHub permission decision only when the invoking Claude Code process carries the explicit VaneHub-managed hook scope. For every unscoped invocation, the wrapper SHALL exit successfully without emitting a permission decision, contacting the VaneHub loopback server, or changing Claude Code's native permission behavior.

#### Scenario: VaneHub-managed Claude Code invokes a mapped tool
- **WHEN** a Claude Code process launched by VaneHub carries the managed hook scope and invokes a mapped tool
- **THEN** the wrapper SHALL resolve the request through the existing authenticated VaneHub permission pipeline
- **AND** the existing approval, audit, timeout, and offline fallback behavior SHALL remain in effect

#### Scenario: Independently launched Claude Code invokes Bash
- **WHEN** a Claude Code process without the managed hook scope invokes `Bash`
- **THEN** the wrapper SHALL exit successfully without emitting `allow`, `ask`, or `deny`
- **AND** Claude Code SHALL continue through its native permission flow

#### Scenario: Unmanaged invocation cannot reach VaneHub
- **WHEN** a Claude Code process without the managed hook scope invokes a matched tool while VaneHub is unavailable
- **THEN** the wrapper SHALL NOT apply the managed-session offline denial
- **AND** SHALL NOT attempt to read discovery data or contact the loopback server

### Requirement: Claude hook Ask responses use committed immutable resolutions

A Claude Code `PreToolUse` request that resolves to Ask SHALL remain blocked until the permissions application use case has claimed the pending request, verified the current hook waiter, and committed an immutable approval resolution and audit. The loopback response SHALL carry or be correlated with the immutable resolution id, and the hook waiter SHALL apply that resolution at most once.

#### Scenario: Human Allow cannot precede persistence

- **WHEN** a user approves a Claude Code hook request and the resolution transaction has not committed
- **THEN** the loopback server SHALL NOT return Allow to the hook wrapper
- **AND** Claude Code SHALL remain blocked or receive a fail-closed typed failure rather than execute early

#### Scenario: The same resolution is delivered twice

- **WHEN** retry logic attempts to deliver the same committed resolution id to one hook waiter more than once
- **THEN** the waiter SHALL apply the first valid delivery at most once
- **AND** subsequent deliveries SHALL return an idempotent acknowledgement without releasing another tool execution

#### Scenario: Hook waiter ended before reservation

- **WHEN** the HTTP request, hook timeout, or originating generation ends before the resolution use case reserves the waiter
- **THEN** the resolution SHALL be classified stale
- **AND** no Allow response or remembered grant SHALL be produced for that ended waiter

### Requirement: Hook delivery uncertainty fails closed across restart

A committed hook resolution without an acknowledged loopback delivery SHALL NOT be replayed after application restart. Its remembered-grant intent SHALL remain inactive and a later Claude Code invocation SHALL undergo a new permission evaluation.

#### Scenario: Application restarts during hook delivery

- **WHEN** the application restarts after committing a hook approval but before recording acknowledgement
- **THEN** the old hook request SHALL remain unresolved only as durable evidence
- **AND** a new hook invocation SHALL NOT inherit execution authority from that uncertain delivery

