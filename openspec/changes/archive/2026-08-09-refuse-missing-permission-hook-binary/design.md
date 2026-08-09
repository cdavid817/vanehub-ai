## Context

`ClaudeCodeHookAdapter` writes `PreToolUse` entries into Claude Code's global `settings.json` through `cli_config`'s hook-projection port. Each entry's `command` is `wrapper_binary_path(app)`, resolved in `bootstrap/permissions.rs` as the Tauri resource directory joined with the binary name, falling back to the running executable's directory.

`tauri.conf.json` declares neither `bundle.resources` nor `bundle.externalBin`, so no packaged build contains the wrapper. In development the fallback happens to land on `target/<profile>/`, where `cargo` has just built it, which is why this never surfaced before a package was installed.

Installation is not automatic: `PermissionsApi::assign_template` calls `install()` only for the `claude-code` agent, and `permissions-approval` requires a distinct first-use confirmation. The damage is therefore opt-in — but when taken, it persists in a file VaneHub does not own.

## Goals / Non-Goals

**Goals:**

- Never name a non-existent command in another tool's global configuration.
- Say why, at the moment the user asks for it.
- Keep the escape hatch open for anyone already affected.
- Ship the wrapper in every supported Tauri package without committing platform binaries.

**Non-Goals:**

- Running or supervising the wrapper as a Tauri sidecar process; Claude Code launches the executable named in its hook configuration.
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

### Prepare a target-qualified Tauri external binary before invoking the CLI

Tauri `bundle.externalBin` expects a source named `vanehub-permission-hook-<target-triple>` (plus `.exe` on Windows) and installs it beside the main executable under the unsuffixed name. A Node preparation script builds only the wrapper binary for the requested Rust target and copies it to the ignored `src-tauri/binaries` staging directory. Every npm entry that invokes `tauri dev` or `tauri build` runs this preparation first and merges a sidecar-specific Tauri config overlay; explicit package scripts pass their matrix target, while host builds derive the target from `rustc -vV`.

The base `tauri.conf.json` deliberately omits `externalBin`. Cargo-only commands still execute Tauri's build script, and putting the declaration in the base config would make `cargo test`, `cargo check`, and Clippy fail before the preparation script can run. The overlay confines the generated-file precondition to Tauri CLI entry points that actually prepare it.

Generated sidecar binaries remain outside Git. The workflow already calls the per-target npm package scripts, so Windows x64, macOS arm64/x64, and Linux x64 all exercise the same preparation path before bundling.

### Resolve beside the executable before the resource directory

An installed external binary lives beside the main executable, not under Tauri's resource directory. Runtime resolution therefore prefers the main executable's parent and uses the resource directory only as a compatibility fallback. Development preparation builds the wrapper into the same Cargo target profile as the application, preserving the same lookup rule in `tauri dev`.

## Risks / Trade-offs

- **A previously working setup now reports an error.** → Only where the binary is genuinely gone. If it was working, the file is present and nothing changes.
- **The check races a delete between the test and the write.** → Accepted. The window is microseconds, the consequence is the pre-existing behaviour, and closing it would mean holding a lock over another tool's configuration file.
- **The target-qualified staging file can become stale.** → Preparation always rebuilds and overwrites the exact target file before Tauri starts, and packaging entry points do not bypass that script.
- **A package can still be damaged after installation.** → The adapter's `is_file()` guard remains authoritative and leaves Claude Code settings untouched.

## Open Questions

None.
