## ADDED Requirements

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
