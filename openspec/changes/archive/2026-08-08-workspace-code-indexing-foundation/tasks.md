## 1. Retrieval Generalization Guardrails

- [x] 1.1 Add failing domain and repository tests for `WorkspaceFile`, explicit retrieval scopes, workspace-filtered vector/FTS candidates, scoped status, rebuild, delete, and stale-model requeue.
- [x] 1.2 Add regression tests proving `recall` still searches the host-wide agent-memory pool and keeps its existing model-visible schema and payload.
- [x] 1.3 Parameterize indexing reconciliation, pending batch claims, model invalidation, and search by source kind and validated scope without changing memory behavior.
- [x] 1.4 Split the bootstrap wiring into memory and workspace-code source adapters and add bounded round-robin source scheduling tests.

## 2. Persistence and Workspace Identity

- [x] 2.1 Add an idempotent SQLite migration for code-index workspace configuration, file manifests, chunk metadata, symbols, bounded audit rows, indexes, and foreign-key cleanup.
- [x] 2.2 Extend retrieval document persistence with `workspace_file` companion metadata and transactional file replacement/removal operations.
- [x] 2.3 Implement stable workspace UUID creation, canonical-root resolution, normalized relative paths, unavailable-root state, and duplicate-root validation.
- [x] 2.4 Add migration and repository tests proving existing agent-memory rows remain readable and workspace deletion cannot affect another workspace or memory.

## 3. Configuration and Safe File Admission

- [x] 3.1 Add Rust configuration models and validation for enablement, selected relative roots, supported language selection, exclusion globs, and the default 100 KiB maximum file size.
- [x] 3.2 Implement workspace-bounded inventory with `ignore::WalkBuilder`, nested `.gitignore`, binary detection, selected roots, size checks, and non-following of escaping symlinks.
- [x] 3.3 Implement and test the non-overridable case-normalized sensitive-file denylist for environment, credential, key, certificate, and common credential-directory patterns.
- [x] 3.4 Implement user exclusion glob compilation with atomic rejection of invalid configurations and deterministic safe skip reason counts.
- [x] 3.5 Add workspace configuration and inventory application-service tests covering precedence between boundary, mandatory denylist, user excludes, language, binary, and size gates.

## 4. Redaction, Tree-sitter Parsing, and Chunking

- [x] 4.1 Add pinned Tree-sitter dependencies and query assets for JavaScript, TypeScript/TSX, Python, Rust, Go, Java, C, and C++ parser families.
- [x] 4.2 Implement bounded content loading, raw SHA-256 file hashing, language detection, parser dispatch, and safe parse failure categories.
- [x] 4.3 Implement the shared code-secret redaction policy and tests proving detected values never enter persisted chunks, embedding inputs, audit content, or search snippets.
- [x] 4.4 Implement symbol extraction with normalized name, kind, container, definition range, and deterministic occurrence identity for every supported parser family.
- [x] 4.5 Implement symbol-first bounded chunking, named-node and line-window fallback splitting, structural context, deterministic chunk keys, and syntax-error recovery.
- [x] 4.6 Add parser fixtures and unit tests for every language, oversized symbols, duplicate names, multibyte line ranges, partial syntax errors, and files with no named definitions.
- [x] 4.7 Define and persist `CODE_INDEX_VERSION` across grammar/query, chunking, and redaction policy and test version-mismatch invalidation.

## 5. Manifest Reconciliation and Worker Control

- [x] 5.1 Add failing application tests for initial inventory, unchanged fingerprint skips, same-content metadata changes, create/change/delete/rename path sets, and atomic stale-chunk cleanup.
- [x] 5.2 Implement manifest inventory reconciliation that reads and parses only new, fingerprint-changed, explicitly targeted, or stale-version files.
- [x] 5.3 Implement `reconcile_paths` for bounded create, modify, rename, and delete sets and retain low-frequency metadata inventory as recovery.
- [x] 5.4 Add workspace generation tokens and cooperative cancellation checks between files and embedding batches, discarding stale in-flight results.
- [x] 5.5 Add provider-profile throttling with one in-flight request, a configurable inter-batch interval, bounded `Retry-After`, hard HTTP timeout, and deterministic worker tests.
- [x] 5.6 Implement phased per-workspace status, internally consistent progress counters, safe failure categories, estimated embedding requests, and bounded local audit retention.

## 6. Embedding Confirmation and Scoped Code Search

- [x] 6.1 Implement local FTS completion before embedding and workspace-specific confirmation state keyed by effective provider, model, and code-index generation.
- [x] 6.2 Add tests proving no code embedding request occurs before confirmation and provider/model changes exclude old vectors and require renewed confirmation.
- [x] 6.3 Define a separate typed code-retrieval port and `search_code` result containing relative path, line range, language, optional symbol metadata, redacted snippet, and matched-via value.
- [x] 6.4 Implement workspace-filtered hybrid code search with keyword-only degradation before confirmation or during embedding failure.
- [x] 6.5 Register `search_code` only for an enabled local current-session workspace, expose exactly `query` and `limit`, and ignore model-supplied scope fields.
- [x] 6.6 Add agent-runtime tests proving cross-workspace candidates cannot appear and returned locations can be passed to the existing `read_file` offset/limit contract.

## 7. Native Commands and Frontend Service Contracts

- [x] 7.1 Add Tauri commands and DTOs for listing workspace indexes, reading/saving configuration, inventory refresh, embedding confirmation, status, audit, rebuild, disable, and delete.
- [x] 7.2 Register commands and expose them through `AgentService` and `tauri-agent-client.ts` without direct component `invoke()` usage.
- [x] 7.3 Implement matching `web-agent-client.ts` state and deterministic mock phase transitions without filesystem or embedding network access.
- [x] 7.4 Add TypeScript contract normalization and service-adapter tests for invalid phases, counts, patterns, languages, and workspace identities.

## 8. Workspace and Retrieval UI

- [x] 8.1 Add a workspace index management view showing stable workspace identity, root availability, phase, file/chunk counts, failures, redactions, and last update.
- [x] 8.2 Add accessible controls for enablement, selected roots, languages, exclusion patterns, maximum size, refresh, rebuild, disable, and confirmed deletion.
- [x] 8.3 Add the external embedding estimate and confirmation dialog with provider, model, exact chunk inputs, and estimated batch requests.
- [x] 8.4 Extend the OnePiece retrieval section with aggregate workspace-code status while retaining global provider/model and agent-memory status semantics.
- [x] 8.5 Add bounded progress presentation for scanning, parsing, confirmation, embedding, cancellation, ready, degraded, and unavailable states without unsupported ETA claims.
- [x] 8.6 Add component tests for configuration validation, phase transitions, confirmation, rebuild/delete isolation, unavailable roots, and Web/mock behavior.
- [x] 8.7 Add Playwright coverage for enabling a mock workspace, filtering files, confirming embedding, observing progress, searching a scoped hit, and deleting only that index.

## 9. Logging, Security, and Documentation

- [x] 9.1 Route code-index diagnostics through unified logging with workspace ids, phases, counts, durations, model ids, and safe reason categories only.
- [x] 9.2 Add sentinel tests proving unified logs and telemetry omit raw code, queries, credentials, detected secret values, absolute/private paths, and provider bodies.
- [x] 9.3 Add audit API tests proving only bounded workspace-relative metadata is returned and cross-workspace audit access is rejected.
- [x] 9.4 Update native context and user documentation for code-index configuration, external-provider privacy, supported languages, retention, rebuild, and deletion.

## 10. Full Verification

- [x] 10.1 Run `npm run lint:ci`, `npm run test`, `npm run test:coverage`, `npm run coverage:policy:test`, `npm run version:unit:test`, and `npm run contracts:check`.
- [x] 10.2 Run `npm run build` and `npx playwright test`.
- [x] 10.3 Run `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`.
- [x] 10.4 Run `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`.
- [x] 10.5 Run `cargo test --manifest-path src-tauri/Cargo.toml` and `cargo check --manifest-path src-tauri/Cargo.toml`.
- [x] 10.6 Run `openspec validate workspace-code-indexing-foundation --strict` and `openspec validate --specs --strict`.
