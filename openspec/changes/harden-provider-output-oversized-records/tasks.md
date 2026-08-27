## 1. Framer degradation semantics

- [x] 1.1 Rework `ProviderOutputFramer` so an overflow drops the current record through its terminating newline and resumes parsing, with a discard counter exposed to the consumer.
- [x] 1.2 Update the framer unit tests that asserted fail-closed overflow to assert the skip behavior (mid-chunk newline, cross-push oversize, unterminated tail), keeping the invalid-UTF-8 fail-closed test.

## 2. Stream consumer

- [x] 2.1 Replace the hardcoded 256 KiB bound in the generation read loop with the parser policy domain maximum (1 MiB).
- [x] 2.2 After the stream ends, record a redacted `warn` unified-log entry naming the number of discarded oversized records for the generation.
- [ ] 2.3 Wire the bound to the provider's declared `ProviderParserPolicy` instead of a constant, so a future per-provider policy change reaches the read loop.

## 3. Verification

- [x] 3.1 `cargo check -p vanehub-ai` and the reworked `providers::output` unit tests pass locally.
- [x] 3.2 A real three-seat desktop session that previously died on an oversized claude-code tool result completes end to end (desktop-multi-agent-longrun layer, 2026-08-24 evidence).
- [x] 3.3 Run the full validation command set from AGENTS.md, then `openspec validate harden-provider-output-oversized-records --strict` (2026-08-25: all green; the architecture line-budget raise for `agent_runtime/infrastructure` rides in the same commit with its reason recorded).
