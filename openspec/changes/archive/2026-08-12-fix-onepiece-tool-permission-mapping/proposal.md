## Why

OnePiece's unified-permission adapter currently maps several built-in tools to synthetic `unknown:*` actions. As a result, read-only tools such as `glob` prompt unexpectedly and trusted agents cannot apply their configured policy to `edit`, contradicting the existing OnePiece and permissions-core specifications.

## What Changes

- Map every built-in OnePiece tool to an established permission action and resource instead of falling through to the unknown-tool fail-closed path.
- Keep unknown and hallucinated tool names fail-closed at `Ask`.
- Add table-driven regression coverage for the complete built-in OnePiece tool catalog, including MCP and unknown-tool boundaries.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

None. This change restores behavior already required by `onepiece-native-agent` and `permissions-core`; it does not change their contracts.

## Impact

- Desktop runtime only: `src-tauri` native OnePiece tool permission classification and focused Rust tests.
- No frontend service contract, Web/mock adapter, database schema, dependency, or runtime-boundary changes.
