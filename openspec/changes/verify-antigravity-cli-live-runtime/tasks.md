## 1. Capture the live facts

These must be run by a human in an interactive terminal — `agy`'s keyring auth exceeds its own
10-second budget in a non-interactive session and falls back to unauthenticated.

```powershell
& "$env:LOCALAPPDATA\agy\bin\agy.exe" -p "Reply with exactly: hello" --output-format stream-json *> "$env:TEMP\agy-simple.jsonl"
& "$env:LOCALAPPDATA\agy\bin\agy.exe" -p "List the files in the current directory, then stop." --output-format stream-json *> "$env:TEMP\agy-tools.jsonl"
& "$env:LOCALAPPDATA\agy\bin\agy.exe" models *> "$env:TEMP\agy-models.txt"
```

- [ ] 1.1 Capture a simple authenticated turn and record the exact `init` payload fields
- [ ] 1.2 Capture a tool-using turn and record the exact `step_update` payload fields
- [ ] 1.3 Capture `agy models` output and record the real model slugs

## 2. Replace inferred shapes with observed ones

- [ ] 2.1 Re-pin `fixtures/antigravity-cli.output.jsonl` to the real capture, replacing the
      placeholder `init` and `step_update` lines
- [ ] 2.2 Confirm or correct the `init.conversation_id` field name the parser reads today
- [ ] 2.3 Map `step_update` to incremental output (`Token` / `Thinking` / `ToolLifecycle` as the
      payload warrants), replacing the current deliberate no-op
- [ ] 2.4 Update the `native-runtime-architecture` delta so the spec states what the parser now
      does instead of why it defers
- [ ] 2.5 Add the real model slugs to the `--model` catalog on both the Rust and TypeScript sides

## 3. Verify end to end

- [ ] 3.1 Launch the desktop app against the authenticated install and run a managed chat turn
- [ ] 3.2 Confirm output streams incrementally rather than arriving only at completion
- [ ] 3.3 Confirm reported usage is persisted, with thinking tokens folded into output
- [ ] 3.4 Confirm resume works: a second turn reuses the conversation id from `init`

## 4. Verification

- [ ] 4.1 `npm run lint:ci`
- [ ] 4.2 `npm run test`
- [ ] 4.3 `npm run build`
- [ ] 4.4 `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`
- [ ] 4.5 `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`
- [ ] 4.6 `cargo test --manifest-path src-tauri/Cargo.toml`
- [ ] 4.7 `cargo check --manifest-path src-tauri/Cargo.toml`
- [ ] 4.8 `openspec validate --specs --strict` and `openspec validate verify-antigravity-cli-live-runtime --strict`
- [ ] 4.9 `npx playwright test` with `PLAYWRIGHT_PORT` pinned to a free port — the config defaults
      to 5174 with `reuseExistingServer: true`, and another worktree's dev server there would
      silently test that checkout instead
