## 1. Prerequisites and Utility Metadata

- [x] 1.1 Verify `establish-effective-skill-runtime` and `add-skill-overlay-governance` are implemented, validated, and expose effective trusted Utility snapshots, assignments, pin state, logical resources, and usage counters.
- [x] 1.2 Add failing Skill metadata tests for valid Utility delegation contracts, missing-contract read-only defaults, unknown capability ids, invalid limits, and platform capping.
- [x] 1.3 Implement typed delegation metadata parsing, stable capability ids, requested limits, effective limits, and safe unavailable reasons.
- [x] 1.4 Extend effective Skill and management response models with Utility eligibility, declared/effective capabilities, requested/effective limits, delegation support, use, and history summaries.
- [x] 1.5 Add assignment tests for supported native API Agents, unsupported CLI Agents, unsupported API runtimes, and legacy unsupported associations.
- [x] 1.6 Implement Utility assignment eligibility without changing Role prompt bindings or CLI mount behavior.

## 2. Delegation Domain and Persistence

- [ ] 2.1 Add domain tests for delegation attempt identities, states, monotonic transitions, terminal immutability, captured revisions, limits, counts, and structured results.
- [ ] 2.2 Implement `UtilityDelegationAttempt`, prepared snapshot, lifecycle state, effective limit, evidence reference, and terminal result domain models.
- [ ] 2.3 Add SQLite migrations for Utility delegation attempts and indexed links to parent runs, generations, messages, sessions, principals, canonical Skills, and execution traces.
- [ ] 2.4 Add database upgrade, rollback-compatibility, pagination, workspace-isolation, status-filter, Agent-filter, and time-filter tests.
- [ ] 2.5 Implement attempt repositories and bounded newest-first history queries without persisting hidden reasoning, credentials, full prompts, unrestricted paths, or unbounded outputs.
- [ ] 2.6 Add recovery tests that convert persisted non-terminal attempts to `interrupted` and never replay provider or tool work after restart.

## 3. Parent-Child Permission Principals

- [ ] 3.1 Add permission-domain tests for stable Utility child-principal identity, valid depth-one parent relationship, unauthorized parent assignment, non-Agent parent, cycles, excessive depth, and parent mutation.
- [ ] 3.2 Activate internal-only child-principal creation and reuse keyed by stable parent Agent principal and canonical Utility id.
- [ ] 3.3 Persist bounded budget configuration on child principals and preserve existing root-principal behavior and migrations.
- [ ] 3.4 Add evaluation tests for child and parent Allow, child Ask, parent Ask, child Deny, parent Deny, remembered child grants, and delegation-start grants.
- [ ] 3.5 Implement explicit-Deny-first parent-chain evaluation where Allow requires both child and parent chain to allow and parent Allow never grants an otherwise unresolved child action.
- [ ] 3.6 Extend permission audit records with safe parent-chain and deciding-mechanism metadata without changing existing approval semantics.
- [ ] 3.7 Verify legacy callers still receive a delegation-not-authorized error when attempting to set `parent_principal_id` directly.

## 4. Fixed Tool and Eligibility Preparation

- [ ] 4.1 Add provider-format tests for one stable `delegate_skill` schema in Anthropic and OpenAI-compatible requests regardless of Utility inventory.
- [ ] 4.2 Add the fixed delegation definition to supported native API Agent tool catalogs and omit it from unsupported runtimes and third-party CLI integrations.
- [ ] 4.3 Add input-validation tests for unknown fields, malformed ids and aliases, task/context/resource limits, invalid logical URIs, and excessive resource counts.
- [ ] 4.4 Implement dispatch-time canonical resolution and eligibility checks for Utility type, enablement, trust, assignment, workspace, effective revision, Overlay health, metadata, and runtime support.
- [ ] 4.5 Build immutable prepared Utility snapshots containing effective instructions, resources, revision hashes, capabilities, limits, provider/model capture, workspace, and parent owner.
- [ ] 4.6 Add stale-snapshot tests for changed effective layer, Overlay revision, trust, assignment, pin state, provider profile, model availability, and capability hash.
- [ ] 4.7 Revalidate prepared snapshots after approval and return a stale structured result rather than executing changed content.

## 5. Start Approval and Approval Ownership

- [ ] 5.1 Define the `agent.delegate` action and revision/capability-bound Utility resource in the unified permission model with default `Ask` behavior.
- [ ] 5.2 Add delegation-start tests for policy Allow, Deny, Ask, Once approval, remembered scopes, timeout, stale generation, and changed revision after approval.
- [ ] 5.3 Route start evaluation and pending decisions through the existing permission service and ApprovalBroker without adding a second queue.
- [ ] 5.4 Extend pending approval models and UI contracts with safe parent Agent, Utility, revision, task summary, workspace, risk, and capability-ceiling context.
- [ ] 5.5 Add owner links for parent-generation start approvals and child-attempt action approvals.
- [ ] 5.6 Implement cascading stale resolution when parent or child cancellation occurs and verify no denied or stale start creates a running child.
- [ ] 5.7 Prove a remembered delegation-start grant cannot authorize any child tool action.

## 6. Reusable Child Generation Runtime

- [ ] 6.1 Refactor the native generation and tool loop behind an execution-owner abstraction while preserving all existing parent generation tests and provider behavior.
- [ ] 6.2 Add child provider-snapshot tests for captured profile, interface format, model, reasoning options, opaque credential handle, later configuration change, and removed credentials.
- [ ] 6.3 Implement child attempt creation without creating a normal user-visible session or sharing the parent provider conversation id.
- [ ] 6.4 Add prompt-envelope tests proving inclusion of native child instructions, effective Utility instructions, task, explicit context, permitted logical resources, capabilities, limits, and output contract.
- [ ] 6.5 Prove child prompts exclude the full parent transcript, hidden reasoning, unrelated memories, environment, raw credentials, arbitrary files, and host paths.
- [ ] 6.6 Implement strict task and context budgeting that rejects overflow instead of silently truncating intent.
- [ ] 6.7 Resolve logical resource references against the captured effective Utility snapshot and reject stale or out-of-scope resources before model execution.

## 7. Child Tool Ceiling and Execution

- [ ] 7.1 Define the platform child-tool capability map over existing file, edit, search, memory, Skill-read, shell, and other supported operations.
- [ ] 7.2 Add intersection tests for platform allowlist, parent mode, Utility declaration, trust, runtime availability, Plan mode, Standard mode, and absent capabilities.
- [ ] 7.3 Implement child catalog construction and repeat the same ceiling check at dispatch.
- [ ] 7.4 Keep `delegate_skill`, dynamic scripts, and MCP tools absent from the initial child catalog and add direct and indirect recursion-refusal tests.
- [ ] 7.5 Reject Plan-mode delegation when any effective Utility capability is non-Plan or mutating instead of silently narrowing its contract.
- [ ] 7.6 Route every child tool call through existing schema validation, workspace sandbox, unified permission evaluation, pending approval, timeout, cancellation, and bounded result handling under the child principal.
- [ ] 7.7 Add child read, search, write, edit, shell-denied, missing-tool, approval, denial, timeout, and result-persistence integration tests.
- [ ] 7.8 Link bounded child tool outcomes to the attempt and parent message without duplicating unbounded content.

## 8. Limits, Cancellation, Results, and Usage

- [ ] 8.1 Centralize platform ceilings for depth, active children, attempts, rounds, duration, task, context, output, and evidence references and serialize effective values into attempts.
- [ ] 8.2 Add tests for one active child per parent, deterministic handling of multiple requested delegations, total-attempt exhaustion, round limit, duration timeout, output truncation, and evidence cap.
- [ ] 8.3 Implement parent and child cancellation-token hierarchy across provider streams, tool work, and approval waits.
- [ ] 8.4 Add independent child cancellation that returns a cancelled tool result while leaving the parent generation active.
- [ ] 8.5 Keep the existing parent stop action cascading to active children and pending approvals.
- [ ] 8.6 Implement bounded structured results for completed, denied, failed, cancelled, timed-out, limited, interrupted, stale, and ineligible outcomes.
- [ ] 8.7 Verify child failures return as tool results and do not automatically fail a still-active parent generation.
- [ ] 8.8 Increment Utility `use_count` exactly once when the first child provider request begins, transactionally with the running attempt, and not for previewed, refused, denied, timed-out-before-start, or pre-start-cancelled calls.

## 9. Observability and Unified Logging

- [ ] 9.1 Add native execution topology tests for parent tool span, child attempt, model rounds, tools, approvals, retry attempts, refusal-before-child, cancellation source, and terminal status.
- [ ] 9.2 Emit correlated Utility delegation spans and links using existing execution-run infrastructure and native fidelity metadata.
- [ ] 9.3 Add metadata-only privacy tests excluding task/context bodies, Utility instructions, hidden reasoning, credentials, file contents, raw commands, and full paths.
- [ ] 9.4 Add bounded low-cardinality metrics for Utility id, status, duration bucket, limit reason, approval outcome, model rounds, and tool count without high-cardinality attempt or content dimensions.
- [ ] 9.5 Route operational diagnostics through unified logging with safe identities, hashes, counts, statuses, and reason codes; add no Utility-specific log files.
- [ ] 9.6 Link Utility history records to existing execution timeline and permission audit queries without duplicating authoritative data.

## 10. Frontend Service and Runtime Adapters

- [ ] 10.1 Extend shared TypeScript contracts and `agent-service.ts` with typed Utility metadata, attempts, lifecycle events, structured results, history, approval context, and child cancellation without `any`.
- [ ] 10.2 Update `tauri-agent-client.ts` as the only frontend native invocation and event-mapping boundary for Utility assignment, history, events, and cancellation.
- [ ] 10.3 Implement deterministic Web/mock delegation state machines for eligible, approval-blocked, running, child approval, completed, denied, failed, limited, cancelled, and interrupted outcomes.
- [ ] 10.4 Add adapter-parity tests proving Tauri payload mappings and Web/mock simulations use identical frontend shapes, stable ids, terminal semantics, and errors.
- [ ] 10.5 Verify Web/mock execution performs no provider, process, filesystem, credential, or native permission side effects.

## 11. Chat Delegation Experience

- [ ] 11.1 Add a child-activity reducer keyed by delegation attempt id that handles awaiting approval, running, tool activity, child approval, and every terminal state without duplicating parent messages.
- [ ] 11.2 Render Utility activity as a collapsible section attached to the parent assistant message with identity, status, elapsed time, counts, limits, summary, evidence, truncation, and safe errors.
- [ ] 11.3 Add an independent active-child cancel control through the service boundary and preserve the existing cascading parent stop behavior.
- [ ] 11.4 Persist bounded child activity projections on completed parent messages and reconstruct chronological attempts after session reload.
- [ ] 11.5 Add chat tests for streamed updates, out-of-order duplicate events, approval transitions, successful and failed continuation, cancellation, restart-interrupted state, history reload, and Web parity.
- [ ] 11.6 Add localized accessible names, live status semantics, keyboard controls, focus behavior, and responsive rendering for delegated activity.

## 12. Skills and Approval UI

- [ ] 12.1 Update Skill inventory and details with Utility type, delegated-delivery explanation, trust, revision, declared/effective capabilities, requested/effective limits, availability, use, last use, and repair reasons.
- [ ] 12.2 Update the selected-Agent assignment board to label native API Utility relationships as delegated capability and suppress unsupported CLI Assign actions without hiding repair state.
- [ ] 12.3 Add bounded Utility history filters, pagination, attempt details, execution timeline links, permission audit links, and empty states through the service boundary.
- [ ] 12.4 Extend existing approval cards with delegation-start and child-action context while preserving pending-list reconciliation, remembered scopes, timeout, notification, and confirmation behavior.
- [ ] 12.5 Add component and interaction tests for eligible and unavailable Utilities, capped capabilities, API assignment, CLI refusal, row-scoped errors, history, approval distinction, Web parity, accessibility, and responsive layout.
- [ ] 12.6 Keep new production TS/TSX modules within the 300-line rule by separating Utility metadata, assignment, history, child activity, and approval context components.
- [ ] 12.7 Run `npx playwright test` and resolve Utility delegation regressions across Skills, chat, approval, cancellation, and history workflows.

## 13. Verification and Documentation

- [ ] 13.1 Document native API versus CLI delegation scope, Utility metadata, capability ceilings, parent-child permissions, approvals, context privacy, limits, cancellation, results, and observability without external-product comparisons.
- [ ] 13.2 Run `npm run lint:ci`.
- [ ] 13.3 Run `npm run test` and `npm run test:coverage`.
- [ ] 13.4 Run `npm run coverage:policy:test`, `npm run version:unit:test`, and `npm run contracts:check`.
- [ ] 13.5 Run `npm run build`.
- [ ] 13.6 Run `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`.
- [ ] 13.7 Run `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`.
- [ ] 13.8 Run `cargo test --manifest-path src-tauri/Cargo.toml` and `cargo check --manifest-path src-tauri/Cargo.toml`.
- [ ] 13.9 Run `openspec validate add-delegated-utility-skills --strict` and `openspec validate --specs --strict`.

