## 1. Wrapper escape hatch and deny messaging

- [x] 1.1 Add `--uninstall` mode to `crates/vanehub-permission-hook`: path-injectable settings surgery that removes only marker-owned `PreToolUse` entries via a temp-file-plus-rename write, with tests for removal, preservation, idempotent no-op, and unparseable-file refusal.
- [x] 1.2 Split the wrapper's offline state into no-discovery vs unreachable and emit distinct deny reasons that name both recovery actions, with tests covering both variants and the unchanged allowlist behavior.

## 2. Desktop startup reconvergence

- [x] 2.1 Add a `PermissionsApi` reconvergence entry point that re-runs `install()` iff the `claude-code` principal has an assigned row, with tests for the assigned, unassigned, and failing-install cases.
- [x] 2.2 Wire the reconvergence best-effort into `bootstrap/permissions.rs` startup assembly.

## 3. Verification

- [x] 3.1 Run `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`, `cargo check --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `npm run native:panic:check`, and `cargo test --workspace` (one pre-existing, unrelated failure skipped: `git_fixtures_cover_non_git_and_common_worktree_states` breaks on zh_CN locale machines because git localizes "not a git repository" — everything else green).
- [x] 3.2 Run `openspec validate add-permission-hook-recovery --strict` and `openspec validate --specs --strict`.
