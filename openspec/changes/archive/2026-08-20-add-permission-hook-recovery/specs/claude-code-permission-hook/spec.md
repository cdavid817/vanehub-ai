## ADDED Requirements

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
