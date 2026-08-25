# Harden provider output parsing against oversized records

## Why

A real multi-agent session died mid-turn because one stream-json record from claude-code — a tool result carrying a large file read — exceeded the CLI output parser's per-record bound. The framer treated the overflow as a protocol error and failed the whole generation, so a seat turn that had already produced eight correct file edits was reported as "Claude Code command failed" with no usable diagnostic. Two aggravating defects were found alongside: the streaming read loop hardcodes a 256 KiB bound instead of consulting the provider's declared parser policy (whose domain maximum is 1 MiB), and a dropped record left no trace in unified logs.

## What Changes

- Treat an oversized provider output record as a degradation, not a protocol failure: drop the record through its terminating newline, keep parsing subsequent records, and let the turn complete on the stream's real terminal event.
- Count dropped records and report them after the stream ends as a redacted `warn` in unified logs, so a drop is never silent.
- Align the streaming read loop's record bound with the parser policy's domain maximum (1 MiB) instead of a hardcoded 256 KiB.
- Keep fail-closed behavior for genuinely malformed output (invalid UTF-8) unchanged.

## Capabilities

### New Capabilities

- None.

### Modified Capabilities

- `agent-provider-runtime`: Require graceful degradation for oversized output records during CLI generation streaming, with the drop surfaced through unified redacted diagnostics.

## Impact

- Affects `src-tauri/src/contexts/agent_runtime/infrastructure/providers/output.rs` (framer skip-and-continue semantics, discard counter) and `src-tauri/src/contexts/agent_runtime/infrastructure/process_adapter.rs` (bound value, post-stream warn log).
- No Tauri command signature, frontend, or database change; the Web/mock adapter is unaffected.
- Existing framer unit tests that asserted the old fail-closed overflow behavior are updated to assert the skip behavior instead.
