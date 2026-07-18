## 1. Shared Contracts

- [x] 1.1 Add Prompt Hook TypeScript models for hook category, stage, source, governance metadata, CLI bindings, preview result, mutation input, and trace summary.
- [x] 1.2 Extend `AgentService` with Prompt Hook list, create, update, delete, enabled-state, CLI-binding, preview, assembled-preview, and recent-trace methods.
- [x] 1.3 Add Tauri adapter methods that call declared native Prompt Hook commands without exposing `invoke()` to React components.
- [x] 1.4 Add Web/mock adapter methods with deterministic built-in hooks, local mock persistence, preview, mutation, binding, and trace behavior.
- [x] 1.5 Add frontend adapter contract tests for Prompt Hook model shape and Tauri/Web method parity.

## 2. Native Registry and Persistence

- [x] 2.1 Add Rust Prompt Hook models, supported category/stage/source/governance enums, validation helpers, and stable CLI agent id validation.
- [x] 2.2 Define the VaneHub built-in default Prompt Hook catalog with product-neutral hooks across `bootstrap`, `callback`, `dynamic`, `law`, `navigation`, `routing`, and `static`.
- [x] 2.3 Add additive SQLite migrations for built-in overrides, user-created hooks, CLI bindings, and recent safe trace summaries.
- [x] 2.4 Implement native list, create, update, delete, enable/disable, CLI-binding, preview, assembled-preview, and recent-trace commands.
- [x] 2.5 Add Rust tests for manifest/catalog validation, immutable built-in enforcement, user hook mutation validation, binding validation, and migration preservation.

## 3. Prompt Hook Pipeline

- [x] 3.1 Implement Rust pipeline ordering, bound-agent filtering, enabled-state resolution, template rendering, deterministic preview context, content hashing, and token estimates.
- [x] 3.2 Implement safe trace summary creation for fired, skipped, disabled, and failed hooks without including raw rendered content by default.
- [x] 3.3 Integrate Prompt Hook diagnostics with unified logging using redacted session id, agent id, hook id, status, hash, estimate, and safe reason codes.
- [x] 3.4 Add Rust tests for pipeline ordering, unbound hook skipping, immutable hook behavior, explicit preview content, trace redaction, and assembled prompt output.

## 4. Chat Runtime Integration

- [x] 4.1 Integrate Prompt Hook assembly into desktop CLI `send_message` after original user-message persistence and before provider invocation.
- [x] 4.2 Pass the assembled effective prompt to existing provider-specific invocation builders for `claude-code`, `codex-cli`, `gemini-cli`, and `opencode`.
- [x] 4.3 Preserve displayed and persisted user message content as the original trimmed input when hooks are applied.
- [x] 4.4 Handle Prompt Hook assembly failure by persisting a failed assistant message and writing redacted unified diagnostics.
- [x] 4.5 Add Rust chat tests covering four stable CLI agent ids, original-message preservation, hook-applied effective prompt, skipped unbound hooks, and assembly failure handling.

## 5. Settings UI

- [x] 5.1 Add the Prompt Hooks settings page registration, navigation icon, search placeholder, and synchronized zh-CN/en i18n keys.
- [x] 5.2 Build the service-backed Prompt Hooks page with shared data fetching, refresh state, stat summary, category/source/enabled/binding filters, and compact visual layout.
- [x] 5.3 Build Prompt Hook list/card components with enable locking, CLI binding checkboxes, source/governance labels, preview actions, edit/delete actions for user hooks, and stable no-layout-shift states.
- [x] 5.4 Build user hook create/edit/delete/preview dialogs with localized validation and bounded rendered-content display.
- [x] 5.5 Build recent trace summary display that shows hook id, status, hash, token estimate, timestamp, and skip reason by default, with explicit preview access only.
- [x] 5.6 Verify the Prompt Hooks page in both `futuristic` and `minimal` styles at desktop and narrow widths for clipping, overlap, and contrast.

## 6. Tests and Validation

- [x] 6.1 Add focused frontend tests for Prompt Hooks filtering, immutable toggles, CLI bindings, user-hook dialogs, preview behavior, trace summaries, and i18n-visible text.
- [x] 6.2 Add Web/mock service tests for built-in hooks, user hook CRUD, bindings, preview, and mock trace parity.
- [x] 6.3 Run `npm run test`.
- [x] 6.4 Run `npm run build`.
- [x] 6.5 Run `cargo test --manifest-path src-tauri/Cargo.toml`.
- [x] 6.6 Run `cargo check --manifest-path src-tauri/Cargo.toml`.
- [x] 6.7 Run `openspec validate --specs --strict`.
- [x] 6.8 Run `openspec validate enhance-prompt-hook-management --strict`.
