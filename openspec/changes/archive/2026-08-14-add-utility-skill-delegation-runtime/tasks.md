## 1. Contracts and domain model

- [x] 1.1 Add Utility delegation request, limits, exact revision snapshot, lifecycle state, terminal classification, and result domain types with validation tests.
- [x] 1.2 Add an idempotent lifecycle state machine that admits one attempt, rejects stale revisions and nested delegation, and accepts exactly one terminal transition.
- [x] 1.3 Define narrow tooling-resolution, child-execution, cancellation, observability, logging, and evolution-evidence ports without cross-context infrastructure imports.

## 2. Effective Utility resolution

- [x] 2.1 Add tooling application/API resolution for an enabled, trusted, available, effective Utility Skill in a canonical workspace with immutable revision and bounded content.
- [x] 2.2 Revalidate the effective revision at admission and return structured refusals for Role, shadowed, disabled, ambiguous, stale, or unsupported targets.
- [x] 2.3 Keep `load_skill` refusal semantics for Utility Skills while exposing bounded delegation discovery metadata to supported native Agents.
- [x] 2.4 Add unit tests for overlay-applied revisions, workspace isolation, alias resolution, shadowing, trust, and revision races.

## 3. Native delegation application service

- [x] 3.1 Implement delegation admission with UUIDv7 delegation/attempt ids, bounded host-capped limits, parent correlation, and safe lifecycle publication.
- [x] 3.2 Implement restricted child generation execution that removes the delegation tool, shares approved provider/model routing, and enforces task, instruction, duration, tool, approval, and result bounds.
- [x] 3.3 Propagate parent cancellation and timeout into the child execution and converge racing callbacks on one terminal result.
- [x] 3.4 Add tests for success, failure, refusal, cancellation, timeout, every limit class, nested delegation, duplicate terminal callbacks, and sink failures.

## 4. Native Agent tool integration

- [x] 4.1 Register the fixed-schema `delegate_utility_skill` tool only for native API Agents whose runtime advertises support.
- [x] 4.2 Map tool calls through the application boundary without exposing provider credentials, host paths, environment variables, or arbitrary executor configuration.
- [x] 4.3 Return bounded structured terminal results and preserve existing parent tool lifecycle/approval behavior.
- [x] 4.4 Add adapter and provider-contract tests proving CLI-like output and generic delegation metadata cannot create authoritative Utility attempts.

## 5. Safe projections and evidence closure

- [x] 5.1 Project started and terminal Utility lifecycle facts into execution observability and safe unified logging with no raw content.
- [x] 5.2 Project terminal facts through `EvidenceEnvelopeSink` with exact canonical Utility revision, correlation ids, workspace scope, fidelity, duration, counts, and terminal classification.
- [x] 5.3 Verify evidence enqueue/logging failures remain fail-open and rate-limited relative to the Utility result.
- [x] 5.4 Complete task 9.2b in `add-skill-evolution-evidence-pipeline` after authoritative runtime integration tests pass.

## 6. Service adapters and Skills UI

- [x] 6.1 Extend shared frontend Skill contracts with delegation capability and safe unavailable-reason fields.
- [x] 6.2 Map native capability through the Tauri adapter and return deterministic `native-runtime-unavailable` behavior from the Web/mock adapter.
- [x] 6.3 Update the Skills inventory/detail UI and all five locales to distinguish delegatable Utility, unavailable Utility, and Role Skill operations accessibly.
- [x] 6.4 Add service-contract, component, responsive, accessibility, and Playwright tests for native/Web capability presentation.

## 7. Architecture and verification

- [x] 7.1 Add architecture tests for tooling/agent-runtime boundaries and security tests confirming raw task, instruction, argument, output, credential, and path content never reaches evidence or logs.
- [x] 7.2 Run `npm run lint:ci`, `npm run test`, `npm run test:coverage`, `npm run build`, `npm run coverage:policy:test`, `npm run version:unit:test`, `npm run contracts:check`, and `npx playwright test`.
- [x] 7.3 Run `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`, `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`, `cargo test --manifest-path src-tauri/Cargo.toml`, and `cargo check --manifest-path src-tauri/Cargo.toml`.
- [x] 7.4 Run `npm run docs:check`, `openspec validate add-utility-skill-delegation-runtime --strict`, `openspec validate add-skill-evolution-evidence-pipeline --strict`, and `openspec validate --specs --strict`.
