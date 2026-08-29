## 0. Baseline, overlap, and failing-characterization tests

- [x] 0.1 Read `AGENTS.md`, `openspec/project.md`, the three affected main specifications, the current permission implementation, and every active change touching permissions, agent-runtime approval, Claude hooks, or database migrations; record overlap and select no migration number yet.
- [x] 0.2 Run `openspec validate fix-permission-decision-atomicity-and-grant-precedence --strict` before production code changes and repair any artifact validation failure.
- [x] 0.3 Add a repository test that inserts conflicting Session, Project, and Global grants in several orders and demonstrates that current lookup can return an insertion/query-order-dependent result.
- [x] 0.4 Add a test that repeats a remembered decision for the same scope key and demonstrates that current storage retains duplicate effective rows.
- [x] 0.5 Add an application interleaving test proving the current resolve command can deliver Allow before grant/audit persistence and that a persistence failure cannot be retried after the pending row is removed.
- [x] 0.6 Add concurrent first-principal tests and evaluation-error evidence tests that fail against the current select-then-insert and silent fallback behavior.

## 1. Domain model and invariants

- [x] 1.1 Add `CanonicalGrantKey`, scope-owner validation, persisted-effect validation, revision, update time, activation state, and optional resolution association to the permissions domain without adding SQLite or Tauri dependencies.
- [x] 1.2 Define deterministic applicable-scope ranking (`Session > Project > Global`) and unit-test every match/non-match combination, including broader conflicting effects.
- [x] 1.3 Replace append semantics in the application port with behavior-oriented `upsert_pending_grant_intent`, `activate_grant_for_resolution`, and deterministic `find_effective_grant` operations.
- [x] 1.4 Add `ApprovalResolution`, `ApprovalResolutionId`, `ApprovalResolutionState`, `ResolutionClaim`, `DeliveryReservation`, and typed delivery outcomes with constructors that prevent mutable decision fields and invalid transitions.
- [x] 1.5 Add domain transition tests for pending → resolving → committed → delivered, pre-commit claim reversion, stale, delivery-failed, aborted-by-restart, duplicate acknowledgement, and prohibited grant activation.

## 2. Migration and schema compatibility

- [x] 2.1 Scan `platform::database`, all context migration modules, and every active change for claimed versions; choose the next free version and add a collision/uniqueness test before registering it.
- [x] 2.2 Implement a transactionally rebuilt `permission_grants` table with validated scope/effect shape, `revision`, `updated_at`, `activation_state`, `resolution_id`, lookup index, and three scope-specific partial unique indexes.
- [x] 2.3 Implement deterministic legacy deduplication by canonical key and test each tie-breaker, malformed scope row, Once/Ask exclusion, preserved valid effect, and idempotent upgrade from the previously released schema.
- [x] 2.4 Add `approval_resolutions` with unique `request_id`, immutable decision metadata, bounded delivery state/counter/error fields, and indexes needed by request lookup and startup reconciliation.
- [x] 2.5 Extend `approval_audit` with nullable backward-compatible resolution/outcome metadata, preserving append-only reads and existing historical rows.
- [x] 2.6 Add migration fault-injection tests proving a failed copy, dedupe, index creation, invariant check, or table swap leaves the pre-migration schema/data intact.
- [x] 2.7 Add schema invariant tests proving duplicate canonical keys, invalid scope ownership, Once/Ask grants, and duplicate request resolutions are rejected by storage as well as domain code.

## 3. Deterministic grant repository

- [x] 3.1 Replace unordered candidate loading and Rust `.find()` with one explicit ranked SQL query that returns only active grants and has deterministic final ordering.
- [x] 3.2 Implement revisioned UPSERT for one canonical key and prove 100 concurrent same-key remembers leave one row with a monotonic final revision and no lost valid state.
- [x] 3.3 Implement pending grant intent and activation/cancellation by `resolution_id`, with idempotent repeated activation and no evaluation visibility before active state.
- [x] 3.4 Preserve exact principal/action/resource matching; add tests that this change does not accidentally introduce wildcard, prefix, display-name, or path-normalization authorization.
- [x] 3.5 Add compatibility tests for previously stored valid Global, Project, and Session grants and for evaluation fallback when no active row applies.

## 4. Atomic resolution repository

- [x] 4.1 Define one permissions application port whose `commit_resolution` transaction writes the immutable resolution, decision audit, and optional pending grant intent together.
- [x] 4.2 Implement the SQLite adapter using one explicit transaction/connection and prepared statements; do not expose `rusqlite::Connection` outside infrastructure.
- [x] 4.3 Add a deterministic failure injector for every statement and commit boundary; assert all-or-nothing rows and that no active grant can remain from a rolled-back transaction.
- [x] 4.4 Implement idempotent read-by-request-id, record-delivery-failure, acknowledge-delivery-and-activate, and mark-aborted-by-restart operations with guarded state transitions.
- [x] 4.5 Add repository tests for duplicate commits, conflicting decision attempts, duplicate delivery acknowledgements, activation retry after an acknowledgement-update failure, and immutable decision fields.

## 5. Pending claim and resolve use case

- [x] 5.1 Replace the pending map's remove-first finalization with internal Pending/Resolving/Committed phases and compare-and-revert ownership for pre-commit retryable failure.
- [x] 5.2 Implement one `ResolveApprovalUseCase` that claims, reserves, commits, delivers, acknowledges, and reconciles human Allow/Deny without command-level orchestration.
- [x] 5.3 Make missing-pending retries consult `approval_resolutions` by request id and return the existing typed state rather than producing a second resolution or an ambiguous not-found.
- [x] 5.4 Preserve Skill/delegation forced-Once rules inside the domain/use case and prove they cannot create remembered grant intent through the new path.
- [x] 5.5 Add application tests for double click, two conflicting callers, cancellation racing human approval, timeout racing human approval, stale generation, reservation failure, commit failure, delivery failure, acknowledgement failure, and successful retry.

## 6. Native Agent and Claude hook delivery adapters

- [x] 6.1 Add the narrow published `agent_runtime::api` contract needed to reserve one current tool-approval waiter and deliver one immutable resolution idempotently; do not expose generation repositories or private workflow state.
- [x] 6.2 Define the consuming `ApprovalDeliveryPort` in `permissions` and implement a native-Agent adapter over the published `agent_runtime` API.
- [x] 6.3 Refactor the Claude hook wait registry to support reservation, immutable resolution-id delivery, duplicate acknowledgement, stale waiter detection, and cancellation without introducing a second decision engine.
- [x] 6.4 Add a routed delivery adapter selected from the request's stable channel and assemble it only in bootstrap.
- [x] 6.5 Reduce `resolve_pending_approval` and related commands to DTO validation/mapping plus one use-case call; add command serialization and stable error-code tests.
- [x] 6.6 Add tests proving no adapter can observe Allow before the commit port returns success and that duplicate `resolution_id` delivery cannot resume native or hook work twice.

## 7. Timeout, evaluation failure, and restart reconciliation

- [x] 7.1 Route timeout sweep through the same claim/resolution use case with `decider = timeout`, preserving the existing bounded timeout and provider continuation semantics.
- [x] 7.2 Implement emergency Deny-only delivery for storage outage, with redacted unified diagnostics, no grant, no execution, and no later reinterpretation as Allow.
- [x] 7.3 Replace principal select-then-insert with atomic repository `get_or_create` while preserving read-only `find_by_agent_id` for settings listing.
- [x] 7.4 Add evaluation-error audit attribution and the unified-log fallback when audit persistence itself is unavailable; verify redaction of resource/tool metadata.
- [x] 7.5 Add startup reconciliation for committed/delivery-failed rows: mark them aborted/delivery-unknown, keep grant intent inactive, and never create a live pending request or target a new generation.
- [x] 7.6 Add restart/fault tests for crash after commit, crash after waiter applies but before acknowledgement, and activation update retry; assert least-privilege outcomes.

## 8. Frontend service, Web/mock, UI, and i18n

- [x] 8.1 Extend the frontend permission service with typed resolving/delivery outcomes while preserving the existing action/resource/effect and pending-list contracts.
- [x] 8.2 Update the Tauri adapter and event mapping without adding direct `invoke()` calls to React components.
- [x] 8.3 Implement deterministic Web/mock claim, commit, delivery acknowledgement, duplicate resolve, pending grant activation, stale resolution, and failure simulation with no native side effects.
- [x] 8.4 Update Approval UI state so one request disables conflicting controls while resolving, reconciles by pull after ambiguous failure, and presents stale/delivery-failed states without claiming execution occurred.
- [x] 8.5 Add all new user-visible strings to every registered locale and pass i18n key/interpolation parity tests.
- [x] 8.6 Add frontend unit/component tests for double-click suppression, event/pull reconciliation, Web duplicate resolve, retry after ambiguous response, and inactive grant behavior.

## 9. Architecture, documentation, and cleanup

- [x] 9.1 Add or update architecture fitness tests proving commands contain no SQL/external-effect orchestration, permissions does not import agent-runtime private modules, and infrastructure implements application-owned ports.
- [x] 9.2 Update permission developer documentation with canonical grant precedence, resolution transaction, reservation/commit/delivery sequence, acknowledgement-gated grant activation, emergency Deny, and restart semantics.
- [x] 9.3 Remove obsolete separate `GrantRepository::create` plus `AuditRepository::append` finalization paths only after all callers and compatibility tests use the atomic port.
- [x] 9.4 Review logs, DTOs, SQLite rows, snapshots, and test fixtures for raw command bodies, secrets, credentials, unrestricted paths, or provider payloads and remove any new leakage.

## 10. Verification

- [x] 10.1 Run the focused permissions domain/application/infrastructure tests, permission-hook workspace member tests, frontend permission tests, and migration upgrade/fault-injection tests; record exact counts and results.
- [x] 10.2 Run `npm run architecture:check` and resolve every cross-context/Tauri-boundary failure without adding blanket allowlists.
- [x] 10.3 Run the full validation command set from `AGENTS.md`: `npm run lint:ci`, `npm run test`, `npm run build`, Cargo fmt/check/clippy/panic-check/test, and `openspec validate --specs --strict`.
- [x] 10.4 Run `openspec validate fix-permission-decision-atomicity-and-grant-precedence --strict` after all task/spec edits.
- [ ] 10.5 Run applicable desktop permission/Claude hook flows with fixed fixtures, record each actually tested operating system as PASSED/FAILED/BLOCKED/NOT RUN, and do not infer untested platform results.
- [x] 10.6 Compare the final implementation against every requirement and scenario, leave any unmet task unchecked, and document residual risk before archive.
