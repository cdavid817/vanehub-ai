## Context

`parse_claude_line` in `providers/output.rs` translates claude-code's `--output-format stream-json` lines into `ProviderOutputEvent`s. Its `result` arm is:

```rust
"result" | "complete" | "completed" => ProviderOutputEvent::Completed(claude_usage(&value)),
"error" | "failed" => ProviderOutputEvent::Failed(provider_failure(&value, "Agent output reported an error.")),
```

The arms are ordered by event type, and claude-code reports failures through `result` with an `is_error` flag rather than through `error`, so the failure path is unreachable for this CLI. Downstream, `process_adapter.rs` composes the terminal event: it prefers a `terminal_error` if one was raised, otherwise checks the exit status, and for a non-zero exit uses stderr when non-empty and a formatted exit status when empty. claude-code writes nothing to stderr, so the generic message always wins.

The fix is narrow because the pieces already exist. `provider_failure()` extracts a diagnostic through `error_value()` (`/result`, `/error`, `/error/message`) and classifies through `structured_error_codes()` (`/error/code`, `/error/status`, `/error/type`, `/error/reason`). The `result` arm has simply never called them.

## Goals / Non-Goals

**Goals:**
- Make a self-reported CLI failure reach the user as the CLI described it.
- Classify authentication and policy failures as non-retryable so failover and retry do not mask them.
- Leave successful runs, including their token accounting, byte-for-byte unchanged.

**Non-Goals:**
- Normalizing other CLIs' error shapes. Codex and OpenCode have their own requirements and were not observed failing this way; changing them without evidence would be speculative.
- Resolving the 403 that surfaced this defect. That is a local credential problem.
- Adding a logging requirement. `unified-log-management` already mandates exit status and unsuccessful-exit diagnostics; this change verifies compliance and fixes code if needed.

## Decisions

### Branch on `is_error` inside the `result` arm rather than adding an arm

Adding `result` to the existing `"error" | "failed"` arm would misroute successful results. Splitting on the flag inside the `result` arm keeps one place responsible for the event type and makes the success path visibly unchanged.

The `error` payload shape claude-code emits does not fit `structured_error_codes()` cleanly: it carries a top-level `error: "authentication_failed"` string and a top-level `api_error_status: 403`, not a nested `error` object. The classifier's probe list therefore needs those two top-level paths added, or an authentication failure will be classified retryable and retried pointlessly.

*Alternative — treat every non-zero exit as failure regardless of parsing:* rejected. It would report an exit status rather than the CLI's own words, which is the actual complaint.

### Let a parsed failure win over the exit-status fallback

`process_adapter.rs` already prefers `terminal_error` when one was raised, so once the parser emits `Failed`, the diagnostic propagates without further change. The remaining edit is to keep a parsed diagnostic from being overwritten when the exit-status branch runs, and to leave the exit-status message as the last resort when genuinely nothing else is known.

### Verify logging before changing it

The thin log observed during testing came from the coordination path, which has since been deleted. `unified-log-management` already requires exit status and unsuccessful-exit diagnostics. Reproduce a failure on the ordinary chat path first: if the exit code and redacted diagnostics are already persisted, there is nothing to do; if not, it is a compliance fix, not a new requirement.

## Risks / Trade-offs

- **Reclassifying failures as non-retryable changes retry behaviour** → Intended: retrying an expired credential wastes time and hides the cause. Scoped to payloads carrying an explicit non-retryable code, so unclassified failures keep today's retryable default.
- **CLI output can contain paths or credentials** → The diagnostic reaches the user through the existing message error field and the log through existing redaction and bounds; neither is written verbatim.
- **`claude_usage()` currently runs on every `result`** → An error result reports zeroed usage. Skipping accounting on the failure branch avoids recording a zero-token turn as a real one; the success branch is untouched.
- **Only claude-code was observed failing this way** → Codex and OpenCode may have equivalent shapes. Rather than guess, this change fixes what was reproduced and leaves a note; a later change can extend it with evidence.

## Migration Plan

1. Add a failing test that feeds a real captured `is_error: true` result line through the parser and expects `Failed` with the CLI's text.
2. Split the `result` arm on `is_error` and extend the classifier's probe paths for the top-level `error` and `api_error_status` fields.
3. Add a test for the adapter path proving a parsed diagnostic survives a non-zero exit with empty stderr.
4. Reproduce a real failure and confirm the message and the log both name the cause.

**Rollback:** revert. No persisted data or schema is involved.

## Open Questions

- Should the user-facing message include the status code (`API Error: 403`) or only the sentence? Including it aids support, and the CLI already puts it in the text, so this change passes the text through unmodified.
