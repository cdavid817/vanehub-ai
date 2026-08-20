# Design

## D1: The global installation model stays

The hook living in Claude Code's global `settings.json` — outliving the VaneHub process and governing every session on the machine — is the original design intent (`add-claude-code-permission-callback` D7; the adapter's own doc comment). This change treats "all sessions degrade to read-only when VaneHub is gone" as correct fail-closed behavior with an unacceptable *recovery cost*, and fixes only the recovery cost. Per-session injection (`claude --settings`) would eliminate the failure class but also the governance property; it is a scope decision for a separate proposal, noted in `proposal.md`'s out-of-scope list.

## D2: The escape hatch lives in the wrapper, not in a new tool

`vanehub-permission-hook --uninstall` reuses the one binary guaranteed to be on disk wherever the problem exists — the installed hook entry names its absolute path, so the stuck user can read the path straight out of the deny situation's `settings.json` (or their PATH'd copy) and run it. There is no privilege escalation: any process able to run the wrapper can already edit `~/.claude/settings.json` directly.

The wrapper deliberately does not link `vanehub_ai_lib`, so three facts are duplicated by hand and must be kept in sync, extending the existing discovery-file duplication contract already documented at the top of `main.rs`:

- the settings path `~/.claude/settings.json` (from `live_config.rs`'s `primary_path("claude-code")`),
- the ownership marker `vanehub-permission-hook` (from `live_config.rs`'s `PERMISSION_HOOK_MARKER`),
- the removal predicate: an entry is VaneHub-owned iff any of its `hooks[].command` strings contains the marker (from `live_config.rs`'s `is_vanehub_owned`).

Uninstall edits only `hooks.PreToolUse`, removing owned entries and leaving every other key — including entries other tools added to the same array — untouched, mirroring `set_permission_hook_entries`'s retain-then-extend semantics with an empty extend. An unparseable settings file fails without writing anything. The write goes through a same-directory temp file plus rename so a crash mid-write cannot truncate the user's settings.

Exit-code contract: hook mode keeps "always exit 0, decision in JSON" (Claude Code's documented contract); uninstall mode is human-invoked and uses conventional exit codes (0 success including nothing-to-remove, nonzero failure with a message on stderr).

## D3: Deny reasons differentiate on signals the wrapper already has

The wrapper can already distinguish "no discovery file" from "discovery present but connection failed" — today both collapse into one `None`. Splitting them costs a two-variant enum and buys precise messages ("VaneHub has not registered on this machine" vs "the VaneHub instance that installed this hook is not running"). A PID-liveness check in the discovery file was considered and rejected: cross-platform liveness probing needs `libc`/`winapi` in a deliberately dependency-minimal binary, and the discovery-present/absent split already carries the diagnostic value. Both messages name both recovery actions so the blocked session's user (or the blocked Claude, relaying to its user) sees the way out in the denial itself.

## D4: Startup reconvergence keys off the assigned principal row

"Hook management enabled" has exactly one durable representation today: the `claude-code` principal has a real (non-synthesized) row, which is what `assign_template` creates when it installs the hook. Startup therefore re-runs `install()` iff `find_principal(CLAUDE_CODE_AGENT_ID)` reports an assigned row. This refreshes the wrapper path after application updates (the entry otherwise keeps naming the old install location forever) and re-heals a hand-mangled entry. It is best-effort in `bootstrap/permissions.rs`, matching the file's existing philosophy for `start_hook_bridge_server`: a failure (for example a missing wrapper binary in a dev build) leaves CLI sessions in the same risk-tiered offline fallback they were already in, and must not fail permissions bootstrap. When no assigned row exists, startup does not touch the settings file at all — a user who never enabled hook management keeps a byte-identical `settings.json`.
