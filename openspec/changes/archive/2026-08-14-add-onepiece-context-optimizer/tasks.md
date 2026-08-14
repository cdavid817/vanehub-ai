## 1. Optimizer Domain Model

- [x] 1.1 Add versioned optimization plan, action, budget, summary boundary, candidate evidence, verification, and fallback reason domain types with bounded counters and safe fingerprints.
- [x] 1.2 Implement deterministic plan ordering and tests for discardable, reinjectable, microcompactable, summarizable, protected, verbatim, unknown, and protocol-incomplete inputs.
- [x] 1.3 Implement contiguous oldest-round summary selection that never splits an API round and stops when the target budget is projected to be met.
- [x] 1.4 Add planner boundary tests for empty context, no reclaimable content, character-only estimates, saturated arithmetic, exact target equality, and candidates larger than the original.
- [x] 1.5 Add plan invariants proving every action references one known component or complete round, actions do not overlap, and protected or verbatim fingerprints are never selected.

## 2. Deterministic Low-Cost Reduction

- [x] 2.1 Implement explicit transient-content removal only for recognized `discardable` components and preserve conservative unknown content.
- [x] 2.2 Define a bounded protocol-valid tool-result replacement carrying tool correlation, outcome state, source fingerprint, and compacted marker without raw output.
- [x] 2.3 Implement large and duplicate old tool-result microcompaction while preserving recent, protected, and protocol-incomplete results.
- [x] 2.4 Add Anthropic fixtures proving microcompacted `tool_result` blocks retain their `tool_use_id`, result status, order, and valid Messages API shape.
- [x] 2.5 Add OpenAI-compatible fixtures proving microcompacted tool messages retain their `tool_call_id`, order, and valid Chat Completions shape.
- [x] 2.6 Add zero-summary-call tests proving low-cost reductions stop once the target budget is satisfied.

## 3. Structured Continuation Summary

- [x] 3.1 Add a versioned structured summary prompt with sections for primary intent, constraints, decisions, files and code, errors and fixes, completed work, pending work, and immediate next action.
- [x] 3.2 Implement summary input construction from only the selected contiguous API-round prefix and ensure no tools, thinking, or user turn-level generation options are inherited.
- [x] 3.3 Implement bounded summary parsing and validation for required sections, empty output, malformed output, duplicate sections, and oversized output.
- [x] 3.4 Add tests proving summary diagnostics and parse failures never retain or log raw summary or selected source content.
- [x] 3.5 Preserve `ContextCompaction` invocation accounting for optimizer summary calls without changing provider usage normalization or overlap semantics.

## 4. Authoritative Reinjection

- [x] 4.1 Define focused application ports and neutral reinjection values for current authoritative state kinds without exposing infrastructure concerns to the domain planner.
- [x] 4.2 Implement bounded reinjection for supported OnePiece memory and runtime context using per-kind, per-item, and aggregate budgets.
- [x] 4.3 Record only safe source kind, revision or fingerprint, and size metadata in the optimization plan and diagnostics.
- [x] 4.4 Add tests for current-source replacement, stale-history removal, unavailable sources, invalid revisions, budget overflow, and preservation fallback.

## 5. Candidate Reconstruction and Verification

- [x] 5.1 Implement provider-neutral candidate execution against an immutable prepared request while retaining the original request for fallback.
- [x] 5.2 Add Anthropic reconstruction for summary boundaries, preserved messages, microcompacted results, and reinjected state.
- [x] 5.3 Add OpenAI-compatible reconstruction for the same neutral candidate semantics.
- [x] 5.4 Reproject reconstructed candidates through the phase-one projection and verify complete recursive character coverage.
- [x] 5.5 Implement pure verification of protected and verbatim fingerprint preservation, component order, protocol completeness, plan/action correspondence, and required reinjections.
- [x] 5.6 Implement Token reduction comparison with character-only fallback and reject equal-size, larger, unverifiable, or target-violating candidates.
- [x] 5.7 Add cross-wire contract tests proving equivalent plans yield equivalent action evidence, semantic preservation, protocol state, and reduction outcomes.
- [x] 5.8 Add mutation-style negative fixtures for missing tool results, duplicate tool ids, reordered protected context, changed current intent, missing reinjection, malformed summary boundaries, and uncovered native blocks.

## 6. Runtime Integration and Compatibility Fallback

- [x] 6.1 Extract the current summary-only compaction into an explicit compatibility path with unchanged six-turn retention, notice, memory extraction, and accounting behavior.
- [x] 6.2 Invoke optimizer-first execution only after `maybe_compact_accounted` has crossed the existing 60,000-character trigger.
- [x] 6.3 Use an accepted candidate for the initial or tool-continuation request and keep the existing visible compaction notice path.
- [x] 6.4 Fall back from planner, reduction, reinjection, summary, reconstruction, or verification failures to summary-only compaction using untouched original turns.
- [x] 6.5 Add regression tests proving Token shadow decisions still cannot trigger or suppress compaction and optimizer execution never runs below the character threshold.
- [x] 6.6 Add tests for zero-call microcompaction, one-call structured summarization, failed optimizer summary followed by compatibility fallback, cancellation, and provider error propagation.
- [x] 6.7 Verify managed CLI Agents, frontend service contracts, Tauri commands, Web/mock behavior, and persisted database schemas remain unchanged.

## 7. Safe Evidence and Documentation

- [x] 7.1 Add bounded unified-log projection for optimizer and verifier versions, action and class counts, before/after quality and occupancy, saved amount, invariant flags, safe fingerprints, and fallback stage/reason.
- [x] 7.2 Add redaction tests proving optimizer diagnostics omit request content, summary content, placeholders' source content, tool data, credentials, headers, frames, and raw provider payloads.
- [x] 7.3 Update native Agent architecture documentation with ordered passes, structured summary sections, reinjection lifecycle, verification gate, accounting behavior, and summary-only fallback.
- [x] 7.4 Document the deliberate deferral of token-aware triggering, automatic suppression, cooldown, circuit breaking, evidence UI, and provider-native cache edits.

## 8. Required Verification

- [x] 8.1 Run `npm run lint:ci` and resolve all findings.
- [x] 8.2 Run `npm run test` and `npm run test:coverage`, including optimizer regression suites, and satisfy coverage policy.
- [x] 8.3 Run `npm run build`, `npm run contracts:check`, `npm run coverage:policy:test`, and `npm run version:unit:test` successfully.
- [x] 8.4 Run `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check` and resolve all formatting differences.
- [x] 8.5 Run `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings` and resolve all warnings.
- [x] 8.6 Run `cargo test --manifest-path src-tauri/Cargo.toml` and verify optimizer, both wire formats, compaction, accounting, memory, and cancellation tests pass.
- [x] 8.7 Run `cargo check --manifest-path src-tauri/Cargo.toml` successfully.
- [x] 8.8 Run `npm run docs:check`, `openspec validate add-onepiece-context-optimizer --strict`, and `openspec validate --specs --strict` successfully.
