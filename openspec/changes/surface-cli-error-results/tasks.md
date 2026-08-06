## 1. Reproduce and Capture

- [ ] 1.1 Reproduce the failure with `echo "say hi" | claude -p --output-format stream-json --include-partial-messages --verbose`, confirming a non-zero exit with empty stderr and an `is_error: true` result line on stdout.
- [ ] 1.2 Save the captured result line as a test fixture so the parser tests run against real CLI output rather than a hand-written guess.

## 2. Parser Fix

- [ ] 2.1 Add a failing test asserting that a `result` line with `is_error: true` parses to a failed event carrying the CLI's own result text.
- [ ] 2.2 Add a failing test asserting that a `result` line without `is_error` still parses to a completed event with its token accounting unchanged.
- [ ] 2.3 Split the `result` arm in `parse_claude_line` on `is_error`, routing the error case through `provider_failure()` and skipping usage accounting for it.
- [ ] 2.4 Add a failing test asserting an authentication failure is classified non-retryable.
- [ ] 2.5 Extend `structured_error_codes()` to probe the top-level `error` string and `api_error_status` number that claude-code emits, keeping the existing nested-object paths.

## 3. Process Adapter Fix

- [ ] 3.1 Add a failing test asserting that a non-zero exit with empty stderr reports a previously parsed diagnostic rather than the exit status.
- [ ] 3.2 Make the terminal-event composition prefer a parsed failure diagnostic, leaving the exit-status message as the last resort.
- [ ] 3.3 Confirm a non-zero exit with no diagnostic and no stderr still reports the exit status, so the fallback is not lost.

## 4. Logging Compliance Check

- [ ] 4.1 Reproduce a CLI failure on the ordinary chat path and inspect the unified log for the exit code and redacted diagnostics that `unified-log-management` already requires.
- [ ] 4.2 If either is absent, fix the code to comply; if both are present, record that no change was needed rather than inventing work.

## 5. Verification

- [ ] 5.1 Run `npm ci` first and confirm `node_modules/.pnpm` is absent, so build verification is trustworthy.
- [ ] 5.2 Run `cargo test`, `cargo check`, and `cargo clippy --manifest-path src-tauri/Cargo.toml`.
- [ ] 5.3 Run `npm run lint`, `npm run test`, and `npm run build` to confirm the frontend is untouched.
- [ ] 5.4 Run `openspec validate surface-cli-error-results --strict` and `openspec validate --specs --strict`.
- [ ] 5.5 With a genuinely failing credential, start a chat session in the desktop client and confirm the user-visible error names the cause instead of an exit code.
- [ ] 5.6 With a working credential, confirm a successful chat still streams and completes with unchanged token accounting.
