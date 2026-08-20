Implementation landed during the 2026-08-19/20 desktop verification pass, ahead of this proposal: each defect was found by running the real client, and the fix shipped with the regression test that reproduced it. The commits are recorded per task so the spec deltas can be checked against what actually changed. Sections 5 and 6 are the parts still open.

## 1. Argument and executable validation

- [x] 1.1 Split launch-value validation so arguments admit tab, CR and LF while the executable, cwd and environment values keep the full control-character prohibition (`d281984f`)
- [x] 1.2 Keep NUL and every non-whitespace control character rejected in arguments, with a test covering a composed multi-line prompt, a NUL, and an escape sequence (`d281984f`)
- [x] 1.3 Measure the executable against the path bound rather than the identifier bound, with a test using a real vendored npm binary path (`86e8b86c`)
- [x] 1.4 Carry the refused constraint on the launch error and render it into the runner lifecycle log (`86e8b86c`)
- [x] 1.5 Carry the spawn failure reason instead of discarding the OS message (`86e8b86c`)

## 2. Agent availability

- [x] 2.1 Invert the availability assessment so the executable decides and the managed SDK decides only when no executable is declared (`d948b41d`)
- [x] 2.2 Report a missing executable against the search path rather than the SDK (`d948b41d`)
- [x] 2.3 Update the pre-existing assessment test that encoded the old order, preserving its coverage of the SDK reasons for the no-executable case (`d948b41d`)

## 3. Provider prompt delivery and output parsing

- [x] 3.1 Move gemini-cli prompt delivery from argv to stdin and update its invocation fixture (`90c7ffad`)
- [x] 3.2 Resolve an unrecognised structured claude-code event to no output, keeping the raw-text fallback for unstructured lines (`0941515e`)
- [x] 3.3 Mark the generic-line fallback test-only now that its last production caller is gone (`dd852fd3`)

## 4. Verification

- [x] 4.1 `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --check`, `npm run native:panic:check`
- [x] 4.2 `npm run lint:ci`, `tsc --noEmit`, `npm run build`, `npm run contracts:check`, `npm run docs:check`
- [x] 4.3 Raise the `agent_runtime/infrastructure` line budget with per-item accounting for the added regression tests and comments (`86e8b86c`, `d8fb8787`)
- [x] 4.4 Confirm on a real desktop client that codex-cli and opencode complete a turn, and that gemini-cli reaches its provider instead of failing at process creation (`test:desktop`, 10/10 spec files)
- [x] 4.5 `openspec validate correct-cli-launch-and-availability-contracts --strict` (valid; `openspec validate --specs --strict` passes 138/138)

## 5. Remaining behaviour to settle

- [x] 5.1 **Decided: not in this change.** The double-counting is not a risk to weigh, it is what a recorded turn does. `--include-partial-messages` is in VaneHub's argv, and the CLI wraps each delta as `{"type":"stream_event","event":{"type":"content_block_delta",...}}` -- the top-level type, which is what the parser dispatches on, so the existing `content_block_delta` arm never fires for it. Unwrapping the envelope would emit the partial `PO` and then the complete `PONG` from the terminal `assistant` event, both recorded verbatim in `providers/tests.rs:1185`. Streaming therefore requires suppressing the terminal event's text as well, which changes what "the reply" is for the transcript and for usage accounting. That is its own change
- [x] 5.2 Swept every consumer of `RunnerLaunchSpec.arguments`; findings recorded in the verification report
- [x] 5.3 **Decided: not in the registry.** Availability is a filesystem probe that answers "is the binary here", and it enumerates every Agent on every listing. Making it answer "will the provider accept this account" would put a network round trip per Agent in that path, and the answer would still be stale by the time a turn starts -- an account can be refused between the listing and the send. The refusal already surfaces where it is current, at the first turn. What would help without either cost is remembering the last observed refusal and showing it as an advisory on the Agent card, which is a session-state feature rather than a registry one, and needs its own proposal

## 6. Archive

- [ ] 6.1 `openspec archive correct-cli-launch-and-availability-contracts` and run `scripts/Update-OpenSpecArchiveIndex.ps1`, committing main specs, archive and index together
