## 1. Contracts and persistence foundations

- [x] 1.1 Define versioned native tool definition, eligibility, validated-input, execution-context, progress, and result-envelope contracts without provider-specific types
- [x] 1.2 Define stable error codes, limit profiles, readiness reason codes, permission actions, and canonical resource shapes for all extended tools
- [x] 1.3 Define versioned frontend DTOs for capability readiness, operations, approvals, Artifacts, delegation attempts, ChangeSets, apply attempts, and recovery state
- [x] 1.4 Add SQLite migrations and repository models for bounded operation metadata, immutable Artifacts, delegation attempts, ChangeSets, apply attempts, and recovery records
- [x] 1.5 Add migration, serialization, compatibility, and repository tests for every new persisted record and enum
- [x] 1.6 Add unified-log event definitions and pre-persistence redaction tests for new tool inputs, outputs, paths, URLs, prompts, credentials, and external-process errors

## 2. Fixed native tool registry

- [x] 2.1 Introduce the fixed `NativeToolHandler` registry and application ports described in the design
- [x] 2.2 Move provider-neutral catalog assembly behind registry definitions and retain Anthropic/OpenAI-compatible wire translations at the provider boundary
- [x] 2.3 Enforce stable `agent_id == "onepiece"` eligibility for every new handler during catalog construction
- [x] 2.4 Revalidate Agent id, generation/session ownership, canonical workspace, execution mode, readiness, policy, limits, and cancellation immediately before dispatch
- [x] 2.5 Route permission requests through the existing unified approval engine with input hashes and stale-approval rejection
- [x] 2.6 Implement shared deadlines, cancellation propagation, progress bounds, monotonic terminal states, cleanup, result truncation, and safe error mapping
- [x] 2.7 Adapt existing file, search, shell, Skill, LSP, and MCP handlers to the registry without changing their public names, schemas, or established behavior
- [x] 2.8 Add catalog and forged-dispatch tests proving custom API Agents and CLI-wrapped Agents cannot discover or invoke the new handlers
- [x] 2.9 Add provider parity and regression tests for registry catalog translation, legacy tools, policy modes, cancellation, and activity persistence

## 3. Artifact storage and publication

- [x] 3.1 Implement an application-owned content-addressed blob store with atomic writes, hash verification, media-type admission, quotas, and canonical metadata
- [x] 3.2 Implement immutable Artifact creation from admitted bytes and bounded text/JSON outputs with operation, source, and derivation lineage
- [x] 3.3 Implement Artifact inspection and bounded preview APIs that never expose arbitrary host paths
- [x] 3.4 Implement application-owned publication references and safe download streaming without accepting provider-authored publication URLs
- [x] 3.5 Implement explicit retention, expiry, reference-aware cleanup, and integrity-failure handling for blobs and metadata
- [x] 3.6 Register the fixed `artifact` handler with OnePiece-only eligibility and operation-specific permission classifications
- [x] 3.7 Add unit and integration tests for deduplication, immutability, tampering, quotas, traversal/symlink/special-file rejection, lineage, publication, download, and cleanup

## 4. Guarded Web research

- [x] 4.1 Implement the reviewed DuckDuckGo search adapter with bounded query options, result normalization, provider provenance, timeout, cancellation, and stable failures
- [x] 4.2 Implement URL normalization plus scheme, credential, port, DNS, IP-range, redirect, and rebinding defenses for guarded HTTP fetching
- [x] 4.3 Implement an isolated HTTP client with no ambient cookies or credentials and hard compressed/expanded byte, media-type, redirect, and duration limits
- [x] 4.4 Implement bounded HTML/text extraction that distinguishes search snippets from fetched content and records final URL and capture provenance
- [x] 4.5 Route separately admitted binary downloads into immutable Artifacts instead of returning executable or unbounded content inline
- [x] 4.6 Register fixed `web_search` and `web_fetch` handlers with independent readiness, permissions, lifecycle, and result envelopes
- [x] 4.7 Add deterministic adapter and adversarial tests for rate limits, malformed responses, SSRF, redirects, DNS changes, decompression bombs, unsupported media, cancellation, truncation, and citation metadata

## 5. Playwright browser automation

- [x] 5.1 Implement the owned Playwright stdio sidecar protocol with version handshake, bounded messages, health checks, restart limits, and process-tree cleanup
- [x] 5.2 Implement isolated browser-context creation with no normal-profile reuse, imported cookies, extensions, ambient credentials, or persistent session leakage
- [x] 5.3 Implement navigation, bounded visible-content inspection, selector-based extraction, screenshots, and constrained JavaScript evaluation
- [x] 5.4 Implement Artifact-mediated upload/download handling and integrity checks without exposing arbitrary host paths to the browser worker
- [x] 5.5 Classify risky navigation and page effects, bind approvals to canonical origin/action/input, and revalidate immediately before execution
- [x] 5.6 Implement human-handoff pause/resume/expiry semantics that prevent automation while control is handed to the user
- [x] 5.7 Register the fixed `browser` handler with per-operation validation, readiness, progress, cancellation, and safe result projection
- [x] 5.8 Add Playwright integration tests for isolation, navigation, extraction, screenshots, JavaScript bounds, approvals, handoff, popup/download limits, cancellation, and crash recovery

## 6. Dedicated code-execution sandbox

- [x] 6.1 Define the V1 runtime allowlist, version probes, source/input contract, output contract, and controller-owned hard ceilings
- [x] 6.2 Implement per-run private workspace creation with explicit Artifact materialization, read-only inputs, output admission, and guaranteed bounded cleanup
- [x] 6.3 Implement Windows restricted-token/AppContainer-compatible process isolation, Job Object CPU/memory/process/time limits, ACL confinement, and descendant termination
- [x] 6.4 Deny sandbox network access independently of Browser/Web availability and fail readiness closed when the required isolation primitive is unavailable
- [x] 6.5 Capture bounded stdout, stderr, exit status, duration, truncation, limit reasons, and admitted output Artifacts without routing through the general shell handler
- [x] 6.6 Register the fixed `code_execution` handler with exact source/runtime/input/limit approval binding
- [x] 6.7 Add isolation and integration tests for filesystem escape, symlinks, special files, network denial, process spawning, CPU/memory/disk/output limits, cancellation, cleanup, and runtime absence

## 7. Managed local OCR inference

- [x] 7.1 Extend the managed PaddleOCR contract with a versioned local inference protocol while preserving its existing installation, health, and self-test behavior
- [x] 7.2 Implement non-destructive OCR readiness checks that never install, download, start inference, or contact a remote service implicitly
- [x] 7.3 Implement Artifact-only image/PDF admission with hash verification, media checks, page selection, byte/pixel/page/rendering limits, and read-only staging
- [x] 7.4 Implement the bounded PaddleOCR worker adapter and managed PDFium page rendering with handshake, cancellation, process limits, and cleanup
- [x] 7.5 Normalize ordered blocks, page references, geometry, optional confidence, text projection, warnings, provenance, and truncation into a versioned result
- [x] 7.6 Register the fixed `ocr` handler and allow derived text/JSON results to be sealed as linked Artifacts
- [x] 7.7 Add fixture and integration tests for images, selected PDF pages, empty text, missing confidence, malformed worker data, oversized inputs, runtime mismatch, privacy redaction, cancellation, and no-remote fallback

## 8. CLI delegation model and readiness

- [x] 8.1 Implement aggregate `delegate_cli` readiness with separate Claude Code/Codex CLI analyze/edit probes, reviewed version ranges, stable reason codes, and no model invocation
- [x] 8.2 Model one logical Delegation with immutable request snapshot, bounded queue state, up to three explicit attempts, and one terminal result
- [x] 8.3 Enforce global, per-session, queue-length, queue-wait, attempt, event, transcript-summary, duration, and result-size ceilings from controller-owned policy
- [x] 8.4 Implement a bounded compatibility circuit breaker that disables only the affected target/mode after repeated protocol failures and exposes recovery diagnostics
- [x] 8.5 Persist safe delegation/attempt metadata and unified-log observations without raw credentials, hidden reasoning, full prompts, or full CLI transcripts
- [x] 8.6 Add state-machine, readiness, scheduling, retry, circuit-breaker, redaction, restart, and persistence tests

## 9. Isolated delegation execution

- [x] 9.1 Create an independent temporary clone from the exact canonical repository baseline using a controlled local object source and no configured remote
- [x] 9.2 Verify source repository identity, commit, clean baseline, submodule policy, path limits, case behavior, and forbidden repository states before delegation
- [x] 9.3 Materialize only the frozen task envelope, admitted context snapshot, and selected Artifact inputs into the clone with untrusted-content boundaries
- [x] 9.4 Build a minimal child environment that lets each CLI use its own supported authentication while excluding copied raw tokens and unrelated ambient variables
- [x] 9.5 Deny network access to delegated child commands while allowing only the delegated CLI process its required provider connectivity
- [x] 9.6 Implement the reviewed Claude Code non-interactive invocation for analyze/edit with fresh sessions and stateful stream/result parsing
- [x] 9.7 Implement the reviewed Codex CLI `exec` invocation for analyze/edit with ephemeral sessions, sandbox selection, JSON events, schema validation, and final-output capture
- [x] 9.8 Implement process-tree cancellation, timeout, output/event bounds, terminal-state validation, and cleanup for both CLI adapters
- [x] 9.9 Normalize provider output into `DelegationAgentReportV1`, treating claims as untrusted and separating report data from host-observed evidence
- [x] 9.10 Add fake-CLI fixtures and compatibility tests for success, schema drift, invalid event order, multiple terminals, prompt injection, credential leakage, child-process escape, cancellation, crash, and cleanup

## 10. Immutable ChangeSet capture and review

- [x] 10.1 Capture host-observed Git status, diff, untracked files, modes, hashes, symlink policy, binary metadata, and base commit after an edit delegation
- [x] 10.2 Reject forbidden paths, repository metadata, special files, unsupported submodule changes, out-of-budget output, and any result that cannot be represented exactly
- [x] 10.3 Seal one immutable content-addressed ChangeSet Artifact with canonical manifest, patches/blobs, base identity, attempt identity, and integrity hashes
- [x] 10.4 Compare the delegated report with host evidence and surface mismatches as warnings without treating model claims as proof
- [x] 10.5 Implement bounded ChangeSet inspection APIs for summaries, file lists, text diffs, binary notices, provenance, and integrity status
- [x] 10.6 Add capture/review tests for text, binary, rename, delete, mode, Unicode/case collisions, symlinks, untracked files, tampering, report mismatch, and deterministic hashing

## 11. Exact once-only ChangeSet application

- [x] 11.1 Register `apply_delegation_changes` separately from delegation and require a non-rememberable once-only approval bound to ChangeSet hash, target repository, and base
- [x] 11.2 Validate canonical target identity, exact base commit, clean worktree/index, platform/path compatibility, Artifact integrity, and one-time status before mutation
- [x] 11.3 Stage application into a private recovery area and preflight every target path, expected old hash, new hash, mode, and required operation atomically
- [x] 11.4 Implement exact all-or-nothing application without stash, rebase, merge, commit, push, remote access, or partial-success semantics
- [x] 11.5 Verify the complete post-apply tree and metadata against the ChangeSet manifest before recording success and consuming the one-time capability
- [x] 11.6 Implement rollback to the verified pre-apply snapshot and durable recovery state when application or verification fails
- [x] 11.7 Implement restart recovery that distinguishes safely completed, rolled-back, and manual-recovery-required states without automatic replay
- [x] 11.8 Add fault-injection tests at every application phase, including stale base, dirty target, hash mismatch, approval replay, concurrent mutation, lock loss, crash, rollback failure, and successful exact apply

## 12. Frontend service boundary and user experience

- [x] 12.1 Extend `AgentService` and related frontend interfaces with the new readiness, operation, Artifact, delegation, review, application, cancellation, and handoff contracts
- [x] 12.2 Implement Tauri commands and `tauri-agent-client.ts` mappings for the new contracts without exposing native implementation details to React
- [x] 12.3 Implement contract-compatible deterministic Web/mock behavior that labels simulated data and returns desktop-runtime-required outcomes for native effects
- [x] 12.4 Add OnePiece capability diagnostics with per-capability/per-mode readiness, stable unavailable reasons, and no side-effectful probes
- [x] 12.5 Add bounded operation/progress surfaces for Browser, Web, sandbox, OCR, Artifact, and delegation activity with cancellation and terminal-state handling
- [x] 12.6 Add Artifact inspection, publication, download, provenance, expiry, and integrity-warning surfaces
- [x] 12.7 Add delegation target/mode selection, attempt history, host-evidence report, ChangeSet diff review, exact approval summary, application, rollback, and recovery surfaces
- [x] 12.8 Add browser human-handoff controls and explicit indicators that automation is paused or resumed
- [x] 12.9 Update Plan/read-only policy projections so Browser, Web, code execution, delegation, publication, and apply are filtered as specified while admitted Artifact reads/local OCR remain available
- [x] 12.10 Add localized accessible labels, keyboard/focus behavior, responsive states, safe error copy, and component/service tests for every new surface
- [x] 12.11 Add tests proving React components use only the shared service boundary and desktop/Web adapters remain type- and behavior-compatible

## 13. Security, rollout, and operational hardening

- [x] 13.1 Add feature gates for each capability and ship read-only readiness/Artifact inspection before effectful execution, delegation edit, and ChangeSet apply
- [x] 13.2 Add policy-matrix tests across Agent identity, Plan/Default execution modes, readiness states, approval outcomes, workspace changes, cancellation, and Web/native runtimes
- [x] 13.3 Add end-to-end redaction tests proving raw credentials, authorization headers, prompts, page/file bodies, OCR text, hidden reasoning, and full external transcripts never enter durable logs
- [x] 13.4 Add concurrency, quota, cleanup, orphan-process, stale-lock, retention, database-restart, and bounded-event stress tests across all new domains
- [x] 13.5 Add compatibility fixtures for supported Playwright, PaddleOCR/PDFium, Claude Code, Codex CLI, runtime, and provider-protocol versions
- [x] 13.6 Document dependency installation, readiness diagnostics, permissions, privacy boundaries, retention, recovery, Web/mock limitations, and operator troubleshooting
- [x] 13.7 Record rollout and rollback criteria for every feature gate and verify disabling a capability leaves unrelated OnePiece and legacy tools functional

## 14. Full verification and handoff

- [x] 14.1 Run `npm run lint:ci` and fix all findings
- [x] 14.2 Run `npm run test` and fix all failures
- [x] 14.3 Run `npm run test:coverage`, `npm run coverage:policy:test`, `npm run version:unit:test`, and `npm run contracts:check`
- [x] 14.4 Run `npm run build` and fix all TypeScript or bundling failures
- [x] 14.5 Run `npx playwright test` for the changed UI behavior and browser integration
- [x] 14.6 Run `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`
- [x] 14.7 Run `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`
- [x] 14.8 Run `cargo test --manifest-path src-tauri/Cargo.toml`
- [x] 14.9 Run `cargo check --manifest-path src-tauri/Cargo.toml`
- [x] 14.10 Run `openspec validate complete-onepiece-builtin-tool-system --strict`
- [x] 14.11 Run `openspec validate --specs --strict`
- [x] 14.12 Record implementation verification evidence and leave the completed change ready for the governed OpenSpec archive workflow

## 15. Integration gap closure discovered during verification

- [x] 15.1 Keep every fixed OnePiece handler structurally registered while projecting backend readiness independently and fail closed for unavailable adapters
- [x] 15.2 Compose the delegation analyze/edit application services into a production `CliDelegationPort` with persisted attempts, strict provider protocols, cancellation, cleanup, and ChangeSet sealing
- [x] 15.3 Compose preflight, once-only approval, exact apply, verification, rollback, and recovery services into a production `ChangeSetApplyPort`
- [x] 15.4 Connect Browser handoff query/begin/resume Tauri commands to the owned browser session service with ownership-token and stale-revision checks
- [x] 15.5 Replace or remove the generic `start_builtin_tool_operation` placeholder so the shared frontend contract cannot advertise an operation that always returns `backend_unavailable`
- [x] 15.6 Add production-composition integration tests for readiness, delegation, apply, handoff, and manual operation controls, then rerun the complete verification matrix
- [x] 15.7 Route manual delegation start/apply commands through the same unified dispatcher and once-only approval lifecycle as model-originated tool calls, without reconstructing target authority from provider or Artifact strings
