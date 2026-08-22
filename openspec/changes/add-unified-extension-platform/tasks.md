## 0. Rebase, contract inventory, and implementation gates

- [x] 0.1 Read root `AGENTS.md`, `openspec/project.md`, every delta spec in this change, and the current main specs for all modified capabilities before changing code.
- [x] 0.2 Rebase or reconcile this change with active Skill Registry, Skill configuration, Utility delegation, Skill evolution, and Prompt Hooks changes; document any contract rename in `design.md` and rerun strict OpenSpec validation.
- [x] 0.3 Inventory current Rust APIs for `tooling::{extensions,mcp,plugin_integrations,prompt_hooks,skill_tools,skills}`, `permissions`, `agent_runtime`, and `communications`; record the published API/port that each adapter will call.
- [x] 0.4 Inventory current frontend service composition, Tauri/Web adapters, settings routes, operation polling, i18n locales, semantic tokens, and settings tests before selecting final filenames.
- [x] 0.5 Add the two-layer capability-gate mechanism described in `design.md`. No feature-flag mechanism exists in the repository today; this change does not create a general one.
  - [x] 0.5.1 Update `design.md`, the `extension-platform` delta spec, and this checklist to state the Build Capability versus Runtime Kill Switch distinction before writing code.
  - [x] 0.5.2 Add Cargo features `extension-wasm-module-runtime = ["skill-tool-module-runtime"]` and `extension-sidecar-runtime = []`, both off by default; reuse the existing Wasmtime module runtime without a second engine or the component model.
  - [x] 0.5.3 Add the closed `ExtensionPlatformFeature` enum and the five-state effective-status union `not_compiled | runtime_disabled | enabled | blocked_by_prerequisite | forced_disabled` in `tooling::extension_platform::domain`, with `FeatureUnavailableInBuild` as an explicit domain error.
  - [x] 0.5.4 Add an additive SQLite migration and repository storing desired state, revision, `updated_at`, `updated_by`, and optional reason only; derive build availability through `cfg!` and never persist it.
  - [x] 0.5.5 Add the application service, published API/port, and immutable cached snapshot; all seven gates start disabled, and missing rows, unknown gates, read failures, and parse failures fail closed to disabled.
  - [x] 0.5.6 Add audit records for every gate mutation with gate, prior/new desired state, revision, actor, and reason.
  - [x] 0.5.7 Add thin Tauri commands and the frontend service boundary for reading and setting gates; defer the full settings UI to Task Group 12.
  - [x] 0.5.8 Add tests for fail-closed defaults, enabling an uncompiled gate, the five-state union never merging `not_compiled` with `runtime_disabled`, stale-revision rejection, audit content, and cross-context access through the published API only.
- [x] 0.6 Define rollout gates in code/config so an incomplete later gate cannot become reachable merely because database migrations are present. Task Group 0 owns the gate contract itself — closing semantics, domain errors, state transitions, and audit — and maps enforcement to the groups that first have something to enforce against. Writing enforcement tests here would mean asserting against install, activation, registration, execution, drain, and quarantine machinery that does not exist yet.
  - [x] 0.6.1 Express sandbox self-test and adapter-parity preconditions as `blocked_by_prerequisite`, ordered below `runtime_disabled` so a gate nobody asked for is never reported as blocked.
  - [x] 0.6.2 State the closing contract in the `extension-platform` delta spec: disabling refuses new install, activation, registration, and execution at once; running work drains under the existing lifecycle policy; a sidecar is terminated after its safe-shutdown timeout; and re-enabling never reactivates a quarantined extension.
  - [x] 0.6.3 Cover the gate state machine itself: every transition among the five statuses, `FeatureUnavailableInBuild`, `StaleRevision`, fail-closed reads, degraded republication with last-known-good retention, and one audit record per accepted mutation and per degradation.
  - [x] 0.6.4 Record the enforcement mapping so no gate ships unverified: install/enable/activation gating verifies in Task Group 4 (4.11); registration and execution gating verifies in Task Group 5 (5.12); WASM and sidecar drain, termination, and crash accounting verify in Task Group 5 (5.13); quarantine and re-enable behavior verifies in Task Group 6 (6.9); existing-subsystem non-regression with every gate off verifies in Task Group 13 (13.11).
- [x] 0.7 Extend the architecture fitness harness to cover the new capability. `scripts/architecture/frontend-rules.mjs` already rejected direct React `invoke()`, and `src-tauri/tests/architecture.rs` already rejected cross-context private-module and concrete-persistence imports, but its `source_scope` resolved `contexts/<context>/<layer>` only. `tooling` nests one level deeper, so all nine of its subdomains — and the planned `permissions/rules` subdomain — were skipped rather than passing, and `extension_platform` would have been added straight into that blind spot.

  Fixed by the prerequisite change `fix-nested-context-architecture-enforcement`, which resolves both path shapes on either separator, repairs the two pre-existing violations it surfaced (`artifacts` now publishes `api.rs`; the `std::net` ban became a semantic rule that permits address value types and still forbids sockets), and adds no exemption list. `extension_platform`, `lifecycle_hooks`, `connectors`, and `permissions::rules` are enforced from their first commit.
- [x] 0.8 Validate the proposal with `openspec validate add-unified-extension-platform --strict` before implementation and after every delta-spec adjustment.

## 1. Extension domain contracts and manifest schema

Implemented in this order. Manifest parsing stays two-stage throughout — `Bounded YAML Parser -> BoundedYamlValue -> ExtensionManifestV1Decoder -> ExtensionManifestV1` — so no domain type is ever constructed from raw text.

- [x] 1.1 Add `src-tauri/src/contexts/tooling/extension_platform/` with `domain`, `application`, `infrastructure`, `api.rs`, and `mod.rs`, following existing DDD layout and visibility conventions. (Landed with the Task Group 0 capability gates.)

### 1.A Characterization tests before extraction

- [x] 1.A.1 Add characterization tests pinning the current accept and reject behavior of `skills/domain/config_document.rs`: every resource limit at and past its boundary, duplicate keys, each unsupported construct, indentation handling, and scalar/sequence/mapping shapes.
- [x] 1.A.2 Pin the exact diagnostic each rejection produces, so a relocation that changes what an operator is told is as visible as one that changes what is accepted.

### 1.B Extract the shared bounded YAML crate

- [x] 1.B.1 Add workspace member `crates/vanehub-bounded-yaml` containing only the restricted lexer, grammar, caller-supplied resource limits, duplicate-key detection, and a generic `BoundedYamlValue` AST. No I/O, no domain semantics, no serde derives that imply a schema.
- [x] 1.B.2 Reject anchors, aliases, merge keys, tags, multi-document streams, and every construct not explicitly supported. Do not add `serde_yaml`.
- [x] 1.B.3 Make `BoundedYamlLimits` a caller-supplied profile and give Skills and Extension Manifest separate profiles; a manifest needing more nodes SHALL NOT widen the Skill bound.
- [x] 1.B.4 Move `SkillConfigDocument` onto the shared parser while its domain decoding and validation stay owned by Skills. Re-run 1.A unchanged and prove identical behavior.
- [x] 1.B.5 Keep the dependency direction clean: Skills does not depend on `extension_platform`, and `extension_platform` does not import `skills` internals. Confirm with `cargo test --manifest-path src-tauri/Cargo.toml --test architecture`.

### 1.C SemVer dependency review

- [x] 1.C.1 Review a maintained `semver` crate for license, NOTICE obligations, advisory history, and maintenance status before any code depends on it; record the outcome.
- [x] 1.C.2 Add it as a workspace dependency with an explicit version, inherited by `src-tauri`. Do not hand-write version or requirement parsing.

### 1.D Manifest domain

- [ ] 1.D.1 Implement validated newtypes for extension id, publisher id, contribution local/global id, package hash, snapshot id, installation id, runtime generation id, operation witness, and activation event.
- [ ] 1.D.2 Implement `VersionedExtensionManifest::V1` and `ExtensionManifestV1` over those newtypes plus `semver::Version` and `semver::VersionReq`.
- [ ] 1.D.3 Implement manifest declarations for runtime, activation events, extension/Skill dependencies, requested capabilities, tools, Skills, MCP definitions, mode presets, Hooks, authorization rules, connectors, configuration schemas, and transforms.

### 1.E Explicit AST decoding

- [ ] 1.E.1 Implement `ExtensionManifestV1Decoder` reading `BoundedYamlValue` explicitly, field by field. No blanket deserialization: an unknown security-sensitive field must be a decision, not an omission.
- [ ] 1.E.2 Reject unknown fields, unsupported schema versions, incompatible application versions, and malformed shapes with stable per-field diagnostics.
- [ ] 1.E.3 Reject a schema version above the one this build supports as incompatible rather than guessing its security semantics.

### 1.F URL and origin validation

- [ ] 1.F.1 Validate requested network origins through `url`, requiring an explicit scheme and host and rejecting wildcards, userinfo, and non-origin forms.

### 1.G Portable package paths

- [ ] 1.G.1 Implement a `PortablePackagePath` value object that checks the raw string **before** `Path::components()` and rejects backslashes, NUL, absolute paths, drive prefixes, UNC prefixes, empty segments, `.`, `..`, and other non-portable forms. `Path::components()` treats a backslash as an ordinary filename character on Unix, so component analysis alone passes a Windows-shaped traversal on a Linux runner.
- [ ] 1.G.2 Reject an invalid path outright; never auto-normalize one into a valid-looking path.
- [ ] 1.G.3 Reject Windows reserved names, alternate data streams, case-fold collisions, and Unicode normalization collisions among declared paths.

### 1.H Contribution uniqueness and references

- [ ] 1.H.1 Namespace every external contribution as `ext::<extension-id>::<kind>::<local-id>` and reject an attempt to claim a non-namespaced native id.
- [ ] 1.H.2 Reject duplicate contribution ids within a manifest and prove every referenced path, schema, and handler resolves to a declared entry.

### 1.I Digest and fixtures

- [ ] 1.I.1 Implement a deterministic canonical manifest digest that does not depend on source key order or insignificant whitespace.
- [ ] 1.I.2 Add canonical valid and invalid manifest fixtures covering every contribution kind, minimum and maximum bounds, forward-version rejection, and deterministic serialization/digest.

### 1.J Bounded schema validator

- [ ] 1.J.1 Add a checked-in schema fixture for `vanehub-extension.yaml` validated by a bounded validator following `BoundedSkillToolSchemaValidator`; do not introduce a full `jsonschema` engine.
- [ ] 1.J.2 Fail closed on an unknown schema keyword rather than ignoring it, and test that Rust validation and the schema fixture accept and reject the same manifests.

### 1.K Invariant and failure tests

- [ ] 1.K.1 Add invariant, table-driven, and bounded-combinatorial tests for identifier normalization, contribution namespacing, SemVer requirements, URL/origin validation, path constraints, duplicate detection, and digest determinism. Do not add `proptest` or `quickcheck`, and do not describe an example test as property-based.
- [ ] 1.K.2 Define stable error codes/DTOs for manifest, compatibility, package, signature, dependency, lifecycle, runtime, contribution, Hook, rule, connector, and stale-witness failures, with a test proving every code is distinct.

## 2. Package security, publisher trust, and immutable storage

- [ ] 2.1 Extract shared safe-archive, path-normalization, and content-addressed-store primitives into an application-owned `platform` module and route `skills`, the future `skill_registry`, and `extension_platform` through it. The remote Skill Registry change has not started, so its primitives do not exist; the only current implementation is `skills/infrastructure/filesystem/overlay_import.rs`, which another context may not import directly.
- [ ] 2.2 Select and pin the ZIP, Ed25519, hashing, canonicalization, and semantic-version dependencies after license, advisory, maintenance, Windows, streaming-limit, and fuzz/test-vector review.
- [ ] 2.3 Implement streamed package hashing, signature-envelope parsing, canonical signed payload construction, publisher-key lookup, key revocation, and deterministic verification diagnostics.
- [ ] 2.4 Add trusted publisher-key repository/application services for list, preview add, add, revoke, and inspect provenance; store key material/fingerprint according to current secure-storage conventions.
- [ ] 2.5 Implement default rejection of unsigned packages and Developer Mode admission only as disabled + Strict with a persistent audited warning.
- [ ] 2.6 Implement bounded archive inspection/extraction using the limits in `design.md`; reject traversal, links, devices, duplicate normalized paths, Unicode/case collisions, compression bombs, oversized schemas/results, and undeclared executables.
- [ ] 2.7 Implement application-owned `quarantine`, content-addressed `packages`, runtime `scratch`, and `sidecars` roots with canonical ownership checks and safe cleanup helpers.
- [ ] 2.8 Implement `ExtensionInstallWitness` generation and validation binding package hash, signature state, manifest digest, installed state, dependencies, capability diff, contribution summary, compatibility, and selected trust profile.
- [ ] 2.9 Implement atomic snapshot publication and compensated SQLite pointer updates; retain the previous active snapshot on every failure path.
- [ ] 2.10 Implement startup reconciliation for abandoned quarantine, unreferenced snapshots, stale scratch directories, orphan sidecar state, and incomplete transaction journals without deleting user-owned paths.
- [ ] 2.11 Add security tests/fuzz targets for archive parser boundaries, path normalization, signature substitution, revoked keys, stale witnesses, rollback attempts, partial writes, cancellation, and concurrent installs.

## 3. Persistence and repositories

- [ ] 3.1 Add additive SQLite migrations for publishers, packages, installations, snapshots, dependencies, contributions, runtime generations, operation witnesses, Hook definitions/bindings/executions, authorization rules/rule sets, connector definitions/instances/bindings.
- [ ] 3.2 Follow current migration numbering, transaction, foreign-key, timestamp, and repository conventions; do not add a second generic operations table.
- [ ] 3.3 Add repositories with optimistic concurrency/version checks for installation state, active snapshot/generation pointers, Hook/rule generations, and connector state transitions.
- [ ] 3.4 Add uniqueness constraints for extension id/version/hash, global contribution id, active generation, rule source/id, connector source/id, and virtual Skill projection identity.
- [ ] 3.5 Add retention and redaction bounds for Hook executions, runtime diagnostics, and operation witnesses; raw secrets and unrestricted environment data must be impossible to persist.
- [ ] 3.6 Seed built-in extension and connector adapters idempotently without overwriting user enablement, configuration, trust, or legacy integration state.
- [ ] 3.7 Add migration upgrade tests from current schema, repeated-start idempotency tests, rollback/compensation tests, and repository concurrency tests.

## 4. Extension lifecycle and dependency resolution

- [ ] 4.1 Implement the installation state machine: Discovered, Validated, InstalledDisabled, Activating, Active, Degraded, Draining, Quarantined, Incompatible, Uninstalling, and Removed.
- [ ] 4.2 Implement preview/start operations for install, enable, disable, reload, rollback, and uninstall using current stable operation ids, progress stages, cancellation semantics, and unified logs.
- [ ] 4.3 Implement deterministic extension dependency resolution with SemVer, compatibility, enabled/installed state, topological order, cycle detection, optional versus required dependencies, and stable diagnostics.
- [ ] 4.4 Resolve required Skill dependencies through the published effective Skill/Registry API; block activation on unresolved required dependencies without installing a competing Skill copy.
- [ ] 4.5 Implement activation-event indexing and single-flight lazy activation; concurrent callers must share one activation result rather than starting duplicate runtimes.
- [ ] 4.6 Implement shadow generation creation, health handshake, contribution prepare/commit, atomic registry swap, old-generation draining, bounded cancellation, shutdown, and rollback.
- [ ] 4.7 Implement disable as an atomic contribution/generation removal for new calls while pinned in-flight calls retain their immutable generation until drain policy completes.
- [ ] 4.8 Implement crash/timeout accounting, the default three-failures-in-five-minutes quarantine policy, manual reset, rollback, and no automatic trust-profile downgrade.
- [ ] 4.9 Implement uninstall eligibility and witness validation; preserve logs, audit, Overlay, Skill configuration, user/project forks, credentials owned by other records, and active rollback evidence.
- [ ] 4.10 Add exhaustive state-machine tests, stale operation tests, concurrent enable/disable/reload tests, failure-injection tests at each lifecycle stage, and restart recovery tests.
- [ ] 4.11 Prove capability-gate enforcement over lifecycle (moved here from Task Group 0, which had no install or activation machinery to assert against): with `extension_platform.external_packages` off, install and update are refused before quarantine; with `extension_platform.catalog` off, enable and lazy activation are refused; turning a gate off while an operation is in flight refuses new work at once and lets the running operation settle under existing policy.

## 5. Runtime host registry, WASM, and sidecar protocol

- [ ] 5.1 Define `RuntimeHost`, `RuntimeInstance`, `RuntimeCallContext`, `CapabilityBroker`, `RuntimeHealth`, and shutdown/drain ports without leaking Wasmtime/process library types into the domain.
- [ ] 5.2 Add a compile-time built-in extension registry for reviewed Rust adapters; reject attempts to select `builtin` from an external `.vhext` manifest.
- [ ] 5.3 Reuse the existing Skill Tool Wasmtime engine/resource accounting through an adapter; avoid a second engine, cache, interruption implementation, or divergent capability model. The engine is core-module based on pinned `wasmtime = "=47.0.3"` without the component-model feature, behind the off-by-default cargo feature `skill-tool-module-runtime`; do not enable `component-model` in this change.
- [ ] 5.4 Implement the extension WASM ABI over core modules for initialize, tool call, Hook invocation, connector operations, logging, and capability-mediated host calls; reject `runtime.kind: wasm-component` with an explicit unsupported-runtime diagnostic.
- [ ] 5.5 Enforce trust-profile timeout, memory, fuel/epoch, result, log, concurrency, scratch, filesystem, network, process, and secret limits; Strict has no ambient network/process/secret authority.
- [ ] 5.6 Define and implement length-prefixed JSON-RPC 2.0 sidecar framing, initialization/version negotiation, correlation ids, frame/nesting limits, heartbeat, cancellation, and the methods listed in `design.md`.
- [ ] 5.7 Implement sidecar launch with scrubbed environment, application-owned cwd, bounded stdout/stderr, process-tree termination, timeout, heartbeat, and orphan cleanup.
- [ ] 5.8 Add a platform sandbox-provider interface and self-test. Standard sidecar activation SHALL fail closed when no verified provider is available; Trusted sidecar behavior must be clearly diagnosed rather than described as fully sandboxed.
- [ ] 5.9 Implement capability-mediated reverse host calls. Deny undeclared host calls before touching filesystem/network/process/credentials and route allowed calls through existing Permissions/credential/network/logging services.
- [ ] 5.10 Add a minimal Trusted-only Python sidecar SDK/bootstrap and fixture implementing `async activate(context) -> Contributions`; do not embed Python in Rust/Tauri.
- [ ] 5.11 Add tests for malformed/oversized frames, protocol mismatch, unsolicited calls, timeout, cancellation, output flood, stderr flood, crash, process-tree cleanup, forbidden capability calls, and runtime reuse/isolation.
- [ ] 5.12 Prove capability-gate enforcement over registration and execution (moved here from Task Group 0): with `extension_platform.wasm_module_runtime` or `extension_platform.sidecar_runtime` off, the matching runtime refuses registration and invocation and reports `not_compiled` or `runtime_disabled` rather than failing opaquely; a Standard sidecar without a verified sandbox provider reports `blocked_by_prerequisite` and never launches.
- [ ] 5.13 Prove WASM and sidecar drain and termination under gate closure (moved here from Task Group 0): turning a runtime gate off drains in-flight calls under the existing bounded window, terminates a sidecar process tree after its safe-shutdown timeout, and counts the closure as an intentional shutdown rather than a crash.

## 6. Transactional contribution registry

- [ ] 6.1 Add immutable `ContributionRegistryGeneration`, indexes by kind/global id/extension/activation event, and an atomic current-generation reader for Agent Runtime and UI.
- [ ] 6.2 Define `ContributionPublisher` prepare/commit/rollback ports and a coordinator that never exposes a partially published extension.
- [ ] 6.3 Add deterministic conflict validation: namespaced external ids cannot collide; built-in aliases require explicit reviewed mappings; duplicate effective tool/mode/connector names remain distinguishable by global id.
- [ ] 6.4 Add contribution eligibility state separate from installation and runtime state, with reasons such as Disabled, MissingDependency, PermissionConflict, RuntimeUnavailable, Quarantined, Incompatible, and Ready.
- [ ] 6.5 Persist contribution provenance with extension id/version/hash, snapshot, generation, local/global id, kind, manifest digest, and adapter commit witness.
- [ ] 6.6 Implement startup rebuilding from installed immutable manifests and compare it with persisted projections; drift fails closed and surfaces diagnostics.
- [ ] 6.7 Add failure injection for every publisher prepare/commit/rollback order and prove the prior generation remains usable after failure.
- [ ] 6.8 Add concurrent-reader tests proving generation snapshots are immutable and in-flight calls cannot observe mixed old/new contributions.
- [ ] 6.9 Prove quarantine survives a gate cycle (moved here from Task Group 0): turning a gate off and back on republishes contributions without reactivating a quarantined extension or reversing a prior recovery decision, and the quarantine reason and its evidence are unchanged by the cycle.

## 7. Tool, Skill, MCP, and interaction-mode adapters

- [ ] 7.1 Implement extension Tool descriptor validation and projection into the current native tool catalog with stable provenance, input/output schemas, handler reference, and activation event.
- [ ] 7.2 Route extension tool calls through current agent-tool execution: schema validation, before/after Hooks, Permissions, approval, timeout, cancellation, output bounds, trace, and audit.
- [ ] 7.3 Restrict first-release tool projection to OnePiece/native Agent unless an existing CLI integration exposes an approved safe tool bridge; do not inject unsupported flags/config into external CLIs.
- [ ] 7.4 Implement Skill subtree validation through the existing Skill parser/package contract and materialize one immutable virtual Registry-layer package per contributed Skill with source `extension:<id>`.
- [ ] 7.5 Preserve `Project > User > Registry > System` resolution and keep extension package signature, Skill effective eligibility, Skill Tool trust, Overlay, configuration, and permission grants as separate gates.
- [ ] 7.6 Implement extension disable/reload/uninstall behavior for virtual Skills without deleting Overlay, configuration, history, or in-flight snapshots.
- [ ] 7.7 Implement read-only namespaced extension-owned MCP definitions with no embedded secret values; bind credentials/environment/headers/Agents through existing MCP flows.
- [ ] 7.8 Apply existing MCP transport/session/tool limits and explicit approval floor to extension definitions; test disable/reload while sessions are active.
- [ ] 7.9 Implement declarative mode-preset projection referencing only registered runtime strategies, policy templates, tool groups, Skills, Hooks, and configuration schemas.
- [ ] 7.10 Reject executable third-party mode strategies and unknown strategy ids with stable diagnostics.
- [ ] 7.11 Add adapter parity/integration tests for projection, effective eligibility, provenance, permission behavior, disable/reload/rollback, and restart reconstruction.

## 8. Typed lifecycle Hook engine

- [ ] 8.1 Add `src-tauri/src/contexts/tooling/lifecycle_hooks/` with versioned event payloads, handler definitions, matchers, admissible decisions, bindings, execution context, trace, and circuit state.
- [ ] 8.2 Implement all internal v1 events in `design.md` and a registry that rejects unknown events or invalid handler/decision combinations.
- [ ] 8.3 Implement deterministic source-tier/priority/id ordering, scope matching, enabled/expiry checks, recursion guards, per-event concurrency, and bounded payload redaction.
- [ ] 8.4 Implement built-in, extension-runtime, command, HTTP, MCP-tool, prompt, and read-only Agent handlers behind ports; route command/network/model/MCP operations through current security and runtime services.
- [ ] 8.5 Implement event-specific decision merging. Deny dominates Ask; input/output/system/message transforms are bounded, typed, provenance-preserving, and forbidden where the event does not admit them.
- [ ] 8.6 Implement synchronous fail-closed enforcement for security-critical handlers and explicit fail-open observational behavior with diagnostics.
- [ ] 8.7 Implement timeouts, retries only where idempotent/configured, error budgets, circuit breakers, cooldown/reset, and extension quarantine interaction.
- [ ] 8.8 Emit Hook trace and audit records with redacted payload digests, handler/source, duration, decision, failure mode, circuit state, and related session/tool/operation ids.
- [ ] 8.9 Implement the versioned Claude Code compatibility catalog and import preview for SessionStart, UserPromptSubmit, PreToolUse, PermissionRequest, PostToolUse, PostToolUseFailure, Stop, StopFailure, PreCompact, PostCompact, and SessionEnd.
- [ ] 8.10 Adapt the current Claude permission-hook bridge to the internal permission/tool Hook events without breaking its fail-closed offline behavior.
- [ ] 8.11 Integrate Agent Runtime, compaction, tool execution, delegation, Permissions, and connector send/receive emitters through the published Hook dispatcher port.
- [ ] 8.12 Add tests for ordering, matching, every decision type, forbidden decisions/patches, recursion, timeout, handler crash, circuit breaker, compatibility mapping, audit redaction, and latency bounds.

## 9. Authorization rule compiler and Permissions integration

- [ ] 9.1 Add `src-tauri/src/contexts/permissions/rules/` with Rule domain types, source provenance, operation-specific matchers, compiler, immutable generations, evaluation trace, project-file loader, and simulation service.
- [ ] 9.2 Implement operation kinds for shell, file read/write, code modification, Git, network, MCP tool, extension tool, and connector operation; normalize current permission requests into these types.
- [ ] 9.3 Implement bounded glob/Rust-regex compilation and evaluation with limits on pattern/input size and stable errors for unsupported or expensive patterns.
- [ ] 9.4 Implement sources: immutable floor, built-in defaults, global/user, project `.vanehub/authorization.yaml`, extension, and explicit session rules.
- [ ] 9.5 Implement deterministic matching/precedence: immutable floor first; Deny dominates Ask; Ask dominates Allow; then current policy template/PDP fallback; Hooks may only preserve/strengthen; remembered grants may satisfy remaining Ask within scope.
- [ ] 9.6 Enforce that downloaded extension rules may only Ask or Deny. Allow is accepted only from reviewed built-in Trusted extensions and never below a safety floor.
- [ ] 9.7 Implement approval-scope constraints, auto-approve validation, expiry, source enablement, priority/specificity trace, and conflict diagnostics.
- [ ] 9.8 Implement debounced project-file watch/read/parse/compile/publish with canonical path checks, symlink-swap defense, partial-write tolerance, and last-known-good retention.
- [ ] 9.9 Implement preview/upsert/delete/list/reload/diagnostics application services and Tauri DTOs. Immutable rules remain read-only.
- [ ] 9.10 Implement non-executing simulation returning normalization, risk, matched rules, floors, template fallback, Hook policy, grant eligibility, and final decision chain.
- [ ] 9.11 Run the rule engine in shadow comparison against current Permissions, add parity/safety fixtures, and switch final evaluation only after no unexplained weakening is observed.
- [ ] 9.12 Add property tests proving added Deny/Ask/floor rules cannot make an operation less restrictive, plus tests for expiry, precedence, malformed YAML, reload races, stale generations, grants, and audit.

## 10. Connector Platform and migration adapters

- [ ] 10.1 Add `src-tauri/src/contexts/tooling/connectors/` with descriptor, instance, type, capability, auth strategy, lifecycle state, health report, binding, source, driver port, and operation models.
- [ ] 10.2 Implement driver registration for built-in and extension-contributed connectors, lazy runtime activation, configuration-schema validation, and contribution provenance.
- [ ] 10.3 Implement auth strategies none, external CLI, API key, OAuth authorization code with PKCE, device code, QR pairing, and host-delegated as typed contracts; individual drivers support only declared strategies.
- [ ] 10.4 Store only credential handles and status. Implement scoped credential-use calls, preserve/replace/clear compensation, redaction, expiry, and no raw secret exposure in list/detail DTOs or logs.
- [ ] 10.5 Implement connector configure/authenticate/test/connect/disconnect/reconnect/refresh lifecycle with stable async operations, generation-safe transitions, cancellation, retry/backoff, and health diagnostics.
- [ ] 10.6 Implement OAuth/remote HTTP security: PKCE, state/nonce where applicable, audience/resource binding, redirect allowlist, no token passthrough, and no credential forwarding across origins.
- [ ] 10.7 Migrate GitHub CLI readiness to a built-in connector driver using current `gh auth status` semantics; preserve legacy catalog ids, commands, and frontend methods as delegating adapters for one release.
- [ ] 10.8 Project Feishu, Telegram, DingTalk, WeCom, and WeChat state/operations through `communications::api`; do not move their runtime, repositories, secrets, or routing logic.
- [ ] 10.9 Project existing MCP definitions/state as Connector Type MCP for visibility only; MCP remains the transport/session owner.
- [ ] 10.10 Project remote workspace/browser/local capability integrations only when a real current API exists; do not create fake readiness rows for unimplemented proprietary systems.
- [ ] 10.11 Add lifecycle, auth race, secret redaction, GitHub parity, IM projection, MCP projection, disable/reload, and restart persistence tests.

## 11. Tauri command and frontend service contracts

- [ ] 11.1 Add thin one-command-per-file Tauri command modules for the extension, publisher, Hook, rule, and connector operations listed in `design.md`.
- [ ] 11.2 Register commands and application state through current composition roots; command files must not contain package parsing, repositories, process management, rule compilation, or Hook execution logic.
- [ ] 11.3 Add stable request/response DTOs, cursor/filter/pagination contracts, operation stages, error codes, and redacted diagnostics; map Rust errors without `unwrap()`/`expect()` in production.
- [ ] 11.4 Add or extend `ExtensionPlatformService` at the frontend service boundary and provide matching Tauri and deterministic Web/mock adapters, following the repository's `<x>-service.ts` + `runtime-<x>-client.ts` + `tauri-<x>-client.ts` + `web-<x>-client.ts` convention. Keep the `extension-platform` stem distinct from the existing local-capability `extension-service.ts` family.
- [ ] 11.5 Ensure all new component data flows through the service interface; add architecture tests rejecting direct Tauri imports/invocations from React components.
- [ ] 11.6 Implement Web/mock fixtures for installed extensions, contributions, Hook traces, rule simulation, connectors, operations, validation failures, quarantine, and empty/loading/error states without claiming native side effects.
- [ ] 11.7 Add frontend/native contract tests for discriminated unions, enum values, nullability, filters, pagination, operation polling/cancellation, stale witnesses, and feature flags.
- [ ] 11.8 Update `npm run contracts:check` fixtures/generation if the repository maintains generated command contracts.

## 12. Unified Extensions UI

- [ ] 12.1 Add the unified Settings → Extensions route/page and tab model: Installed, Contributions, Hooks, Rules, Connections, Diagnostics. Settings page id `extensions` already belongs to the local OCR/ASR/TTS capabilities page and `src/settings/settings-pages.test.ts` asserts its position relative to `im`, `plugins`, and `usage`; resolve that collision explicitly and update those assertions rather than shadowing the existing id.
- [ ] 12.2 Add compatibility redirect from the old Plugin Integrations route to Connections and retain deep links to Skills, MCP, Prompt Hooks, Agent Policies, local capabilities, and IM configuration.
- [ ] 12.3 Implement Installed filters/search/cards/table, state/runtime/trust/signature badges, contribution counts, health summary, and enable/disable/reload/rollback/uninstall actions.
- [ ] 12.4 Implement the seven-step local package install wizard with package validation, publisher/signature, dependencies, contribution review, capability diff, trust-profile restriction, witness confirmation, and operation progress.
- [ ] 12.5 Implement extension details with Overview, Contributions, Permissions, Dependencies, Runtime, and Logs; prevent raw secret/environment/path disclosure.
- [ ] 12.6 Implement Contributions explorer grouped by tool/Skill/MCP/mode/Hook/rule/connector with source, global id, eligibility, dependency/runtime state, and authoritative-page deep links.
- [ ] 12.7 Implement Hooks table/editor/test/trace UI with event-specific fields, immutable extension rows, source/scope/priority/failure mode/circuit state, and Claude import preview.
- [ ] 12.8 Implement Rules table/structured editor/YAML preview/project diagnostics/last-known-good status and decision simulator with a readable decision chain.
- [ ] 12.9 Implement Connections cards/list/config/auth/test/connect/disconnect/reconnect/bindings and projection badges for GitHub, IM, MCP, and extension sources.
- [ ] 12.10 Implement Diagnostics views for package validation, operations, generations, activation/crash/quarantine, adapter rollback, Hook metrics, rule compilation, and connector health; add copy-redacted-report action.
- [ ] 12.11 Split production TS/TSX files by feature/component so every file is ≤300 physical lines; do not add ESLint exemptions.
- [ ] 12.12 Use existing semantic tokens/Tailwind/primitives, compact desktop density, responsive stacking, keyboard navigation, visible focus, accessible names/live status, and loading/empty/error/disabled states.
- [ ] 12.13 Add translation keys and reviewed copy for every supported locale; do not leave hard-coded user-visible strings.
- [ ] 12.14 Add Vitest/component tests and Playwright flows for install review, feature flags, operations, Hooks, rules, connectors, diagnostics, legacy redirect, accessibility, and Web/mock honesty.

## 13. Existing subsystem compatibility and deprecation

- [ ] 13.1 Keep `plugin_integrations` public behavior operational through a compatibility adapter; mark internal APIs deprecated only after new connector parity tests pass.
- [ ] 13.2 Project existing OCR/ASR/TTS local extensions into the unified catalog while retaining current installation/process ownership and existing specialist UI actions.
- [ ] 13.3 Project Prompt Hooks into the generalized Hook list without converting template text into executable code or changing current draft/publish/rollback/evaluation behavior.
- [ ] 13.4 Preserve current Agent Policies templates and approval broker. Rules augment operation-specific decisions; they do not silently rewrite template assignments.
- [ ] 13.5 Preserve current MCP project/user configuration, transport behavior, explicit approval floor, bindings, and active-session semantics.
- [ ] 13.6 Preserve current Skill source precedence, Overlay, configuration, delegation, Registry provenance, and executable Skill Tool trust boundaries.
- [ ] 13.7 Preserve current IM connector runtime and routing. Unified actions delegate to Communications and use its generation-safe lifecycle.
- [ ] 13.8 Keep the internal provider SDK static and reject external extension contributions that attempt model-provider or CLI-provider registration.
- [ ] 13.9 Add compatibility tests proving old routes/service methods/Tauri commands and new projections report equivalent state for at least one release.
- [ ] 13.10 Add deprecation documentation and follow-up removal criteria; do not delete compatibility APIs in this change.
- [ ] 13.11 Prove gate scoping against the real subsystems (moved here from Task Group 0, which had no consumers to regress): with every `extension_platform.*` gate off, Prompt Hooks, the Permissions decision point and its immutable floors, IM connectors, Skill Tool enablement, and the local OCR/ASR/TTS pages behave exactly as they do without this change.

## 14. Observability, audit, security review, and documentation

- [ ] 14.1 Route all native logs through unified logging with extension id/version/hash, installation, runtime/registry generation, contribution, Hook/rule/connector, session/tool/operation correlation, and pre-persistence redaction.
- [ ] 14.2 Add structured audit records for publisher trust changes, install/enable/disable/reload/rollback/uninstall, capability review, activation, tool/Hook/rule decisions, connector auth/use, quarantine, and compatibility adapter calls.
- [ ] 14.3 Add metrics/diagnostics for activation duration, lazy-activation hit rate, runtime memory/fuel/timeouts, sidecar crashes, Hook latency/errors/circuit state, rule compile/evaluation, connector health, and adapter rollback.
- [ ] 14.4 Add bounded diagnostic export with explicit redaction tests for tokens, headers, secrets, environment, prompt content according to current logging policy, and user paths.
- [ ] 14.5 Perform a focused security review against the threat list in `design.md`; record mitigations and unresolved platform limitations in repository documentation.
- [ ] 14.6 Add developer documentation for manifest, contribution points, runtime/trust matrix, WASM ABI, sidecar protocol, Python SDK, Hooks, rules, connectors, packaging/signing, testing, and compatibility limits.
- [ ] 14.7 Add user documentation in English and Simplified Chinese for installing/reviewing/enabling/removing extensions, Developer Mode risks, Hooks, rules, connections, diagnostics, and recovery.
- [ ] 14.8 Update architecture inventory, bounded-context ownership, native API docs, settings navigation docs, and README roadmap/status when the feature gate becomes available.

## 15. Integration, evidence, and release gate

- [ ] 15.1 Add end-to-end signed fixture extensions covering: data-only contributions, WASM tool, Skill, MCP definition, mode preset, Hook deny/Ask, authorization rule, connector, incompatible version, missing dependency, crash, and update capability expansion.
- [ ] 15.2 Add native integration tests for install → enable → lazy activation → tool permission → Hook/rule decision → disable → reload → rollback → uninstall.
- [ ] 15.3 Add desktop tests for real Tauri IPC, restart persistence, package file selection, operation progress, Settings navigation, approval dialog interaction, diagnostics, and legacy compatibility.
- [ ] 15.4 Add platform-specific sidecar cleanup/sandbox evidence for Windows, macOS, and Linux. Mark unsupported Standard sidecar as `BLOCKED`/disabled, never as silently passed.
- [ ] 15.5 Run package/parser fuzz corpus and property tests long enough to produce retained evidence; add regressions for every discovered crash or policy weakening.
- [ ] 15.6 Run `npm run architecture:check`, `npm run contracts:check`, Playwright, desktop unit/E2E, documentation checks, coverage policy tests, and all feature-specific suites affected by this change.
- [ ] 15.7 Run the exact mandatory validation commands from root `AGENTS.md`:
  - [ ] `npm run lint:ci`
  - [ ] `npm run test`
  - [ ] `npm run build`
  - [ ] `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`
  - [ ] `cargo check --workspace`
  - [ ] `cargo clippy --workspace --all-targets -- -D warnings`
  - [ ] `npm run native:panic:check`
  - [ ] `cargo test --workspace`
  - [ ] `openspec validate --specs --strict`
- [ ] 15.8 Run `openspec validate add-unified-extension-platform --strict` and retain implementation validation evidence before archive.
- [ ] 15.9 Keep all seven `extension_platform.*` runtime gates disabled by default, and keep the `extension-wasm-module-runtime` and `extension-sidecar-runtime` Cargo features off, until all preceding security, parity, migration, and desktop gates pass.
- [ ] 15.10 Update this checklist only when implementation and tests for the task are complete; do not mark architecture scaffolding as completion of runtime or UI behavior.
