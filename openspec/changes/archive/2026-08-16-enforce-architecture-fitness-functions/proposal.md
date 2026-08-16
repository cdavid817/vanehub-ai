## Why

VaneHub's frontend/runtime boundaries, native DDD dependency rules, and repository constraints are documented but only partially enforced by focused tests and lint rules. A single, CI-required architecture fitness gate is needed now so later roadmap changes cannot silently erode those established boundaries.

## What Changes

- Add a registry of focused frontend, native, and repository architecture rules with stable rule ids and actionable file-and-line diagnostics.
- Add AST-backed frontend checks for direct Tauri access, runtime-specific component branching, prohibited state-management libraries, and Tauri/Web adapter contract parity.
- Extend the existing Rust syntax-tree architecture tests to cover command thinness and composition-root ownership in addition to domain/application and cross-context dependency direction.
- Preserve the existing 300-line, TypeScript `any`/`@ts-ignore`, Rust panic-path, and no-new-allowlist controls while exposing them through one `npm run architecture:check` entry point.
- Add positive and negative fixtures for every new detector and make the unified command an explicit CI gate with stable diagnostics.
- This affects repository tooling for both desktop and Web runtime implementations; it does not change runtime APIs, UI behavior, persistence, or user data.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `repository-governance`: Require a unified, fixture-tested architecture rule registry and dependency/state-management constraints without new blanket exemptions.
- `frontend-runtime-architecture`: Make the React service boundary and Tauri/Web adapter parity mechanically enforceable.
- `native-runtime-architecture`: Make DDD dependency direction, cross-context access, command thinness, and bootstrap composition mechanically enforceable.
- `continuous-integration`: Require architecture fitness as a named CI gate with actionable diagnostics.

## Impact

- Affected tooling: `package.json`, `eslint.config.js`, focused scripts and fixture tests under `scripts/`, and `.github/workflows/ci.yml`.
- Affected native verification: `src-tauri/tests/architecture.rs` and dedicated native architecture fixtures.
- Existing frontend service contracts and both runtime adapters remain unchanged; checks protect their parity rather than introducing a new abstraction.
- No new production dependency, state-management library, database migration, Tauri command, or UI surface is introduced.
