## 1. Schema and domain contracts

- [x] 1.1 Extend Skill frontmatter parsing with optional `config_schema` and add domain models for normalized schemas, fields, presentation metadata, secret classification, scope, provenance, readiness, drift, revisions, and immutable snapshots.
- [x] 1.2 Implement canonical schema hashing and the bounded supported subset for scalar, enum, bounded scalar-list, and one-level grouped fields with size, depth, key, default, annotation, and reference restrictions.
- [x] 1.3 Add fixtures and tests for valid schemas plus unknown keywords, unsafe references, duplicate normalized keys, invalid defaults, excessive depth/size, unsupported types, and malicious labels/help text.
- [ ] 1.4 Extend effective Skill overview contracts with configurable state, active schema/revision, available scopes, required readiness, drift, and redacted scoped summaries.

## 2. Persistence and effective resolution

- [ ] 2.1 Add additive SQLite migrations for typed non-secret User/Project records, canonical workspace identity, schema/base revision witnesses, stored revision, validation state, and secret-presence metadata.
- [ ] 2.2 Implement repositories with compare-and-swap saves, property-level overrides, reset, orphan retention, cleanup state, and migration-equivalence coverage for existing databases.
- [ ] 2.3 Implement effective resolution as Project over User over schema default with stable property ordering, per-property provenance, required readiness, and no inherited-value materialization.
- [ ] 2.4 Reject Project operations without a canonical workspace and test isolation between workspaces, scopes, similarly named Skills, and shadowed Skill revisions.
- [ ] 2.5 Implement schema drift classification for compatible, migration-required, and invalid states without coercing, deleting, or activating incompatible values.
- [ ] 2.6 Add corruption recovery and concurrency tests proving a failed/stale save preserves the prior complete stored record.

## 3. Secret isolation and compensation

- [ ] 3.1 Extend the native credential-store abstraction with opaque Skill configuration slots and read models limited to configured, missing, or error state.
- [ ] 3.2 Implement preserve, replace, and clear mutation intents without returning stored values or credential aliases through DTOs.
- [ ] 3.3 Implement credential/SQLite compensation and explicit recovery state for failures before, during, and after each resource update.
- [ ] 3.4 Require explicit reconciliation when a property changes between secret and non-secret and prevent automatic movement between SQLite and the credential store.
- [ ] 3.5 Add tests proving secrets never enter SQLite bodies, frontend responses, Web persistence, logs, prompts, transcripts, dossiers, usage records, evolution signals, or error messages.

## 4. Validation, preview, and reconciliation services

- [ ] 4.1 Implement authoritative validate, effective-preview, save, reset-property, reset-scope, secret-clear, reconcile, and deletion-retention application operations.
- [ ] 4.2 Validate exact schema hash, base revision, stored revision, keys, types, formats, constraints, scope, payload size, and required readiness before mutation.
- [ ] 4.3 Preserve prior state and unsaved-compatible error detail for unknown keys, stale revisions, schema changes, credential failures, repository failures, and invalid workspace identity.
- [ ] 4.4 Implement explicit reconciliation that lets users map or discard obsolete non-secret fields and clear incompatible credential slots without silent conversion.
- [ ] 4.5 Emit redacted unified-log events for validation, save, reset, secret mutation, drift, reconciliation, archive, restore, delete, retention, and cleanup.

## 5. Runtime snapshots and activation

- [ ] 5.1 Implement immutable snapshot creation bound to Skill id, effective base/schema revisions, canonical workspace, non-secret values, provenance, secret-presence state, readiness, and digest.
- [ ] 5.2 Resolve one snapshot at Role activation and keep it fixed for that loaded Role context until a later activation.
- [ ] 5.3 Resolve one snapshot at Utility delegation and keep it isolated to that child execution even when configuration changes concurrently.
- [ ] 5.4 Integrate snapshot consumption with Skill tool invocation while allowing secret use only through an explicitly declared, permission-checked native property capability.
- [ ] 5.5 Fail only the affected activation when required configuration is missing, invalid, migration-required, or oversized and preserve unrelated Agent prompt sections and Skills.
- [ ] 5.6 Add concurrency and lifecycle tests for edits during activation, scope precedence changes, archive, disable, restore, replacement, deletion, cancellation, and stale snapshots.

## 6. Prompt, CLI, Overlay, and evolution boundaries

- [ ] 6.1 Serialize bounded non-secret configuration in stable key order for eligible native API Skill contexts and count it against a declared configuration subsection budget.
- [ ] 6.2 Include only configured/missing state for secret properties and test that over-budget configuration fails activation instead of truncating semantic values.
- [ ] 6.3 Keep external CLI mounts, files, environment variables, arguments, and processes free of managed values and expose unsupported configuration consumption for bindings without a bridge.
- [ ] 6.4 Recompute schema hashes and drift when an authorized Overlay changes `config_schema`, while preventing Overlay operations from writing User/Project records or credential slots.
- [ ] 6.5 Filter runtime values and credential aliases from evolution signals, candidate seeds, LLM review, dossiers, Curator records, notifications, and auto-apply; retain only safe structural readiness metadata.

## 7. Commands and frontend adapters

- [ ] 7.1 Add Rust/Tauri commands for descriptor/read, validate/preview, save, reset, secret clear, reconcile, retention choice, and cleanup with mapped command-boundary errors.
- [ ] 7.2 Register commands and extend generated/shared TypeScript contracts for normalized descriptors, scope/provenance, redacted values, secret intent/state, validation, drift, revisions, and runtime support.
- [ ] 7.3 Extend `AgentService` and `tauri-agent-client.ts` with configuration operations, keeping direct `invoke()` calls out of React components.
- [ ] 7.4 Extend `web-agent-client.ts` with deterministic non-secret preview/edit parity and explicit unsupported secure-secret persistence/native consumption; never fabricate configured credentials.
- [ ] 7.5 Add adapter contract tests for stale responses, selected-Skill changes, workspace changes, backend errors, secret redaction, and Web/native capability differences.

## 8. Configuration UI

- [ ] 8.1 Add a Configuration tab to Skill details with readiness, schema revision, scope switcher, inheritance/effective provenance, and non-configurable state.
- [ ] 8.2 Build normalized descriptor-driven controls for supported text, multiline, integer, number, boolean, enum, bounded multi-value, grouping, advanced disclosure, required, description, and default metadata.
- [ ] 8.3 Add secret controls that show only configured/missing/error and submit explicit preserve, replace, or clear intent with confirmation where needed.
- [ ] 8.4 Add effective preview, per-field and summary validation, save, property/scope reset, draft preservation, stale-write refresh, and explicit drift reconciliation flows.
- [ ] 8.5 Add archive/delete retention choices and credential-cleanup recovery status without exposing aliases or values.
- [ ] 8.6 Keep production components below 300 physical lines and add keyboard, screen-reader, focus, non-color status, narrow viewport, sanitized label, and no-horizontal-overflow tests.
- [ ] 8.7 Add Playwright scenarios for User/Project inheritance, secret replace/preserve/clear, invalid save, concurrent stale save, schema drift, reconciliation, archive/restore, deletion choice, and unsupported Web behavior.

## 9. Lifecycle and operational safety

- [ ] 9.1 Refresh schema validation and configuration readiness on Skill create, import, enable, effective-scope change, replace, restore, archive, and delete.
- [ ] 9.2 Retain archived configuration without issuing snapshots and revalidate it before restored activation.
- [ ] 9.3 Require retain-or-delete choice for configured user-created Skill deletion and implement bounded orphan retention plus explicit audited cleanup.
- [ ] 9.4 Add tests proving one invalid configuration disables only its affected Skill activation and cannot corrupt packages, other Skills, workspaces, prompts, tools, or sessions.
- [ ] 9.5 Add a runtime consumption kill switch that stops issuing new snapshots while preserving stored configuration and credentials for rollback.

## 10. Verification

- [ ] 10.1 Run `npm run lint:ci`, `npm run test`, `npm run test:coverage`, `npm run build`, `npm run coverage:policy:test`, `npm run version:unit:test`, and `npm run contracts:check`.
- [ ] 10.2 Run `npx playwright test` for Skill configuration UI behavior.
- [ ] 10.3 Run `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`, `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`, `cargo test --manifest-path src-tauri/Cargo.toml`, and `cargo check --manifest-path src-tauri/Cargo.toml`.
- [ ] 10.4 Run targeted credential compensation, secret-leak scanning, schema fuzz/adversarial fixtures, concurrency, migration-equivalence, and Web/native adapter parity tests.
- [ ] 10.5 Run `openspec validate add-skill-configuration-management --strict` and `openspec validate --specs --strict`, then record implementation and rollback evidence before archive.
