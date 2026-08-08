## Context

`ClaudeCodeHookAdapter` writes `PreToolUse` entries into Claude Code's global `settings.json` through `cli_config`'s hook-projection port. Each entry's `command` is `wrapper_binary_path(app)`, resolved in `bootstrap/permissions.rs` as the Tauri resource directory joined with the binary name, falling back to the running executable's directory.

`tauri.conf.json` declares neither `bundle.resources` nor `bundle.externalBin`, so no packaged build contains the wrapper. In development the fallback happens to land on `target/<profile>/`, where `cargo` has just built it, which is why this never surfaced before a package was installed.

Installation is not automatic: `PermissionsApi::assign_template` calls `install()` only for the `claude-code` agent, and `permissions-approval` requires a distinct first-use confirmation. The damage is therefore opt-in — but when taken, it persists in a file VaneHub does not own.

## Goals / Non-Goals

**Goals:**

- Never name a non-existent command in another tool's global configuration.
- Say why, at the moment the user asks for it.
- Keep the escape hatch open for anyone already affected.

**Non-Goals:**

- Shipping the wrapper binary. That is the real fix and is deliberately separate; see Open Questions.
- Changing how the path is resolved. The current resolution is also wrong for a sidecar, but correcting it without shipping the binary would change one wrong path for another.
- Any Web-runtime behaviour. That runtime has no permission-hook surface.

## Decisions

### Check in the adapter, not at the call site

`install()` verifies `wrapper_path.is_file()` before delegating to the projection. The adapter is the only place that knows what the entries contain and which binary they name, so it is the only place that can check the two are consistent. A caller-side check would have to duplicate the path and stay in sync with it.

`is_file()` rather than `exists()`: a directory at that path would satisfy `exists()` and still be unexecutable.

### Guard `install` only, never `remove`

`remove()` writes an empty entry list and names no binary, so it has no precondition to violate. Guarding it would strand exactly the users this change exists for — anyone who enabled the hook against a build that wrote it — with entries they can no longer clear from inside the app.

### Report through the existing infrastructure error

The failure is returned as `PermissionsApplicationError::Infrastructure { category: "cli_config" }`, which `assign_template` already propagates to the command layer. No new error variant, no new command, and the existing frontend path surfaces it.

The message names the resolved path. A user who reports "hook management won't turn on" then arrives with the location that was checked, which is what distinguishes "this build doesn't ship it" from "it was there and moved".

## Risks / Trade-offs

- **A previously working setup now reports an error.** → Only where the binary is genuinely gone. If it was working, the file is present and nothing changes.
- **The check races a delete between the test and the write.** → Accepted. The window is microseconds, the consequence is the pre-existing behaviour, and closing it would mean holding a lock over another tool's configuration file.
- **Reporting an error is not the same as making the feature work.** → It is not meant to be. The feature is unavailable in packaged builds either way; this change makes that visible instead of leaving a broken hook behind. The preview release notes state it plainly.

## Open Questions

- Shipping the wrapper as a Tauri sidecar needs `bundle.externalBin`, a step that builds it per target and renames it to `vanehub-permission-hook-<triple>`, and a corrected resolution order — a sidecar lands beside the main executable, so preferring the resource directory would still miss it. All four packaging targets need to be validated. Worth doing before `0.1.0`; too large to fold in here.
