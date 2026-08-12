## 1. Security protocol and domain contracts

- [ ] 1.1 Add the `skill_registry` Rust context with source, trusted root, publisher namespace, package/Skill/version identity, target, catalog, provenance, snapshot, compatibility, operation, revocation, cache, quarantine, and recovery models.
- [ ] 1.2 Define ports for repository metadata verification, network transport, credential storage, archive inspection/extraction, package validation, filesystem transaction, SQLite state, effective Skill publication, tasks, clocks, and unified logs.
- [ ] 1.3 Evaluate and pin maintained repository-security, cryptographic, archive, and semantic-version dependencies after license, advisory, algorithm, test-vector, Windows support, and maintenance review.
- [ ] 1.4 Define stable shared contracts for source ids, publisher namespaces, target custom metadata, one-package/one-Skill manifests, revocation, immutable preview witnesses, and error codes.
- [ ] 1.5 Add canonical fixtures for first-party/custom roots, rotations, delegations, packages, revocations, and every supported metadata version.

## 2. Registry source trust and credentials

- [ ] 2.1 Add SQLite migrations/repositories for sources, highest verified metadata versions, expiry/freshness, root fingerprints, credential presence, health, refresh witnesses, and redacted errors.
- [ ] 2.2 Seed the first-party source with build-pinned root metadata and verify startup adoption without replacing user source state.
- [ ] 2.3 Implement custom-source preview/add/update/enable/disable/remove with HTTPS validation and independently supplied root trust; reject endpoint-provided trust-on-first-use.
- [ ] 2.4 Implement old-root/new-root threshold rotation and tests for valid rotation, skipped versions, unknown keys, insufficient thresholds, key rollback, and endpoint root substitution.
- [ ] 2.5 Store authenticated-source credentials only through the operating-system credential store with preserve/replace/clear and compensation; expose only configured/missing/error state.
- [ ] 2.6 Add source lifecycle tests for concurrent edits, stale witnesses, source disablement with installed packages, credential failures, and removal eligibility.

## 3. Metadata verification and catalog

- [ ] 3.1 Implement freshness, snapshot, targets, and delegated publisher metadata refresh with threshold signatures, versions, expiry, hashes, lengths, consistent snapshots, and namespace constraints.
- [ ] 3.2 Persist highest verified versions and reject rollback, freeze, mix-and-match, delegation escape, target substitution, expired chains, and incomplete refreshes without replacing prior verified state.
- [ ] 3.3 Implement bounded conditional refresh with jitter/backoff, pagination, search, filters, details, version history, compatibility, risk, content categories, installed/update, and revocation projections.
- [ ] 3.4 Sanitize all publisher-controlled display text, cap lengths, reject control characters in identifiers, and route validated explicit links through the safe opener.
- [ ] 3.5 Add offline/stale catalog reads with last-verified metadata and tests proving cached display state cannot authorize a mutation.

## 4. Network transport and quarantine cache

- [ ] 4.1 Implement the Rust-managed registry HTTP adapter using active application proxy/bypass settings, HTTPS, DNS/connect/read/total timeouts, streamed size limits, cancellation, and conditional requests.
- [ ] 4.2 Disable redirects by default and implement bounded same-origin or metadata-authorized content-origin redirects without forwarding credentials across origins.
- [ ] 4.3 Stream targets into unique application-owned quarantine files while hashing and enforcing authorized length; handle cancellation, partial objects, retry, and cleanup safely.
- [ ] 4.4 Implement the content-addressed cache with atomic object publication, metadata references, quota/LRU state, active/rollback exclusions, quarantine reasons, and startup reconciliation.
- [ ] 4.5 Add network tests for proxy application, bypass, redirect abuse, credential stripping, slow responses, truncated/oversized bodies, hash mismatch, cancellation, concurrent downloads, and offline behavior.

## 5. Archive and package validation

- [ ] 5.1 Select one reviewed archive format and implement streaming inspection/extraction only into unique quarantine staging directories.
- [ ] 5.2 Enforce 16 MiB compressed, 64 MiB expanded, 512-file, 8 MiB per-file, depth-eight, path-240, and 100:1 ratio ceilings, allowing source policy only to tighten them.
- [ ] 5.3 Reject absolute/traversal paths, normalized duplicate targets, Unicode/case collisions, Windows reserved names, alternate streams, links, devices, sparse surprises, unsupported entry kinds, and unknown top-level content.
- [ ] 5.4 Validate required `SKILL.md`, standard metadata, one-package/one-Skill identity, source/publisher namespace, compatibility, allowed support directories, configuration schema, tool manifests, and complete content hashes.
- [ ] 5.5 Record bundled executable-tool presence and hashes without registering or trusting them during package installation.
- [ ] 5.6 Add fuzz/adversarial fixtures for decompression bombs, malformed archives, path ambiguity, parser crashes, package/metadata identity mismatch, and resource-limit exhaustion.

## 6. Immutable installation transactions

- [ ] 6.1 Add application-owned installed snapshot, staging, journal, and rollback directory policies with explicit resolved-path containment checks before cleanup.
- [ ] 6.2 Add SQLite migrations/repositories for installed lineage, active and retained rollback snapshots, metadata/content witnesses, validation, integrity, compatibility, revocation, and transaction state.
- [ ] 6.3 Implement immutable install preview for install/update/downgrade/rollback/uninstall, including source, publisher, ids, versions, hashes, sizes, changes, trust consequences, risk, compatibility, shadowing, and retained user state.
- [ ] 6.4 Bind confirmation to a witness over metadata, target, package summary, installed state, and action; reject stale previews before download or mutation.
- [ ] 6.5 Implement atomic staging publication plus database active-pointer commit with compensation and startup recovery for every partial filesystem/database failure point.
- [ ] 6.6 Publish eligible installed definitions into the Registry layer and atomically refresh `Project > User > Registry > System` effective resolution.
- [ ] 6.7 Keep in-flight contexts pinned to immutable snapshots, retire unreferenced snapshots under retention policy, and verify concurrent refresh/install/load behavior.

## 7. Version lifecycle, integrity, and revocation

- [ ] 7.1 Implement background metadata-only update checks that never download package bodies or install versions automatically.
- [ ] 7.2 Implement explicit compatible update, selected downgrade, retained rollback, and uninstall without silently crossing source, publisher namespace, package id, or stable Skill id.
- [ ] 7.3 Preserve the prior active snapshot on verification, validation, publication, persistence, cancellation, or post-install integrity failure.
- [ ] 7.4 Recheck installed content against retained target and extracted manifests; treat drift as ineligible and offer verified reinstall/rollback/uninstall rather than adoption.
- [ ] 7.5 Implement signed revocation ingestion with severity/reason/replacement guidance and critical fail-closed eligibility for new Role, Utility, configuration, and bundled-tool activation.
- [ ] 7.6 Cancel pending not-yet-started work on critical revocation, retain user-owned data/evidence, filter recovery to verified non-revoked targets, and never auto-switch versions.
- [ ] 7.7 Test revocation apply/clear, offline stale freshness, higher-priority shadowing, active contexts, pending approvals, missing recovery versions, and source disablement.

## 8. Trust-domain and user-data boundaries

- [ ] 8.1 Keep registry provenance status independent from Overlay trust, configuration values, permission grants, approval decisions, executable-tool trust, and evolution auto-apply.
- [ ] 8.2 Require exact-revision Skill tool trust after installing a package containing declarative or WASM tools and test that provenance cannot bypass permission evaluation.
- [ ] 8.3 Implement Overlay customization against immutable Registry bases while preserving base hashes, reconciliation evidence, and update previews.
- [ ] 8.4 Implement explicit fork to User/Project scope with new local provenance and no inherited publisher authorization or executable trust.
- [ ] 8.5 Preserve forks, Overlay/history, configuration, usage, audit, and unrelated cache data on uninstall; expose separately authorized cleanup actions.
- [ ] 8.6 Filter credentials, tokens, query strings, response bodies, sensitive paths, package payloads, and user-owned values from logs, tasks, UI DTOs, evidence, and evolution signals.

## 9. Commands and frontend adapters

- [ ] 9.1 Add Rust/Tauri commands for source lifecycle, catalog query/detail, refresh, preview, operation start/cancel/status, integrity recheck, quarantine, cache cleanup, revocation status, and recovery with mapped errors.
- [ ] 9.2 Register commands and extend generated/shared TypeScript contracts for trust roots, catalog/version/provenance, previews, tasks, installed/rollback state, revocation, cache, integrity, and runtime support.
- [ ] 9.3 Extend `AgentService` and `tauri-agent-client.ts` with registry operations, keeping native invocation out of React components.
- [ ] 9.4 Extend `web-agent-client.ts` with deterministic/remote-ready catalog inspection and explicit unsupported local trust, credential, cache, extraction, and install behavior.
- [ ] 9.5 Add adapter contract tests for pagination, stale responses, source switching, cancellation, redaction, operation resumption, and Web/native capability differences.

## 10. Registry and installed-Skill UI

- [ ] 10.1 Add Installed and Registry sub-views to the Skills page with source selector/health, bounded search/filter, catalog cards, provenance, compatibility, content categories, installed/update/shadowed/revoked states, and sanitized details.
- [ ] 10.2 Add custom-source governance for bootstrap fingerprint confirmation, credentials, enable/disable, refresh, verified rotation, authentication repair, and eligible removal.
- [ ] 10.3 Add install/update/downgrade/rollback/uninstall previews showing immutable witnesses, exact changes, effective-layer impact, separate trust gates, retained state, risks, and stale-preview recovery.
- [ ] 10.4 Add cancellable progress and redacted operation logs for refresh, download, validation, installation, rollback, uninstall, integrity, cache, and recovery without blocking navigation.
- [ ] 10.5 Add persistent critical-revocation warnings and verified recovery choices in catalog, installed details, and relevant activation surfaces even when shadowed.
- [ ] 10.6 Show offline/stale metadata, source-disabled, integrity-drift, quarantined, recovery-required, and unsupported Web states without fabricating success.
- [ ] 10.7 Keep production components below 300 physical lines and add keyboard, focus, screen-reader, non-color status, safe-link, sanitized-text, narrow-layout, and no-overflow tests.
- [ ] 10.8 Add Playwright flows for browse/install, custom source, credential repair, update failure preservation, rollback, uninstall retention, offline catalog, critical revocation, integrity drift, stale preview, cancellation, and Web unsupported behavior.

## 11. Audit, operations, and rollout safety

- [ ] 11.1 Emit redacted unified-log events with correlation, source/package/version/snapshot ids, metadata versions, hashes, stages, outcome, duration, byte counts, revocation, and recovery—never credentials or response/package bodies.
- [ ] 11.2 Reuse backend-managed tasks for long operations and persist enough bounded state to resume observation or reconcile after application restart.
- [ ] 11.3 Add global registry network and mutation kill switches that preserve installed eligible snapshots and critical locally recorded revocations.
- [ ] 11.4 Add safe startup reconciliation for partial downloads, quarantine, transaction journals, orphan snapshots, cache references, active pointers, and rollback retention.
- [ ] 11.5 Document first-party root bootstrap/rotation, publisher delegation, incident revocation, recovery, custom-source trust, cache cleanup, and rollback runbooks.

## 12. Verification

- [ ] 12.1 Run `npm run lint:ci`, `npm run test`, `npm run test:coverage`, `npm run build`, `npm run coverage:policy:test`, `npm run version:unit:test`, and `npm run contracts:check`.
- [ ] 12.2 Run `npx playwright test` for Registry and installed-Skill UI behavior.
- [ ] 12.3 Run `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`, `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`, `cargo test --manifest-path src-tauri/Cargo.toml`, and `cargo check --manifest-path src-tauri/Cargo.toml`.
- [ ] 12.4 Run repository-security official test vectors, hostile metadata/archive corpus, fuzz targets, proxy/redirect tests, failure-injection transactions, offline/expiry, revocation, concurrency, and Windows path-semantics suites.
- [ ] 12.5 Run dependency license/advisory review and verify the Registry feature with network/mutation kill switches enabled and disabled.
- [ ] 12.6 Run `openspec validate add-remote-skill-registry-and-supply-chain-governance --strict` and `openspec validate --specs --strict`, then record implementation, root/protocol fixture versions, and rollback evidence before archive.
