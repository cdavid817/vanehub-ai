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
- [ ] 4.5 `openspec validate correct-cli-launch-and-availability-contracts --strict`

## 5. Remaining behaviour to settle

- [ ] 5.1 Decide whether claude-code should stream from `content_block_delta` rather than landing the reply in one piece, and whether that risks double-counting against the final `assistant` event
- [ ] 5.2 Confirm no consumer of `RunnerLaunchSpec` re-serialises arguments into a single string, so admitting line breaks cannot reach a shell through a path this change did not inspect
- [ ] 5.3 Decide whether an Agent reported available but refused by its provider for account reasons should surface that distinction in the registry, rather than only at the first turn

## 6. Archive

- [ ] 6.1 `openspec archive correct-cli-launch-and-availability-contracts` and run `scripts/Update-OpenSpecArchiveIndex.ps1`, committing main specs, archive and index together
