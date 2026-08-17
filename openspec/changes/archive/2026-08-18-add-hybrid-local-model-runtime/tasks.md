## 1. Planning Gate

- [x] 1.1 Review proposal, design, delta specs, active-change overlap, prerequisite archives, and affected existing contracts for internal consistency.
- [x] 1.2 Run `openspec validate add-hybrid-local-model-runtime --strict` and resolve every finding before editing business code.

## 2. Domain Contracts and Migration

- [x] 2.1 Add endpoint runtime kind, source, authentication, privacy, capability-state, context-provenance, timeout, and immutable Profile snapshot domain types with invariant tests.
- [x] 2.2 Add Hybrid task class, data policy, routing rule, selection reason, fallback/waiting outcome, and deterministic policy evaluation with privacy negative tests.
- [x] 2.3 Add a forward-only SQLite migration for Profile metadata and routing rules while preserving existing OnePiece/API Agent identities, active Profiles, and credential references.
- [x] 2.4 Extend Agent Runtime repository ports and SQLite adapters for atomic Profile/rule writes, deterministic ordering, disabled/dangling rule handling, and migration fixture coverage.

## 3. Discovery and Verification

- [x] 3.1 Add application ports and use cases for explicit manual endpoint validation, local discovery, and Profile verification through existing operations and unified diagnostics.
- [x] 3.2 Implement loopback-only allowlisted discovery with bounded concurrency, timeout, response size, redirect revalidation, model-list variations, and no generation requests.
- [x] 3.3 Add fake HTTP server tests for Ollama, LM Studio, vLLM, SGLang, generic OpenAI-compatible, timeout, malformed, oversized, redirect, and unsupported model-list responses.
- [x] 3.4 Add SSRF and privacy negative tests proving automatic discovery cannot scan LAN/non-loopback hosts and probes/logs contain no prompts, code, credentials, headers, or raw bodies.

## 4. Profile Application Services

- [x] 4.1 Extend OnePiece Profile list/save/edit/activate/delete/remove-all services for catalog and custom endpoints, optional authentication, conservative metadata, and compatibility defaults.
- [x] 4.2 Extend user-created API Agent registration/readiness for explicit unauthenticated local/private endpoints without weakening authenticated endpoint behavior.
- [x] 4.3 Add application tests for catalog immutability, unsafe custom input, credential preservation/removal, active snapshot stability, Profile deletion, and Local labeling semantics.

## 5. Hybrid Routing and Capability Admission

- [x] 5.1 Implement visible ordered rule CRUD, enable/disable behavior, task-class matching, direct active-Profile fallback, deterministic reason codes, and route preview.
- [x] 5.2 Integrate routing before Context Engine planning and freeze one Profile snapshot for credential, context, request, operation, and accounting attribution.
- [x] 5.3 Enforce `cloud-allowed`, `local-preferred`, and `local-only` admission including waiting-for-user-choice and no-cloud-contact negative tests.
- [x] 5.4 Negotiate text, tools, image, structured-output, and reasoning capabilities before provider contact; add unsupported/unknown/fallback tests.

## 6. Context and Provider Execution

- [x] 6.1 Feed verified/configured/unknown Profile capacity and reserve provenance into context measurement and Context Engine planning without model-name inference.
- [x] 6.2 Recompute context selection for a pre-contact fallback Profile and add context overflow plus cross-endpoint same-model tests.
- [x] 6.3 Extend the shared OpenAI-compatible gateway for optional authentication, Profile timeout, unsupported-field omission, and immutable routing metadata without product-specific branches.
- [x] 6.4 Bound context-limit recovery to one policy-authorized reduction attempt and add provider-down, timeout, malformed stream, missing usage, and no-fabricated-cost tests.
- [x] 6.5 Add large-stream chunk-partition and structural buffer/work-budget benchmarks proving ordered output and non-blocking bounded processing.

## 7. Native Commands and Composition

- [x] 7.1 Add one-command-per-file Tauri handlers and DTO/error mapping for Profile metadata, discovery/verification operations, rules, and route preview.
- [x] 7.2 Register commands and assemble concrete discovery, repository, credential, operations, logging, routing, and generation dependencies only in bootstrap.
- [x] 7.3 Add command serialization, classified-error, operation lifecycle, cancellation, redaction, and architecture-boundary tests.

## 8. Frontend Contracts and Adapters

- [x] 8.1 Extend shared TypeScript contracts/types and `AgentService` with exact Profile, capability, privacy, context, operation, rule, and preview models.
- [x] 8.2 Implement Tauri adapter command mappings and contract tests without direct `invoke()` use outside the adapter.
- [x] 8.3 Implement deterministic Web/mock Profile, discovery, verification, rule, routing, local-only, and operation behavior with parity tests and no network access.
- [x] 8.4 Run `npm run contracts:check` and targeted frontend/Rust contract tests; fix all parity findings before UI work.

## 9. Settings UI and Localization

- [x] 9.1 Add compact catalog/custom Profile controls for endpoint, discovery, verification, model, optional auth, timeout, capabilities, privacy, and context provenance using shared semantic tokens.
- [x] 9.2 Add visible Local/Private labels without security claims, operation progress/errors, capability warnings, and preserved existing Profile cards/actions.
- [x] 9.3 Add accessible Hybrid Routing rule editor, enable/disable controls, preferred/fallback selection, data policy, reason preview, and local-only waiting state.
- [x] 9.4 Add every user-visible key to every registered locale and pass resource parity and hard-coded-visible-text tests.
- [x] 9.5 Add Vitest component/service coverage for validation, discovery states, model variations, unsupported tools, privacy fallback, context limits, and narrow layouts.

## 10. E2E, Visual, Desktop, and Performance Verification

- [x] 10.1 Add cross-layer Playwright, Rust, and Desktop E2E flows for manual localhost Profile, discovery/verification, local text turn, routing/fallback, unsupported tools, local-only blocking, and missing usage.
- [x] 10.2 Add stable visual coverage for affected surfaces in futuristic/minimal themes at desktop and narrow viewports; inspect overlap, clipping, contrast, focus, and blank panels.
- [x] 10.3 Add Windows desktop localhost integration using a deterministic fake server and verify discovery, model listing, text streaming, operation state, and shutdown cleanup.
- [x] 10.4 Run deterministic performance benchmarks for discovery bounds, rule evaluation, context recalculation, and large streaming responses; record reproducible evidence.

## 11. Full Quality Gates

- [x] 11.1 Run `npm run lint:ci`, `npm run test`, and `npm run build`.
- [x] 11.2 Run `npm run test:coverage`, `npm run coverage:policy:test`, `npm run version:unit:test`, and `npm run contracts:check`.
- [x] 11.3 Run `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check` and `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`.
- [x] 11.4 Run `cargo test --manifest-path src-tauri/Cargo.toml` and `cargo check --manifest-path src-tauri/Cargo.toml`.
- [x] 11.5 Run `npx playwright test`, `npm run desktop:unit:test`, and `npm run test:desktop`; record actual Windows evidence and mark macOS/Linux `NOT RUN` unless independently executed.
- [x] 11.6 Run security negative suites and structural performance benchmarks again and retain sanitized evidence.
- [x] 11.7 Run `openspec validate --specs --strict` and `openspec validate add-hybrid-local-model-runtime --strict`.

## 12. Verification and Archive

- [x] 12.1 Verify every requirement/scenario against implementation and tests, resolve all critical/warning findings, and confirm no requirement 12+ roadmap work entered scope.
- [x] 12.2 Record implementation, migration, functional/UI/visual/spec/security/performance/Desktop Smoke evidence in the change artifacts before archive.
- [x] 12.3 Archive with `openspec archive add-hybrid-local-model-runtime`, then run `powershell -ExecutionPolicy Bypass -File scripts/Update-OpenSpecArchiveIndex.ps1`.
- [x] 12.4 Run post-archive `openspec validate --specs --strict`, verify the archive index and working tree, and prepare the complete implementation report.
