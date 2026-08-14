## 1. Production Trigger Domain

- [x] 1.1 Rename the shadow decision vocabulary to a production compaction decision and bump the policy version
- [x] 1.2 Preserve threshold, reserve, buffer, unknown-capacity, and characters-only reason semantics
- [x] 1.3 Add a pure authoritative selector that chooses Token-aware evidence or character fallback
- [x] 1.4 Unit-test authoritative true, authoritative false, unknown-capacity fallback, and character-only fallback
- [x] 1.5 Update context snapshot exports and comparison diagnostics for the production decision vocabulary

## 2. Automatic Compaction Control

- [x] 2.1 Add provider-neutral automatic/suppressed mode to the generation process request
- [x] 2.2 Default every production generation constructor to automatic mode
- [x] 2.3 Add a generation-local compaction state with success baseline, consecutive failures, and open-circuit state
- [x] 2.4 Implement the 8,192-character minimum-growth cooldown policy
- [x] 2.5 Implement the two-consecutive-failure circuit policy and success reset
- [x] 2.6 Unit-test suppression, cooldown boundary, failure threshold, success reset, and fresh-generation defaults

## 3. Trigger Integration

- [x] 3.1 Build and analyze the exact prepared request snapshot before each compaction check
- [x] 3.2 Reuse the latest correlated provider usage anchor for tool-continuation compaction decisions
- [x] 3.3 Apply the Token-aware selector before the optimizer and retain character fallback for insufficient evidence
- [x] 3.4 Ensure sufficient Token-aware false evidence suppresses legacy character-triggered compaction
- [x] 3.5 Ensure sufficient Token-aware true evidence can trigger below the legacy character threshold
- [x] 3.6 Preserve the unchanged request when no safe old-round boundary exists

## 4. Typed Outcomes and Safety Guards

- [x] 4.1 Introduce typed not-eligible, bypassed, compacted, failed, and terminal compaction outcomes
- [x] 4.2 Refactor optimizer success and compatibility success to report an installed candidate explicitly
- [x] 4.3 Stop compatibility fallback from re-evaluating the legacy character trigger
- [x] 4.4 Update cooldown and circuit state only from typed attempt outcomes
- [x] 4.5 Ensure suppressed, cooldown, and open-circuit paths make zero summary-provider calls
- [x] 4.6 Preserve event-sink failures as terminal generation failures

## 5. Diagnostics and Documentation

- [x] 5.1 Add bounded reason codes for trigger source, request suppression, cooldown, insufficient boundary, failure, and open circuit
- [x] 5.2 Record policy version, measurement quality, occupancy evidence, legacy comparison, cooldown growth, failure count, and circuit state through unified logging
- [x] 5.3 Add tests proving control diagnostics exclude prompt, tool, summary, credential, and provider-payload content
- [x] 5.4 Update native-agent architecture documentation with active trigger and generation-scoped controls
- [x] 5.5 Confirm Web/mock behavior remains unchanged and document provider-native cache edits and evidence UI as deferred

## 6. Regression and Verification

- [x] 6.1 Add Anthropic fixtures for Token trigger, character fallback, suppression, cooldown, and circuit behavior
- [x] 6.2 Add OpenAI-compatible fixtures for Token trigger and character fallback parity
- [x] 6.3 Run focused Rust context-measurement and API process adapter tests
- [x] 6.4 Run `npm run lint:ci`, `npm run test`, `npm run test:coverage`, and `npm run build`
- [x] 6.5 Run `npm run contracts:check`, `npm run coverage:policy:test`, `npm run version:unit:test`, and `npm run docs:check`
- [x] 6.6 Run `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`
- [x] 6.7 Run `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`
- [x] 6.8 Run `cargo test --manifest-path src-tauri/Cargo.toml` and `cargo check --manifest-path src-tauri/Cargo.toml`
- [x] 6.9 Run `openspec validate add-onepiece-context-trigger-and-suppression --strict` and `openspec validate --specs --strict`
- [x] 6.10 Run `git diff --check` and review the final worktree diff for scope and secret safety
