## Context

VaneHub AI carries four built-in CLI agents. Each one was absorbed into the same set of layers: a built-in catalog row in `agents`, a managed chat invocation contract plus output parser, an installation/version lifecycle, a global configuration profile, a typed launch-parameter profile, and a policy-template projection. Those layers each keep their own hard-coded list of which agent ids they cover, and the four lists are not identical — `managedCliAgentIds` has four entries while `cliConfigAgentIds` has three, because `gemini-cli` has no relay-style configuration profile.

Antigravity CLI (`agy`) is Google's successor to Gemini CLI. Integrating it is mostly a matter of adding a fifth entry to those lists, but three of its properties fall outside what the existing layers assume:

1. **No package-manager distribution.** Installer script only — `install.sh` on Unix, `install.ps1` on Windows. Binary lands in `~/.local/bin/agy` or `%LOCALAPPDATA%\agy\bin`. Today `ToolDefinition.package_name` is a required `&'static str` and `install_command_for` unconditionally appends an `|| npm install -g <package>` fallback; the script-install path in `package_adapter.rs` runs through `bash -lc`, which is why Claude Code relies on `winget_package_id` for Windows. A CLI with neither npm nor winget currently has no automated Windows install path at all.
2. **No API-key authentication.** Credentials live in the OS keyring behind Google Sign-In. `GEMINI_API_KEY` is explicitly ignored. Every existing configuration profile kind models a credential.
3. **No usable endpoint redirection.** The wire protocol is Google CodeAssist (`cloudcode-pa.googleapis.com`), not OpenAI Chat/Responses, Anthropic Messages, or the Gemini API. `CLOUD_CODE_URL` overrides the endpoint but the binary still demands a valid Google OAuth token first, so a third-party relay cannot satisfy it.

The runtime boundary is unchanged: React components talk to `src/services/agent-service.ts`; the Tauri adapter and the Web/mock adapter must expose the same surface; CLI detection, process launch, SQLite, and configuration-file I/O stay in Rust.

## Goals / Non-Goals

**Goals:**

- `antigravity-cli` behaves as a first-class built-in CLI agent everywhere the other four appear, with no surface silently treating the roster as four.
- Its configuration profile manages settings the CLI actually honors, rather than mirroring a relay-profile shape that cannot function.
- Installation management gains genuine support for script-only CLIs on both Unix and Windows, rather than special-casing this one agent.
- Capability differences (no credential, no endpoint override) are expressed as data on the profile kind, so UI stays free of `agentId === "antigravity-cli"` branches.

**Non-Goals:**

- Interactive embedded-terminal reported-usage ingestion (transcript location undocumented; deferred).
- Provider-endpoint/relay profiles for this CLI.
- Consolidating the several built-in-CLI id arrays into one capability matrix.
- Any change to how the other four CLIs are invoked, configured, or installed, beyond mechanical adaptation to the widened `ToolDefinition` shape.

## Decisions

### D1. The configuration profile manages local settings, not a provider endpoint

`CliConfigPayload` gains an `Antigravity` variant holding `tool_permission`, `enable_terminal_sandbox`, `verbosity`, `model`, and a pass-through map for unmodelled keys. `primary_path` resolves to `~/.gemini/antigravity-cli/settings.json`; the fragment builder reuses the JSON handling already written for Claude Code's `settings.json` rather than the TOML or JSON5 paths. Drift detection, import, malformed-document reporting, and startup sync all work unchanged, because they operate on the fragment abstraction rather than on payload semantics.

*Alternatives considered.* **Exclude the agent from the configuration layer entirely** — smallest change, and defensible since `gemini-cli` is already excluded, but it leaves the settings page with a visible hole for an agent the user is expected to configure. **Model a relay profile anyway, for shape symmetry** — rejected outright: it would render a base-URL field and a credential field for a CLI that ignores both, which is a control that lies about what it does.

### D2. Capability differences are declared, not branched on

Rather than teaching the configuration dialog that `antigravity-cli` has no credential, the profile kind declares `supportsCredential: false` and `supportsEndpointOverride: false`, and the dialog renders from those declarations. `validationState` for such a kind is `valid` or `invalid` only — `needs-credential` is unreachable, and `credentialConfigured` is permanently `false`.

*Alternative considered:* a conditional on the agent id inside the dialog. Rejected — it violates the project's preference for additive adapter changes over agent-specific UI branches, and the same conditional would then have to be duplicated in validation, in the profile list, and in the startup-sync summary.

### D3. Installation: widen `ToolDefinition` rather than special-case the agent

`package_name` becomes `Option<&'static str>`, and a `powershell_install_url` field joins `script_install_url`. `install_command_for` emits the npm fallback only when a package name exists. On Windows, a definition carrying `powershell_install_url` becomes `Wget`-eligible through a PowerShell invocation instead of `bash -lc`.

This edits all four existing catalog entries' construction sites — a mechanical but repo-wide diff. Taking the change now is cheaper than the alternative, which is a parallel "script-only CLI" code path that would immediately need its own conflict detection, version probing, and eligibility derivation.

*Alternative considered:* leave Windows install manual and surface the documented PowerShell one-liner as copyable guidance. This is the fallback if PowerShell-driven installation proves unreliable under the app's process-spawning constraints (the codebase already had to suppress console windows raised by capability probes), and it is the recommended de-scope if D3 runs long.

### D4. Invocation uses `stream-json`, prompt as argument, resume by conversation id

`agy [managed parameters] --conversation <id> -p <prompt> --output-format stream-json`. Prompt travels as an argument, matching `gemini-cli` and `opencode` rather than the stdin delivery used for `claude-code` and `codex-cli`, because `-p` takes the prompt as its value. The invocation contract is pinned by a fixture in `fixtures/invocations.json` so argument order cannot drift unnoticed.

*Alternative considered:* `--output-format json`, a single terminal object. Simpler to parse and immune to any `step_update` schema uncertainty, but it yields no incremental output, so the chat surface would sit blank until completion. Streaming is worth the schema risk; the parser treats unknown `step_update` fields as ignorable rather than as errors, which contains that risk.

### D5. Status vocabulary maps to lifecycle states explicitly

`SUCCESS` completes. `ERROR` and `INVALID` fail, preserving `result.error`. `CANCELED` and `INTERRUPTED` map to a stopped state rather than a failure, so a user-initiated stop does not surface as an error. `WAITING` and `RUNNING` are non-terminal and must not appear on a `result` event; if one does, it is treated as a protocol violation and reported as a failure rather than silently ignored.

### D6. `thinking_tokens` folds into output

Antigravity reports `input_tokens`, `output_tokens`, `thinking_tokens`, `cache_read_tokens`, and `total_tokens`. Reasoning tokens fold into the output count, consistent with the existing treatment of codex-cli, opencode, and gemini-cli reasoning tokens, so cross-agent usage comparisons stay meaningful. `total_tokens` is recorded as reported rather than recomputed.

### D7. Policy projection uses `--mode`, Antigravity's own graduated execution control

**Revised after installing the real CLI (v1.1.11).** The published documentation lists only `--sandbox` and `--dangerously-skip-permissions` as launch-time overrides, which would have left `standard`, `trusted`, and `yolo` projecting identically — a real fidelity loss this design originally accepted and recorded. `agy --help` on the installed binary shows a third, undocumented-on-the-web flag:

```
--mode  Set the agent execution mode for this session (accept-edits, plan)
```

That is exactly the catalog-legal graduated control the other three CLIs use (`--permission-mode`, `--approval-mode`, `--auto`), so the projection uses it and the four templates stay distinguishable:

| Template | Projection |
|---|---|
| `readonly` | `--mode plan` + `--sandbox` — plans without applying changes, contained |
| `standard` | no mode override + no sandbox — the CLI's own `request-review` default asks before acting |
| `trusted` / `yolo` | `--mode accept-edits` — auto-approves edits, matching how the other CLIs collapse these two |

`--dangerously-skip-permissions` is never projected, and is excluded from the parameter catalog entirely: the catalog asserts for every managed CLI that no exposed flag contains "dangerously" (`cli_parameters.rs`), and `--mode` now provides the permissive posture without it.

Verified against the installed binary: `request-review`, `proceed-in-sandbox`, `always-proceed`, and `strict` all appear in `agy.exe`, confirming the settings-side vocabulary this design models; `accept-edits` and `plan` appear as the `--mode` values.

*Lesson recorded rather than quietly fixed:* the original table was derived from published docs and was materially wrong about what the tool can express. The flags that matter most for a security projection were the ones the docs omitted.

This is a real fidelity loss and is recorded as such rather than papered over. The `opencode` precedent — injecting `OPENCODE_PERMISSION` to express a posture the flag set lacked — has no documented Antigravity equivalent; if implementation finds one (an env var, or a settings key honored per-invocation), `trusted`/`yolo` should be separated from `standard` and this table revised.

### D8. No migration is added; registry seeding already back-fills

Investigation during implementation showed a migration is unnecessary. `seed_registry` runs on every bootstrap — `Database::new` calls it right after `migrate` (`src-tauri/src/platform/database/mod.rs:75`) — and every insert it performs is `INSERT OR IGNORE`, covering `agents`, `agent_modes`, and `agent_capability_tags` alike. Adding the catalog entry therefore back-fills databases created before Antigravity existed, and re-running is a no-op, which is exactly the idempotence the spec requires. Adding a migration on top would duplicate that work and claim a version number for nothing.

The version-number check was still worth running, and it confirmed the hazard is live rather than theoretical: the shared `%APPDATA%\ai.vanehub.app\vanehub.sqlite` already records version **49** (`workspace-code-index-foundation`), applied by a concurrent worktree branch that has not merged. This branch's own file lists migrations only through 48, so incrementing from the source would have produced a 49 that is permanently skipped on this machine. **The next migration added on any branch must start at 50.** The codebase already carries a comment recording the same class of collision at version 42/43.

*Alternative considered:* add a migration anyway, for an explicit audit trail of when the agent appeared. Rejected — the seeding path already guarantees the outcome, and a redundant migration would consume a version number that the collision above makes scarce.

## Risks / Trade-offs

- **Undocumented `step_update` payload shape** → Parser ignores unknown fields and never fails a run on an unrecognized event kind; a live capture during implementation replaces the provisional mapping, and fixtures are pinned to whatever that capture actually contains.
- **Unverified `agy --version` / `agy update`** → A wrong version command degrades into the existing `VersionCheckStatus::Failed` path, which the UI already renders as diagnosable rather than fatal. Confirm against a real install before the change is archived.
- **PowerShell-driven install may raise a console window or trip execution policy** → D3's stated fallback is manual guidance on Windows; the install path is behind lifecycle eligibility, so degrading it does not affect detection or version reporting.
- **Unauthenticated runs may surface as generic launch failures** → Until the exit code and stderr signature are confirmed, an unauthenticated run reports as a launch failure with the CLI's own stderr preserved, which is unhelpful but not wrong. Mapping it to `needs-authentication` is a follow-up within this change once verified.
- **Widening `ToolDefinition` touches all four existing CLI definitions** → Mechanical diff, fully covered by the existing catalog-ordering and lifecycle-eligibility unit tests; those tests must be updated to five entries, and their assertions are what prove the other four were not disturbed.
- **Fourteen capabilities carry delta specs** → Most are single-enumeration extensions. The substantive ones are `cli-agent-config-management`, `settings-cli-management-ui`, `native-runtime-architecture`, and `cli-agent-permission-launch-flags`; the rest should be reviewed as a batch.
- **Google may change the installer URL or the settings path** → Both are data in a single catalog entry and a single path resolver, not scattered constants.

## Migration Plan

1. No schema migration is added (D8). The catalog entry alone back-fills existing databases through the bootstrap seeding path.
2. The row is inert until a user selects the agent: availability probing reports `Command 'agy' was not found on PATH.` on machines without it, which is the same state any uninstalled CLI already produces.
3. **Rollback:** reverting the code is safe on its own; the seeded row becomes an orphan the older build does not recognize. Sessions created against `antigravity-cli` before a rollback would reference an unknown agent id, so rollback after real use requires either keeping the row or archiving those sessions. Given the row is inert without the binary, keeping it is the recommended rollback posture.

## Open Questions

- What does `agy --version` actually print, and does `agy update` exist? (Blocks final version-probe and lifecycle wiring.)
- What are the exact `step_update` field names in a real streaming run?
- What exit code and stderr does an unauthenticated non-interactive run produce? (Blocks the `needs-authentication` mapping.)
- What does `--sandbox` actually contain, and is there any catalog-legal mechanism — env var or per-invocation settings key — to express a permissive posture at launch without the bypass flag? (Would let D7 separate `trusted`/`yolo` from `standard`.)
- Where does Antigravity write interactive session transcripts? (Unblocks the deferred terminal-mode usage ingestion, which is out of scope here.)
