# Type Contracts

## Decision

Use `ts-rs` for Rust-to-TypeScript model contract generation.

VaneHub's frontend service interfaces intentionally stay hand-authored because they express UI/runtime semantics, but command payload/result models should be generated or verified from Rust models. `ts-rs` fits this shape: each serializable Rust model can derive a TypeScript representation without forcing a different Tauri command registration model.

## Alternatives Considered

- `specta` / `tauri-specta`: strong fit for end-to-end Tauri command type generation, but it adds tighter coupling to Tauri command wiring and has a broader integration surface for this codebase's current needs.
- Manual TypeScript definitions only: keeps dependencies low, but the existing parallel Rust/TypeScript models can drift silently as Agent, MCP, SDK, and operation models grow.

## Contract Scope

Generate or verify TypeScript models for:

- Agent registry entries, workflow state, readiness, launch results, and session details.
- MCP server configuration, partial updates, status, test results, import/export payloads, and tool metadata.
- SDK definitions, status maps, version maps, operation requests, operation results, and logs.
- Observable operation/task models introduced by the native task foundation.

## Workflow

1. Rust models that cross the frontend/backend boundary derive `TS` in addition to `Serialize` and `Deserialize`.
2. Generated files are written under a stable frontend contract directory.
3. Hand-authored service interfaces import generated model types where the generated shape is the service shape.
4. A verification command regenerates contracts and fails when committed contract files are stale.

This keeps React components behind service interfaces while reducing drift between Rust command models and TypeScript types.
