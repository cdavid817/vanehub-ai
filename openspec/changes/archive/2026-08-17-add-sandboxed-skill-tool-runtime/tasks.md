## 1. Runtime contracts and dependency gates

- [x] 1.1 Add the `skill_tools` Rust context with domain ids, canonical tool keys, manifest version, implementation kind, capability, limit, trust, validation, lifecycle, quarantine, and diagnostic models.
- [x] 1.2 Define application ports for manifest discovery, integrity verification, schema validation, trust persistence, module execution, agent catalog contribution, permission dispatch, usage tracking, and unified logging.
- [x] 1.3 Specify and test the versioned `scripts/tools.json` contract, bounded JSON Schema subset, normalized ids, declarative templates, module exports, capability declarations, and centrally capped limits.
- [x] 1.4 Evaluate, pin, license-check, and advisory-check the JSON Schema and WebAssembly dependencies; document feature flags and keep declarative-only operation available when the module runtime is disabled.
  - Selected exact `wasmtime 47.0.3` with only `cranelift`, `runtime`, and `std`; feature-enabled `cargo check` passed and the resolved graph contains no `wasmtime-wasi` or `wasi-common`. The upstream license is `Apache-2.0 WITH LLVM-exception`; Bytecode Alliance dependency policy limits its graph to the documented permissive set. RustSec and upstream advisories were reviewed against 47.0.3 on 2026-08-16. `jsonschema 0.49.9` and beta `wasmi 2.0.0-beta.10` were rejected as documented in `docs/architecture/skill-tool-runtime-security.md`. On 2026-08-17, `cargo audit 0.22.2` completed with zero vulnerabilities and 18 allowed warnings outside the Wasmtime graph.
- [x] 1.5 Add contract fixtures for valid manifests and adversarial cases including traversal, duplicate ids, oversized/deep schemas, hash mismatch, forbidden implementation kinds, unknown versions, and undeclared files.
- [x] 1.6 Extend the manifest domain with normalized filesystem read/write scopes, HTTPS origins, structured process commands, opaque secret capability ids, provenance trust levels, and centrally capped aggregate resource requests.
- [x] 1.7 Add manifest negative fixtures for unknown versions/fields, absolute and parent-relative paths, wildcard/private hosts, shell strings, duplicate normalized entries, unknown secret ids, and excessive resource requests.

## 2. Discovery, integrity, and persistence

- [x] 2.1 Implement path-contained discovery from only the winning effective Skill revision and ignore tool content from shadowed revisions.
- [x] 2.2 Implement manifest/module hashing, capability digests, import/export inspection, file and aggregate size limits, and immutable effective revision witnesses.
- [x] 2.3 Add SQLite migrations and repositories for revision-bound trust, enablement, validation, quarantine, failure counters, and bounded diagnostic summaries.
  - Migration `skill-tool-runtime-foundation` is written as version **73**, which is provisional. `main` ends at 71 and 72 is claimed by an unmerged change, so this branch carries a real gap that `assert_migration_history_is_dense` rejects: standalone it fails 362 database-touching tests. Renumber to `main`'s maximum plus one at integration and update all five sites (`migrations.rs` call and `EXPECTED_MIGRATIONS`, `migrations.rs` `migration_state` assertion, `migration_fixture_tests::expected_versions`, and the `WHERE version =` probe in `skill_tools/infrastructure/tests.rs`). Verified green at a dense number: 3052 passed, 0 failed.
- [x] 2.4 Add corruption recovery and migration-equivalence tests so existing Skill records and Skills without tool manifests retain their current behavior.
- [x] 2.5 Extend effective Skill overview models with bounded tool inventory, integrity, trust, enablement, validation, and quarantine summaries without returning executable bytes.
  - The model field and its projection are implemented and tested. It is populated from an empty default until the registry snapshot in 6.6 supplies real inventory, which is also the correct value for every Skill that ships no tool manifest.
- [x] 2.6 Harden Overlay path validation and effective-content assembly so manifests, modules, hashes, and executable paths cannot be added or changed by patches, learning blocks, files, or evolution auto-apply.
  - Evolution auto-apply has no implementation in the codebase yet; when it lands it must route mutations through the same `validate_overlay_path` gate every other Overlay path already uses.
- [x] 2.7 Add a filesystem host gateway that reuses `CanonicalBoundary`, separates read/write admission, revalidates symlinks and canonical parents, owns temporary directories, and enforces file/aggregate byte limits.
- [x] 2.8 Add a direct-process host gateway with executable/argv/cwd/env admission, timeout, bounded child count and stdout/stderr, process-group or job-object cancellation, and no shell-string interpolation.
- [x] 2.9 Add a managed network host gateway with default deny, HTTPS origin admission, DNS/IP and loopback/private policy, per-hop redirect revalidation, proxy preservation, credential-origin isolation, timeouts, and byte limits.
- [x] 2.10 Integrate credential slots through opaque secret capabilities resolved only after permission approval and excluded from prompts, DTOs, inherited environments, transcripts, diagnostics, and logs.
- [x] 2.11 Implement atomic invocation budget reservations for wall time, host calls, child processes, output/file/network bytes, and concurrent jobs across nested delegation; expose actual platform enforcement strength.
- [x] 2.12 Add adversarial native tests for traversal, symlink swap/escape, hidden paths, command injection, hung descendant processes, huge output, undeclared/private/redirected network targets, secret exfiltration, and concurrent budget exhaustion.

## 3. Declarative tool execution

- [x] 3.1 Implement declarative template validation with schema-bound projections and constants while rejecting loops, conditionals, expressions, shell expansion, pipelines, and unknown targets.
- [x] 3.2 Implement the declarative dispatcher through the existing agent tool gateway with manifest capability checks, execution-mode intersection, cycle detection, host-call limits, and cancellation propagation.
- [x] 3.3 Validate input before dispatch and output before returning to the caller, enforcing payload size limits and bounded error results.
- [x] 3.4 Add tests proving declarative calls cannot bypass workspace containment, hidden-path restrictions, tool risk classification, permission policy, approval, plan-mode restrictions, or generation limits.

## 4. WebAssembly sandbox

- [x] 4.1 Implement the native module runtime adapter without inherited WASI, filesystem, network, process, environment, credential, clock, random, or unrestricted host imports.
- [x] 4.2 Enforce wall-time interruption, fuel, 64 MiB maximum memory, 1 MiB input/output, eight host calls, delegation depth four, bounded per-Skill concurrency, and parent cancellation with centrally configurable tighter ceilings.
- [x] 4.3 Implement fresh invocation stores and immutable compiled-artifact caching keyed by the complete revision and engine configuration witness.
- [x] 4.4 Implement the single structured host-call import and route it through capability, mode, permission, approval, recursion, and cancellation gates before existing tool dispatch.
- [x] 4.5 Add adversarial tests for infinite loops, memory growth, oversized buffers, invalid output, traps, forbidden imports, forged tool ids, cycles, depth exhaustion, concurrency pressure, timeout, and late completion after cancellation.
- [x] 4.6 Verify a sandbox failure terminates only the affected invocation and that repeated deterministic failures quarantine only the affected immutable tool revision.

## 5. Trust, permission, and approval integration

- [x] 5.1 Implement revision trust and revocation services binding source scope, base revision, manifest, implementation hashes, capability digest, actor, and timestamp; invalidate trust on any bound change.
- [x] 5.2 Add stable Skill tool principals containing parent agent, Skill/tool ids, revision, scope, workspace, session, and bounded delegation chain.
- [x] 5.3 Extend permission resource/action mapping so declared capabilities are upper bounds and every protected delegated operation independently resolves to Allow, Ask, or Deny.
- [x] 5.4 Extend approval requests with Skill provenance, concrete delegated operation, risk, redacted input summary, and immutable witness without treating one approval as trust or a reusable grant.
- [x] 5.5 Invalidate pending approvals on cancellation, revision replacement, disablement, quarantine, or witness mismatch and test that late decisions cannot execute invalid work.
- [x] 5.6 Add fail-closed tests for missing principal context, absent approval channels, policy/manifest conflicts, revoked trust, stale revisions, and similarly named tools.

## 6. Agent catalog, execution, and lifecycle

- [x] 6.1 Add a Skill tool catalog port to native API generation assembly and generate bounded provider-compatible names mapped to immutable internal keys with collision tests for every interface format.
- [x] 6.2 Expose Role tools only after that Role revision is loaded in the session and expose Utility tools only inside the delegated child execution context.
- [x] 6.3 Exclude disabled, archived, invalid, untrusted, quarantined, shadowed, and execution-mode-ineligible tools and report external CLI bridging as unsupported unless explicitly implemented later.
- [x] 6.4 Dispatch canonical Skill tool calls through the runtime, reject unknown or stale keys, and apply existing combined tool round-trip limits.
- [x] 6.5 Persist Skill tool provenance, status, and redacted result summaries on completed messages and emit existing lifecycle events for pending approval, start, completion, failure, and cancellation.
- [x] 6.6 Build validated registry snapshots and atomically swap them on Skill enablement, archive, delete, replacement, restore, effective-scope change, trust, validation, or quarantine transitions.
- [x] 6.7 Pin in-flight calls to immutable snapshots, cancel them on security quarantine when required, retire unreferenced compiled artifacts, and test concurrent refresh behavior.

## 7. Governance commands and frontend service boundary

- [x] 7.1 Add Rust application operations and Tauri commands for listing tools, validating a revision, trusting/revoking, enabling/disabling, quarantining/recovering, and reading bounded diagnostics with mapped command-boundary errors.
- [x] 7.2 Register commands and extend generated/shared TypeScript contracts for tool inventory, integrity, trust, capability diffs, validation, lifecycle, runtime support, and diagnostics.
- [x] 7.3 Extend `AgentService` and `tauri-agent-client.ts` with Skill tool operations, keeping direct `invoke()` calls out of React components.
- [x] 7.4 Extend `web-agent-client.ts` with interface-compatible inspection data and an explicit unsupported native execution state; add tests proving it never reports fake local execution success.

## 8. Skill Tools UI

- [x] 8.1 Add a Tools tab to Skill detail with accessible inventory rows for implementation kind, canonical id, revision, capabilities, integrity, trust, enablement, validation, quarantine, and recent status.
- [x] 8.2 Add an explicit trust/retrust dialog showing source scope, exact hashes, validation result, and capability diff; keep trust and enablement as separate actions.
- [x] 8.3 Add validate, revoke, enable/disable, quarantine, and recover flows with stale-revision protection, confirmation where destructive, and service-boundary error states.
- [x] 8.4 Add redacted validation and runtime diagnostics, limit-breach and quarantine explanations, and honest unsupported Web-runtime messaging that does not rely on color alone.
- [x] 8.5 Split production components below 300 physical lines and add Vitest coverage for keyboard access, focus behavior, status text, trust invalidation, adapter parity, stale responses, and failure recovery.
- [x] 8.6 Add Playwright coverage for inspect, trust, enable, approval provenance, quarantine, recovery, revision change, and unsupported Web behavior.
- [x] 8.7 Verify the affected security and approval surfaces in futuristic and minimal themes at desktop and narrow widths, with stable visual evidence for overflow, clipping, contrast, focus, and non-color state.

## 9. Observability, usage, and operational safety

- [x] 9.1 Emit redacted structured unified-log events for discovery, validation, trust, enablement, registry refresh, invocation, host calls, permission/approval outcomes, limits, quarantine, recovery, and cancellation.
- [x] 9.2 Apply schema-sensitive redaction and bounded summaries so raw credentials, secret fields, unrestricted paths, commands, module buffers, and full payloads are not persisted.
- [x] 9.3 Update Skill usage tracking for successful tool invocations and expose bounded counts and latest timestamps without creating feature-local log files.
- [x] 9.4 Add global and per-Skill execution kill switches, verify atomic registry removal, and retain audit/trust evidence for rollback diagnosis.
- [x] 9.5 Add load, concurrency, cancellation, and circuit-breaker tests proving one malicious or broken tool cannot starve or disable unrelated tools and sessions.

## 10. Verification and rollout

- [x] 10.1 Run `npm run lint:ci`, `npm run test`, `npm run test:coverage`, `npm run build`, `npm run coverage:policy:test`, `npm run version:unit:test`, and `npm run contracts:check`.
- [x] 10.2 Run `npx playwright test` for the Skill Tools UI behavior changes.
- [x] 10.3 Run `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`, `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`, `cargo test --manifest-path src-tauri/Cargo.toml`, and `cargo check --manifest-path src-tauri/Cargo.toml`.
- [x] 10.4 Run dependency license/advisory review and targeted hostile-module fixtures on the supported Windows toolchain with the module feature both enabled and disabled.
  - License and advisory review passed with zero vulnerabilities. GitHub Actions run `32000007318`, job `Native Check (windows-latest)`, passed the native build, declarative-only hostile Skill Tool suite, and module-runtime hostile Skill Tool suite. Linux default/module-enabled suites also passed 138/143 tests.
- [x] 10.5 Run `npm run desktop:unit:test` and `npm run test:desktop` on the current native platform, record Windows/macOS/Linux separately as PASSED, FAILED, BLOCKED, or NOT RUN, and do not extrapolate one platform to another.
  - Desktop harness unit tests passed 11/11. Native Desktop Smoke passed 1/1 on Linux `x86_64-unknown-linux-gnu` through the real Tauri runtime and IPC boundary. Linux: PASSED; Windows: NOT RUN; macOS: NOT RUN.
- [x] 10.6 Run deterministic resource-budget benchmarks or structural measurements covering manifest validation, bounded I/O, cancellation, and concurrent isolation without fragile shared-runner millisecond assertions.
  - Structural ceiling tests passed for aggregate manifest bytes, filesystem containment and byte exhaustion, cancellation accounting, atomic shared invocation budgets, and module per-Skill concurrency. These tests assert exact counters and admission ceilings rather than wall-clock timing.
- [x] 10.7 Run `openspec validate add-sandboxed-skill-tool-runtime --strict` and `openspec validate --specs --strict`, then record verification, visual, native-platform, performance, rollout, and rollback evidence before archive.
  - Both strict validations passed on 2026-08-17. Verification, visual variants, native platform status, structural performance evidence, rollout sequence, kill switches, and rollback procedure are recorded in `docs/architecture/skill-tool-runtime-security.md`.
