## 1. Pin the locale at the adapter

- [x] 1.1 Set `LC_ALL=C` on the git process in `GitAdapter::execute`, and in `execute_with_environment` apply it before caller-supplied variables so explicit environment still wins.
- [x] 1.2 Add regression coverage that classifies a non-Git directory and an untracked path with a non-English locale in the parent environment (for example `LANG`/`LC_ALL` set to `zh_CN.UTF-8` around the adapter call).

## 2. Verification

- [x] 2.1 Confirm `git_fixtures_cover_non_git_and_common_worktree_states` passes on a zh_CN-locale machine without `--skip`.
- [x] 2.2 Run `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`, `cargo check --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `npm run native:panic:check`, and `cargo test --workspace`.
- [x] 2.3 Run `openspec validate pin-git-message-locale --strict` and `openspec validate --specs --strict`.
