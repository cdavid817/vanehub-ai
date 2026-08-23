# CLI lifecycle and global configuration

`tooling` is the largest bounded context; its Skill and MCP subdomains each have their own chapter ([Skill management](skill-management.md), [MCP tools and clients](mcp-tools.md)). This chapter covers the other half: **discovering the CLIs themselves, resolving conflicts, planning and executing a change, and writing provider configuration into each CLI's own configuration file**.

## The catalog is a compile-time constant

`CLI_TOOL_DEFINITIONS` is a `&[CliToolDefinition]` const slice, not a runtime-extensible registry. Each entry names the executables to look for, the distributions the CLI is available from, and the probes that can be run against it:

| Agent | Executables | Distributions |
| --- | --- | --- |
| Claude Code | `claude` | npm `@anthropic-ai/claude-code`, WinGet `Anthropic.ClaudeCode`, vendor installer |
| Codex CLI | `codex` | npm `@openai/codex` |
| Gemini CLI | `gemini` | npm `@google/gemini-cli` |
| OpenCode | `opencode` | npm `opencode-ai`, vendor installer |
| Antigravity CLI | `agy` | vendor installer only |

A distribution carries its own capabilities, so "can this be downgraded" is data on the definition rather than a conditional somewhere in the UI. npm supports every action at an exact version; WinGet supports install, upgrade, and uninstall, with downgrade and reinstall deliberately disabled until each is separately verified; a vendor installer installs and upgrades to latest and pins nothing.

## Two identities, not one active installation

Discovery walks `PATH` in real order and then a bounded set of known locations. It never recursively scans a disk. What it produces is a list of installations, and the snapshot names two of them:

- `path_selected_installation_id` — what a shell would reach, decided by `PATH` order alone.
- `recommended_installation_id` — what the backend would act on, decided by probe results.

They are separate fields because they are separate questions, and collapsing them is what made a broken launcher earlier in `PATH` invisible: the page reported the healthy copy while the terminal ran the broken one.

The same split governs launching. `CliApi::resolve_executable` reads this snapshot, follows the recommended installation, and returns an **absolute path or nothing** — a bare command name would re-enter `PATH` resolution inside the child process, and `PATH` is precisely what is in dispute.

## Conflicts are structured values

`derive_conflicts` produces zero or more `CliConflict`s, each carrying a kind, a severity, the installations involved, `blocks_mutation`, `blocks_launch`, and a stable `reason_code` the frontend localizes. There are nine kinds:

`duplicate-launcher-alias`, `path-shadowing`, `broken-path-precedence`, `multiple-installation-sources`, `version-divergence`, `ambiguous-source-ownership`, `environment-path-divergence`, `architecture-mismatch`, `stale-launcher-target`.

Two properties matter more than the list:

- **`blocks_mutation` and `blocks_launch` are decided in the backend.** A UI that re-derived them from the kind would disagree the first time a kind's severity changed.
- **Launcher families are folded first.** One npm global install on Windows writes `tool`, `tool.cmd`, and `tool.ps1` side by side; without grouping, one installation reports as three competing ones.

## Sources decide capability; capability is never inferred from a name

`CliSourceKind` names where a copy came from: `Npm`, `Winget`, `VendorInstaller`, `Homebrew`, `Bun`, `Volta`, `Desktop`, `System`, `Manual`, `Unknown`. `CliSourceManagement` says what VaneHub can do about it — `managed` for the first three, `detect-only` for the rest.

**Detect-only is a statement about VaneHub's capability, never about the installation's health.** A Homebrew-installed CLI that runs fine is healthy and detect-only at once. Each detect-only kind carries a `guidance_code` naming the tool that does own it, so the answer to "why is there no upgrade button" is "run `brew upgrade`" rather than "unsupported".

Version catalogs are per source. A WinGet installation's update state comes from WinGet's own catalog; borrowing npm's is the defect this model removes.

`CliSourceConfidence` separates `unknown`, `inferred`, and `verified`. A path heuristic is *inferred* — it is enough to offer an action, and not enough to claim ownership.

## The action plan is the contract

Nothing mutates without a plan. `prepare_cli_action` takes what the user chose — agent, source, target version, channel — and returns an operation id; the plan it produces carries the action the backend derived, the exact version transition, a **structured `argv` preview**, preconditions, warnings, elevation and network requirements, and an expiry.

`execute_cli_action` takes **only a plan id and the revision the user was shown**. There is no parameter on that call from which a command could be rebuilt, which is what makes "the version reviewed is the version that runs" structural rather than a convention. A plan is single use, valid for ten minutes, and bound to an environment fingerprint; expiry, reuse, a revision mismatch, and a moved environment are four distinct refusals with four stable categories.

The direction is derived in one place. `action: null` means "move this tool to the chosen version" and the backend decides whether that is an install, an upgrade, or a downgrade — there is exactly one version comparison in the product, `NormalizedCliVersion`, and a version that does not parse stays *opaque* rather than being guessed at.

**There is no fallback.** A vendor installer that fails does not silently become an npm install, and every plan says so on its face.

## After an external effect

A package manager cannot be undone by writing an older row, so the outcome vocabulary distinguishes five terminal states: `verified`, `applied-unverified`, `changed-but-failed`, `no-change-failed`, `cancelled`.

`changed-but-failed` is the one that justifies the rest. When a command fails but post-mutation detection observes a changed host, the honest report is that something happened and it was not what was asked for. **The pre-operation snapshot is never restored as a claimed rollback**, and when detection itself fails the last known values are kept and labelled stale with a warning attached.

## Global configuration: rewriting each CLI's own file

The `cli_config` subdomain is the only part of `tooling` that **actively rewrites an external program's configuration file**. All five Agents have their own managed file; the semantics are covered in [CLI Agent global configuration](../../cli-agent-global-configuration.md).

Four write constraints:

- **Only VaneHub-owned fields are replaced.** Hooks, permissions, and plugins in `settings.json`, and projects, MCP servers, comments, and unrelated providers in `config.toml`, are all preserved as-is.
- **The full result is built in memory first, then swapped in atomically.** When Codex touches multiple files, any single step failing rolls back every file already changed.
- **Managed fields are read back before switching configurations.** Leaving one configuration reads the managed fields out of the currently active file and writes them back into that configuration — otherwise your manual tweaks to the live file would be silently discarded.
- **Drift is only reported, never overwritten.** After applying, a fingerprint is kept for the managed fragment; if the file is changed externally, drift is reported, and the write is aborted if concurrent modification is detected mid-apply.

Credentials are stored in the operating system's credential service, scoped per Agent/profile, never in SQLite; plaintext is only written into a file the CLI requires plaintext for when you explicitly hit "apply." **VaneHub never captures a provider credential of its own**: sign-in belongs to the vendor's CLI, and the only thing read out of a sign-in probe is a normalized summary.

**Running CLI processes are never restarted automatically after a successful apply** — no hot reload is claimed.

## Compatibility with the pre-change model

The `cli_tool_status` table from the flat model is still created and still readable, so an upgrading install sees its tools before the first refresh. It is **read-only and never authoritative**: a leftover row becomes a *stale* snapshot only when no real one exists, nothing writes it, and an architecture test fails the build if a second reader or any writer appears.

## Relationship to other contexts

- How a detected CLI becomes a usable Agent is covered in [Agent lifecycle and provider runtime](agent-lifecycle.md).
- The interactive path once a CLI process starts is in [Terminal and PTY runtime](terminal-runtime.md); the non-interactive delegation path is in [CLI delegation and the ChangeSet pipeline](cli-delegation.md).
- The provider catalog is shared with OnePiece; see [OnePiece native Agent](onepiece-native-agent.md) and the [built-in model provider catalog](../../model-providers.md) (Simplified Chinese).
- The user-facing flow is covered in the user guide's chapter on installing and authenticating a CLI.

## Where the design lives

This chapter orients contributors; the authoritative requirements live in the corresponding main specs under `openspec/specs`.
