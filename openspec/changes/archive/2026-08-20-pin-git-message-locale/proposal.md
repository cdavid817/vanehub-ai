## Why

`session_queries.rs` classifies Git command outcomes by matching English stderr substrings: `"not a git repository"` decides whether a session root is a non-Git directory (twice), and `"did not match any files"`/`"pathspec"` decides whether a path is untracked. But `GitAdapter::execute` inherits the host environment, so on any machine with a non-English locale (for example `LANG=zh_CN.UTF-8`, where git prints "不是 git 仓库") the substrings never match. The user-visible result violates `session-project-inspection`'s existing "Non-Git session" scenario: instead of the localized non-Git empty state, the Changes tab surfaces a raw "Git status failed." error for every non-Git folder, and untracked-path probes misreport as hard failures. The unit test `git_fixtures_cover_non_git_and_common_worktree_states` fails on such machines for the same reason — CI's English runners never see it.

## What Changes

- Pin the message locale (`LC_ALL=C`) on every Git invocation `GitAdapter` makes, so stderr/stdout classification is stable regardless of the host's display language. Caller-supplied environment in `execute_with_environment` still wins over the pinned default.
- No change to how classified outcomes are presented: user-facing labels stay localized per the existing spec; only the *classification input* becomes locale-independent.
- Add regression coverage that exercises outcome classification under a non-English locale.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `session-project-inspection`: Git inspection outcome classification is locale-independent; the non-Git empty state and untracked detection work on hosts with any display language.

## Impact

- `src-tauri/src/platform/git/mod.rs`: `execute`/`execute_with_environment` pin `LC_ALL=C` for the spawned git process only.
- `src-tauri/src/contexts/workspaces/infrastructure/session_queries.rs`: no logic change expected; its substring matching becomes reliable. Its git fixture test starts passing on non-English machines.
- No Tauri command changes, no schema changes, no frontend changes. Filename handling is unaffected: git emits paths as bytes and callers already pass `core.quotepath=false`.
