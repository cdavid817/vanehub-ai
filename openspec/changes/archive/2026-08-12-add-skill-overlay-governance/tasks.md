## 1. Prerequisite and Domain Foundation

- [x] 1.1 Verify `establish-effective-skill-runtime` is implemented and its effective package snapshot, logical resources, pin state, usage sidecars, and full validation suite pass before starting Overlay code.
- [x] 1.2 Add failing Rust domain tests for Overlay scope, trust, mutation states, conflict states, base witnesses, revisions, and prior-revision hashes.
- [x] 1.3 Implement validated `OverlayDocument`, `OverlayPatch`, `OverlayLearnBlock`, `OverlayFile`, `OverlayConflict`, and `OverlayTrust` domain types without `unwrap()` or `expect()` in production code.
- [x] 1.4 Add state-transition tests for active, disabled, reverted, trusted, untrusted, active-conflict, resolved, and ignored states.
- [x] 1.5 Implement domain transition rules, including immutable ids, monotonic revisions, trust-by-revision, and retained audit identity.
- [x] 1.6 Define bounded application models and errors for summaries, details, previews, diffs, resources, history, mutations, imports, promotion, pinned refusal, stale witnesses, limits, integrity, and reconciliation.

## 2. Deterministic Replay Engine

- [x] 2.1 Add exact-patch tests for unique replacement, zero matches, multiple matches, `replace_all`, Unicode content, newline differences, ordering, disable, and revert.
- [x] 2.2 Implement pure exact patch replay with no whitespace normalization or fuzzy matching.
- [x] 2.3 Add learned-guidance tests for one stable delimiter, scope ordering, block ordering, disable, revert, and delimiter-injection refusal.
- [x] 2.4 Implement learned-guidance rendering after successful patch replay without modifying stored base text.
- [x] 2.5 Add resource replay tests for System→User→Project precedence, logical-path shadowing, workspace isolation, media types, disable, and revert.
- [x] 2.6 Implement Overlay resource merging over the effective package snapshot with bounded shadow summaries.
- [x] 2.7 Add scope-chain tests proving one failed scope rolls back completely, blocks dependent higher scopes, and preserves the last healthy lower-scope hash.
- [x] 2.8 Implement deterministic per-scope replay results containing base, intermediate, final, blocked, conflict, and integrity states.
- [x] 2.9 Add property-oriented tests proving identical snapshots and Overlay revisions always produce identical effective content, resources, hashes, and conflicts.

## 3. Content and Path Security

- [x] 3.1 Define shared limits for instruction mutations, mutation counts, path length and depth, archive entries, per-file size, expanded import size, and active history segments.
- [x] 3.2 Add scanner tests for private keys, credential structures, prompt-authority overrides, script markup, delimiter forgery, and safe literal guidance.
- [x] 3.3 Implement versioned deterministic text scanning that returns safe rule ids without logging matched content.
- [x] 3.4 Add Windows and cross-platform path tests for absolute paths, parent traversal, hidden components, reserved devices, alternate streams, unsupported top-level directories, and escaping links.
- [x] 3.5 Implement canonical Overlay path validation for `references`, `templates`, and `assets` before filesystem access.
- [x] 3.6 Add media validation tests for prohibited extensions, disguised executable signatures, valid UTF-8 documents, and permitted bounded binary assets.
- [x] 3.7 Implement deny-first extension, media-type, and magic-signature validation; keep binary assets unavailable to text-only reads.
- [x] 3.8 Route all Overlay validation and refusal diagnostics through unified logging with redacted identities, paths, hashes, sizes, and rule ids only.

## 4. Overlay Storage and History

- [x] 4.1 Define application ports for Overlay manifests, content-addressed payloads, effective package snapshots, scanner, pin state, usage state, history, clock, logging, and transaction execution.
- [x] 4.2 Add storage-layout tests for System, User, and canonical-workspace Project manifests, payload roots, history roots, and reserved-directory exclusion.
- [x] 4.3 Implement versioned JSON manifest parsing and serialization with unsupported-future-version refusal.
- [x] 4.4 Implement content-addressed payload staging, hash verification, reference tracking, and recovery-safe orphan cleanup.
- [x] 4.5 Add append-only history tests for event fields, event hash linkage, segment linkage, 4 MiB rollover, pagination, tampering, truncation, and missing segments.
- [x] 4.6 Implement ordered JSONL history segments with verification and bounded reads; never silently repair unverifiable history.
- [x] 4.7 Add transaction recovery tests for interruption before payload staging, manifest swap, history append, usage update, commit marker, and cleanup.
- [x] 4.8 Extend Skill filesystem transactions so manifest, payload reachability, history event, and usage counters commit or roll back together.
- [x] 4.9 Add per-Overlay locking and revision recheck tests for concurrent local callers.

## 5. Preview, CAS, and Manual Mutations

- [x] 5.1 Implement one pure preparation pipeline shared by preview and commit for identity, scope, pin, limits, scanning, witnesses, tentative revision, replay, and diff.
- [x] 5.2 Add preview/commit parity tests proving the same witnesses produce the same replay result and stale live state forces re-preview.
- [x] 5.3 Implement Overlay query, effective diff, and non-persisting preview operations in the Skill application service.
- [x] 5.4 Add failing CAS tests for stale Overlay revision, changed base instruction hash, changed package hash, changed payload, and changed pin state.
- [x] 5.5 Implement exact patch creation, disable, and revert using revision and base witnesses.
- [x] 5.6 Implement learned-guidance creation, disable, and revert using the same validation and transaction path.
- [x] 5.7 Implement supporting-file add, replace, disable, and revert with staged content-addressed payloads.
- [x] 5.8 Enforce pinned refusal for every mutation before durable staging while continuing to replay already committed healthy revisions.
- [x] 5.9 Increment `patch_count` and `overlay_mutation_count` transactionally for successful operations only and add rollback consistency tests.

## 6. Import Quarantine and Trust

- [x] 6.1 Add bounded import tests for compressed size, expanded size, entry count, duplicate paths, traversal, links, unsupported versions, excessive mutations, file limits, and partial extraction cleanup.
- [x] 6.2 Implement the ZIP v1 `overlay.json` plus `payloads/sha256/<hash>` import profile in an isolated staging directory, enforce exact manifest-to-payload closure, normalize imported trust, and complete scanning before durable untrusted state is created.
- [x] 6.3 Add tests proving imported Overlays remain excluded from instructions and resources before promotion.
- [x] 6.4 Implement untrusted import detail and diff queries with safe source summaries, hashes, scan versions, mutations, resources, and conflicts.
- [x] 6.5 Add trust-promotion tests for exact reviewed revision and document hash, changed import, changed base, changed scan result, and pinned target.
- [x] 6.6 Implement explicit promotion that reruns validation and replay and trusts only the reviewed revision.
- [x] 6.7 Verify no operation or UI contract can bypass a hard-deny scan result with a force or trust-anyway flag.

## 7. Base Drift and Reconciliation

- [x] 7.1 Add drift tests for unchanged base, instruction-only changes, resource-only changes, changed effective layer, clean replay, patch conflicts, resource conflicts, earlier-scope failure, and integrity failure.
- [x] 7.2 Implement `base_hash_changed`, `needs_reconcile`, blocked-scope, and last-healthy-effective state calculation without updating witnesses.
- [x] 7.3 Add reconciliation preview models containing witnessed base, current base, proposed effective result, per-conflict choices, and final complete diff.
- [x] 7.4 Implement reconciliation by editing a mutation, disabling an ignored mutation, and confirming a clean replay against current witnesses.
- [x] 7.5 Append resolved and ignored conflict history without deleting prior conflict or mutation records.
- [x] 7.6 Add stale reconciliation tests proving UI edits can be retained by the caller but no stale result commits.
- [x] 7.7 Verify a pinned Skill reports drift but refuses reconciliation until explicitly unpinned.

## 8. Effective Runtime Integration

- [x] 8.1 Add integration tests proving list preview, eager prompt, on-demand load, resource read, and CLI-derived mount share the same Overlay-applied effective hash.
- [x] 8.2 Route effective Skill snapshot construction through Overlay discovery and replay after four-layer base resolution.
- [x] 8.3 Update eager API-agent prompt assembly to consume the last healthy Overlay result while preserving existing character budgets and usage semantics.
- [x] 8.4 Update `load_skill` and `read_skill_resource` to consume the same Overlay-applied instructions, logical resources, media rules, and revision witnesses.
- [x] 8.5 Update CLI-derived Skill mount/cache materialization to use healthy effective content without exposing Overlay storage or mutable payload paths.
- [x] 8.6 Add regression tests proving untrusted, conflicted, invalid, or blocked Overlays never reach any Agent and emit only redacted unified diagnostics.
- [x] 8.7 Invalidate effective catalog and derived mount caches after committed Overlay mutations, trust changes, and reconciliation.

## 9. Native API and Frontend Service Contracts

- [x] 9.1 Add Tauri commands for Overlay summary, detail, preview, history, patch, guidance, file, import, promotion, disable, revert, and reconciliation operations with structured errors.
- [x] 9.2 Extend shared TypeScript Skill contracts and `agent-service.ts` with typed Overlay models and methods without `any` or direct component invocation.
- [x] 9.3 Update `tauri-agent-client.ts` as the only frontend native invocation boundary and add payload-mapping tests for every Overlay operation and error.
- [x] 9.4 Implement `web-agent-client.ts` Overlay state simulation for revisions, stale witnesses, trust quarantine, pinning, conflicts, history verification, and revert.
- [x] 9.5 Add adapter-parity contract tests proving equivalent Tauri payloads and Web/mock scenarios produce the same frontend shapes and state transitions.

## 10. Per-Skill Overlay UI

- [x] 10.1 Add an Overlay area to Skill details with status, active scopes, base/effective hashes, trust, pinned state, mutation counts, conflicts, and resource shadowing.
- [x] 10.2 Add bounded base-versus-effective and per-scope diff views with clear base layer and Overlay scope labels.
- [x] 10.3 Implement exact-patch and learned-guidance dialogs that require a successful preview before commit and preserve input on validation or stale errors.
- [x] 10.4 Implement supporting-resource management with allowed-directory guidance, media and size feedback, shadowing, preview metadata, disable, and revert.
- [x] 10.5 Implement import review and trust-promotion UI showing safe source metadata, scan version, hashes, diff, files, conflicts, and exact reviewed witnesses.
- [x] 10.6 Implement a three-way reconciliation workspace with per-conflict edit or ignore choices and a required final effective preview.
- [x] 10.7 Implement a paginated verified history timeline with action, actor, scope, revision transition, trust, conflict, timestamp, safe diff summary, and revert controls.
- [x] 10.8 Make every Overlay action read-only for pinned Skills while keeping the committed effective content, details, and history visible.
- [x] 10.9 Add localized text for base packages, Overlay scopes, trust quarantine, pinned refusal, scanning, stale witnesses, reconciliation, blocked scopes, and history integrity.
- [x] 10.10 Add component and interaction tests for System bases, no-Overlay empty state, multiple scopes, untrusted imports, stale forms, conflicts, rollback, responsive layouts, and keyboard accessibility.
- [x] 10.11 Keep all new production TS/TSX files within the 300-line rule by splitting state hooks, diff panels, mutation dialogs, reconciliation, resources, and history into focused modules.
- [x] 10.12 Run `npx playwright test` and resolve Overlay workflow regressions in both desktop-equivalent and Web/mock UI behavior.

## 11. Verification and Documentation

- [x] 11.1 Document Overlay scope versus Skill layer, storage boundaries, trust, replay fallback, pinning, limits, and recovery without external-product comparisons or deferred self-evolution claims.
- [x] 11.2 Run `npm run lint:ci`.
- [x] 11.3 Run `npm run test` and `npm run test:coverage`.
- [x] 11.4 Run `npm run coverage:policy:test`, `npm run version:unit:test`, and `npm run contracts:check`.
- [x] 11.5 Run `npm run build`.
- [x] 11.6 Run `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`.
- [x] 11.7 Run `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`.
- [x] 11.8 Run `cargo test --manifest-path src-tauri/Cargo.toml` and `cargo check --manifest-path src-tauri/Cargo.toml`.
- [x] 11.9 Run `openspec validate add-skill-overlay-governance --strict` and `openspec validate --specs --strict`.
