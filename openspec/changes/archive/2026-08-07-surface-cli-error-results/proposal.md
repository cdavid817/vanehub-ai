## Why

When a claude-code CLI run fails, the user is told `Agent CLI exited with status exit code: 1.` and nothing else — even though the CLI stated the reason plainly.

This was found during manual testing on 2026-08-05. Running the exact command the app runs reproduces it:

```
echo "say hi" | claude -p --output-format stream-json --include-partial-messages --verbose
→ exit 1, stdout 8335 bytes, stderr 0 bytes
```

The final stdout line was `{"is_error":true,"terminal_reason":"api_error","api_error_status":403,"result":"Failed to authenticate. API Error: 403 Request not allowed","error":"authentication_failed"}`.

Two gaps turn that into a useless message. `parse_claude_line` maps event type `result` to `Completed` without consulting `is_error`, so the failure is read as a success and the diagnostic is discarded — the adjacent arm handles `type: "error"`, but claude-code does not use it. Then, because the process exited non-zero with empty stderr, the process adapter falls back to a generic message, since it only uses stderr for failure text.

The effect is that any claude-code failure that reports itself in structured output — expired credentials, quota, policy rejection, upstream API errors — reaches the user as an exit code. This affects every CLI execution path, and the diagnostic the user needs was already in hand and thrown away.

A logging gap was observed alongside it: the run recorded only `classification=Retryable`, with no exit code, stdout, or stderr. That observation came from the coordination path, which has since been removed, and `unified-log-management` already requires exit status and unsuccessful-exit diagnostics to be persisted. Whether the ordinary chat path complies is unverified, so this change verifies it and fixes the code only if it does not — it does not propose a new logging requirement for something the specs already mandate.

## What Changes

- Treat a claude-code `result` event carrying `is_error: true` as a failure rather than a completion, and surface the CLI's own `result` text as the failure diagnostic.
- Classify such a failure using the structured codes the payload already carries, so an authentication or policy failure is not retried as though it were transient.
- Fall back to the parsed diagnostic when a process exits non-zero with empty stderr, instead of reporting only the exit status.
- Verify that a failed managed Agent process already persists its exit code and redacted diagnostics as `unified-log-management` requires, and correct the code if it does not. No new logging requirement is proposed.
- **Not a behaviour change for successful runs.** A `result` event without `is_error` continues to complete exactly as today, including its token accounting.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `session-runtime-management`: Adds a claude-code output-normalization requirement covering error results, alongside the existing Codex and OpenCode normalization requirements, and requires the process adapter to prefer a parsed diagnostic over a bare exit status.

`unified-log-management` is deliberately absent: its "Persist runtime stdout and stderr" and "Persist unsupported runtime diagnostics" requirements already cover exit status and unsuccessful exits. If the code does not honour them, that is a compliance defect to fix, not a requirement to add.

## Impact

- **Desktop runtime only.** The Web/mock adapter does not spawn processes and is unaffected; no `AgentService` method signature changes, so both adapters stay interface-identical without edits.
- **Backend:** `providers/output.rs` (the `result` arm and its classification) and `process_adapter.rs` (the empty-stderr fallback and the exit telemetry). Both are shared by every CLI agent path — chat, scheduled tasks, and the Loop runtime all benefit.
- **Frontend:** none. The improved text flows through the existing message error field.
- **Reuse:** `provider_failure()` already extracts a diagnostic from `/result`, `/error`, and `/error/message`, and `structured_error_codes()` already inspects `/error/code`, `/error/status`, `/error/type`, and `/error/reason`. The `result` arm simply never called them; this change wires up machinery that exists rather than adding a parallel path.
- **Risk of over-reporting:** raw CLI output can contain paths or credentials, so the logged excerpt must go through the existing redaction and length bounds rather than being written verbatim.
- **Not in scope:** the 403 itself. That is a local credential problem for the operator to resolve; this change only ensures the product says so.
