## Context

See `proposal.md` for motivation. Existing Skill management supports local import and two legacy management scopes. Planned effective resolution adds a `Project > User > Registry > System` order, immutable effective revisions, Overlay governance, configuration, and sandboxed tools. VaneHub already has Rust-owned filesystem/SQLite transactions, credential storage, application proxy settings, shared task/log patterns, and frontend service adapters, but no remote Skill repository protocol.

The third layer is named Registry because it describes installed package provenance and precedence, not a live network mount. Network unavailability must never make an already verified immutable package disappear.

## Goals / Non-Goals

**Goals:**

- Bootstrap and rotate registry trust without allowing an endpoint to self-authorize.
- Discover and install exact publisher-authorized package versions through a bounded pipeline.
- Preserve current working revisions across failed updates and retain verified rollback evidence.
- Separate distribution provenance from all runtime authority and user-owned customization.
- Provide explicit revocation containment, offline behavior, and operator-visible recovery.

**Non-Goals:**

- Implementing the public registry server, publisher portal, billing, ratings, comments, telemetry, or recommendation ranking.
- Installing directly from arbitrary Git URLs, branches, local command output, or unsigned archives through the Registry workflow.
- Resolving inter-Skill dependency graphs or automatically installing dependencies in the first release.
- Automatically installing updates or silently switching publishers/Skill identities.
- Treating a verified package as safe to execute, auto-evolve, access secrets, or bypass permissions.

## Decisions

### 1. Use repository-security metadata rather than a signed flat index

Each source follows a TUF-style role model: locally trusted root; short-lived freshness metadata; snapshot metadata binding delegated role versions; and target/delegation metadata authorizing package targets. Threshold signatures, monotonic versions, expiry, target hashes/lengths, consistent snapshots, and namespace delegation address key compromise, rollback, freeze, and mix-and-match risks that a single signed index does not.

The implementation uses a maintained Rust repository-security library only after test-vector, license, advisory, and platform review. The application domain depends on a verification port rather than library types. First-party root metadata ships with VaneHub. Custom sources require the user to provide root metadata or a fingerprint through a channel independent of the endpoint. Trust-on-first-use from downloaded metadata is rejected.

Initial signature support is whatever reviewed algorithms the chosen implementation securely supports; product metadata declares algorithm/key ids and thresholds rather than embedding an application-specific signature format. Root rotation follows the repository protocol's old-root/new-root verification chain.

### 2. Separate source, package, Skill, publisher, and snapshot identities

Stable internal keys are tuples, not display strings:

- source id identifies the configured trust domain;
- publisher namespace is authorized by delegation;
- package id and semantic version select a target;
- package manifest declares one stable Skill id in the first release;
- content hash and installed snapshot id bind local bytes.

One-package/one-Skill simplifies precedence, uninstall, revocation, and rollback. Package bundles and dependencies are deferred. A publisher cannot claim an id outside its delegated namespace. Changing source or publisher creates a different provenance lineage even if display names match.

### 3. Store installed Registry packages as immutable content snapshots

The native layer owns three separated roots under application data: content-addressed download cache, quarantine/staging, and installed snapshots. Active records point to an immutable snapshot; files are never edited in place. Installation creates a unique staging directory, verifies/extracts/validates, fsyncs or uses the platform-equivalent durable replacement strategy, publishes by atomic rename on the same volume, and commits SQLite active/rollback state through the existing compensated transaction pattern.

On failure the prior pointer and directory remain intact. Orphan staging and unreferenced published snapshots are reconciled at startup using transaction journals. Destructive cleanup resolves and verifies every application-owned target before removal.

Registry base content is read-only. Overlay customizes without changing provenance. Forking copies validated content into User or Project scope with new local provenance and no inherited publisher/tool trust.

### 4. Bound the archive and reject ambiguous filesystem semantics

The first release uses one archive format supported by a memory-safe reviewed Rust parser, selected during implementation based on deterministic cross-platform path handling. Defaults are centrally configured: 16 MiB compressed package, 64 MiB expanded content, 512 files, 8 MiB per file, nesting depth 8, path length 240, and compression ratio 100:1; registry policy can tighten but not widen application ceilings.

Extraction normalizes separators and Unicode, performs Windows case-fold/reserved-name/alternate-stream checks even on other development platforms, and rejects absolute/traversal paths, duplicate normalized targets, symlinks, hardlinks, devices, sparse surprises, and unknown top-level content. `SKILL.md` is required. Known support directories and manifest files are allowlisted. Executable content is accepted only in the separately specified bounded Skill tool formats and remains non-executable after install until independently trusted.

### 5. Bind preview and confirmation to immutable witnesses

An install-plan witness hashes source trust state, metadata versions, target identity/hash/length, installed state, effective-resolution impact, package-manifest summary, normalized permission manifest and version-to-version authority diff, and requested action. Confirmation supplies that witness. The service refreshes/validates critical state before mutation; any mismatch returns stale-preview instead of silently acting on a changed target. Every install reviews requested authority. An update that expands filesystem, process, network, secret, or resource authority always requires a fresh explicit confirmation and never inherits a prior approval.

Install/update/downgrade/rollback/uninstall are backend-managed cancellable operations with progress stages. Update checks fetch only bounded metadata using conditional requests and jittered schedules. Package bodies download only after explicit confirmation. No automatic package install is included.

### 6. Treat provenance, content risk, and runtime trust as separate gates

Repository verification answers “was this exact target authorized by this configured source and publisher?” Package validation answers “does it meet structural and compatibility constraints?” Neither answers “may it execute or access resources?”

- Overlay remains imported/untrusted until its own policy promotes it.
- Skill configuration records are user-owned and never sourced from a package beyond schema defaults.
- Bundled WASM/declarative tools require exact-revision tool trust and normal permissions.
- Evolution suggestions from Registry content pass the same evidence and Curator gates; registry signatures do not authorize auto-apply.
- Forks begin as local packages and lose Registry publisher trust.

Install preview displays each gate separately to prevent one green badge from implying universal safety.

### 7. Make revocation signed, explicit, and fail closed for new activation

Target custom metadata carries revocation state, severity, reason code, replacement constraints, and timestamp. Only currently verified metadata can add or clear revocation. Critical security revocation marks the installed snapshot runtime-ineligible, prevents new Role/Utility loads and bundled tool calls, invalidates pending not-yet-started work, and signals active owners according to cancellation policy. It does not delete files, Overlay, config, or history.

Recovery options are filtered to currently authorized non-revoked targets: update, retained rollback, uninstall, or rely on an already-existing higher-priority local override. The system never auto-selects another version. Offline operation retains the last verified revocation state; stale freshness is visible. This balances availability with evidence-based containment.

### 8. Use bounded cache with strict offline authorization

Downloaded objects are addressed by target digest and cannot be considered installed from their filename or URL. Cache metadata records last access, source references, verified-at state, quarantine reason, and retention. Eviction uses a bounded quota and excludes active snapshots, transaction-referenced objects, and the configured rollback candidate.

Installed snapshots work offline because they were verified and copied into the installed root. Cached catalogs may be displayed with stale timestamps. New state-changing operations require a currently valid metadata chain; the presence of cached bytes alone is insufficient. This avoids an “offline mode” that silently disables expiry protection.

### 9. Network behavior reuses the VaneHub client boundary

All source and package requests use the Rust-managed network adapter with current proxy/bypass settings, HTTPS, DNS/connection/read/total timeouts, streamed byte limits, cancellation, conditional headers, and a strict user agent. Redirects are disabled by default; source policy can allow a bounded same-origin or explicitly metadata-authorized content origin. Authorization headers never cross origin. Query strings and response bodies are excluded from persistent logs.

Registry credentials use the operating-system credential store with preserve/replace/clear semantics. SQLite and DTOs carry only presence/error state. Browser fetch is not used by React.

### 10. Integrate as a tooling subdomain through published ports and service adapters

Rust adds `contexts/tooling/skill_registry` as an independent tooling subdomain for source/trust/catalog/package/install/revocation/cache models and adapters for repository verification, network, credential store, archive extraction, filesystem transaction, SQLite, tasks, and unified logs. It publishes a narrow API/port to effective Skill resolution and the sandboxed Skill-tool runtime; consumers never import its repositories or infrastructure, and prompt assembly never performs network access.

The Registry consumes the normalized permission-manifest contract owned by the Skill tool runtime and persists only its canonical digest plus bounded review projection. It does not evaluate operational permissions or create grants. This preserves `tooling` as the existing bounded context and avoids an unregistered peer context or a second permission architecture.

Tauri commands expose source lifecycle, catalog query/detail, refresh, operation preview/start/cancel/status, integrity recheck, and cache/recovery actions. TypeScript contracts extend `AgentService`; `tauri-agent-client.ts` owns native invocation. `web-agent-client.ts` can expose deterministic catalog fixtures or a future secure HTTP service but reports native install/cache/credential actions as unsupported.

### 11. Add Registry as a Skills-page sub-surface

The Skills page gains Installed and Registry views rather than a separate top-level settings route. Registry includes source selector/health, search/filter, cards, details/version history, source management, immutable preview dialogs, progress drawers, revocation banners, and recovery. Installed Skill details show provenance, active/rollback versions, integrity, source health, shadowing, separate trust states, and user-owned retained data.

All remote display strings are sanitized plain text with length limits. Explicit links use the safe opener after scheme/origin validation. Components remain below 300 lines and preserve keyboard, focus, screen-reader, non-color, and narrow-layout behavior.

## Risks / Trade-offs

- [Repository-security metadata and key rotation are operationally complex] → Use a reviewed implementation, published runbooks/test vectors, first-party pinned roots, and explicit custom-source bootstrap.
- [A compromised publisher key can authorize malicious content in its namespace] → Use delegated namespaces/thresholds, short-lived metadata, revocation, structural validation, separate runtime trust, and visible publisher provenance.
- [Strict metadata expiry reduces offline install availability] → Keep installed snapshots usable and stale catalogs inspectable; do not weaken authorization for new mutations.
- [Archive parsing expands the attack surface] → Support one reviewed format, stream with hard limits, reject ambiguous entries, fuzz parsers, and extract only into quarantine.
- [Critical revocation can interrupt workflows] → Apply it only from verified metadata, preserve evidence/data, communicate severity, and provide verified recovery without silent version switching.
- [Immutable snapshots consume disk] → Bound package/cache sizes and retained rollback versions, deduplicate cached objects, and expose safe cleanup.
- [Custom registries can disappear or rotate ownership] → Preserve installed provenance/evidence, require explicit root changes, and never rebind package lineage by display name.

## Migration Plan

1. Add source/trust/metadata/cache/install persistence and read-only first-party catalog refresh behind a disabled feature flag.
2. Integrate reviewed repository verification, adversarial metadata fixtures, proxy-aware network transport, and credential isolation.
3. Add quarantine download, bounded extraction/package validation, install preview, and immutable staging without effective-runtime activation.
4. Add normalized permission review and update-diff witnesses, then enable atomic Registry-layer installation and effective resolution, update/downgrade/rollback/uninstall, and integrity reconciliation.
5. Add UI source/catalog/version governance, operation progress, offline states, revocation containment, and recovery.
6. Roll out custom sources only after trust-bootstrap and rotation tests pass; retain a global registry-network/install kill switch.
7. Roll back by disabling refresh and new mutations while leaving verified installed snapshots and evidence available; critical locally recorded revocations remain enforced until explicitly repaired by verified metadata or uninstall.
