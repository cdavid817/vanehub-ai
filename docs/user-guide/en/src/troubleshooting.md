# Troubleshooting

Find your symptom below. Most problems are not bugs — they are an unmet precondition.

## First, confirm which runtime you are in

**This is the easiest thing to misjudge.** If you opened VaneHub AI in a browser, then: CLIs are not detected, processes are not started, settings are not saved, and files are not changed — **even when the interface reports success**.

How to tell, and the full capability table, are in [Runtime and feature labels](runtime-labels.md).

Unless noted otherwise, everything below assumes desktop.

## CLI issues

### A CLI shows "Not Installed" but I definitely installed it

VaneHub AI resolves the `PATH` **the desktop application can see**, which is not necessarily the one in your terminal — especially when launched from an icon rather than from a terminal.

1. Run the command in an ordinary terminal (`claude` / `codex` / `gemini` / `opencode`) and confirm it works
2. If it works, this is a `PATH` problem: fix the `PATH` visible to the desktop application, then **restart VaneHub AI**
3. If it does not work, this is an installation problem: see [Install and authenticate a CLI](getting-started.md)

### It shows "Installed but not runnable"

**Do not reinstall.** The message in the interface is the answer:

> The active CLI is installed but cannot run. Check Node, PATH, or the tool environment; reinstalling the same version usually will not repair it by itself.

The file is there; the environment cannot run it — usually the Node version or the CLI's own dependencies.

### It shows "Installation conflict"

The same CLI is installed more than once. Select **Diagnose Conflicts** to see **Installation diagnostics**, which lists every local installation path and marks the **Active** one.

**Upgrade only the active copy.** Upgrade the wrong one and the command line still resolves the old version, which looks as though the upgrade did nothing.

If the active copy was not installed through npm, update it through its original source — installing another copy through npm only makes the conflict worse.

### I upgraded but the version did not change

See above. Most likely the upgrade applied to a copy that is not the active one.

### The Agent asks me to sign in

Complete authentication in that CLI. **VaneHub AI does not store provider credentials** and cannot walk a vendor's sign-in flow. Once authenticated, come back and refresh detection.

OnePiece is the exception — its API key is stored by VaneHub AI; see [Native API Agent](native-agent.md).

## Session issues

### Creating a session reports "Agent unavailable"

The CLI is not installed properly or not authenticated. Go back to the CLI section above.

### Creating a session reports "Git worktree failed"

Worktrees are only available for Git projects. When picking a directory, the interface marks it **Git** or **Folder** — a "Folder" disables the worktree option.

If it is marked Git and still fails, check whether a worktree or branch of the same name (`vanehub/<name>`) already exists.

### The session state is wrong after a crash

When the application restarts after an abnormal exit — a crash, a forced termination, a power loss — it reconciles interrupted sessions once.

**The verdict comes only from durable business evidence** — sessions, messages, operations, and tool records. It does **not** infer an outcome from timestamps, display order, diagnostic logs, or observability records. So even if the unified logs are missing, expired, or something you turned off, recovery does not invent evidence to compensate.

Reconciliation has three outcomes:

| Recovery status | When | Result |
| --- | --- | --- |
| **Clean** | It can be conclusively correlated with one completed or failed response | The session returns to idle or failed |
| **Action required** | There is tool activity whose effect cannot be determined, or the evidence conflicts | **It waits for you** |
| **Quarantined** | The evidence is structurally invalid | Held for human intervention |

When there is only an unfinished response and no tool activity at all, **partial content that was already persisted is preserved**, the response is marked interrupted or failed, and the session returns to a terminal state.

### A session is stuck at "action required"

This state means **an external side effect cannot be determined to have happened or not**. For example a file-writing tool call was interrupted halfway, and the durable records do not show whether the write succeeded.

**The system will not guess for you.** You can acknowledge the current recovery revision, which:

- Records your acknowledgement and clears the recovery gate
- **Preserves all evidence and the lifecycle state it was interrupted in**
- **Does not retry the original generation**
- **Does not imply that the uncertain side effect did or did not occur**

If the session changed while you were deciding, a submitted revision that no longer matches is rejected and the current state is returned for review — which prevents you judging on stale information.

**Interrupted work is never replayed automatically.** The provider's resume metadata is preserved, but the system does not resume or resend that generation. Continuing is up to you.

Recovery itself is idempotent: repeated launches and concurrent state changes produce no duplicate reports and never regress a state that has already terminated.

> A recovery report records only correlations, the decision, safe reason codes, and bounded evidence references. **It contains no prompts, message bodies, tool payloads, commands, credentials, or raw provider errors.**

### A tab says "the workspace request is invalid or outside what this session allows"

The requested path went outside the session workspace boundary. VaneHub AI hard-blocks paths outside the workspace, and this is not something configuration can relax.

Two related messages: **"the requested session resource was not found"** (the resource was deleted or moved), and **"the current runtime does not support this workspace operation"** (the operation needs desktop).

### A tab only shows some of the data

> The first release uses bounded loading, and only part of the results may be shown.

This is a design constraint, not lost data. Terminal output has its own separate capacity and retention policy.

### A chunk is missing from the middle of the terminal output

Output capture uses a bounded queue, and when it fills it inserts a gap marker rather than blocking the process. Records past the retention period or capacity limit are cleaned up. For long-term retention, rely on the unified logs — see [Observability and logs](observability.md).

## Multi-Agent issues

### One seat never gets the turn

**Check the handle first.** Handles derive from the expert role name: whitespace becomes `-`, and a colliding name gets a suffix automatically.

**An `@` inside a fenced code block is deliberately ignored** — pasting code containing an `@` does not trigger a handoff by mistake.

If it stopped because the mention limit or the handoff chain depth limit was reached, the session states the specific reason.

### I mentioned an Agent but the flow did not stop

**Only a handoff interrupts the flow**; a plain mention is lightweight. This is deliberate: if every mention meant stopping to wait, the Agents would learn not to mention anyone, and you would lose that visibility. See [Multi-Agent group chat and `@` handoff](multi-agent-workflow.md).

## Memory and personalization

### The Agent has no memory

**Most likely OnePiece's provider is not configured.**

Memory extraction for CLI Agents is performed by OnePiece — with it unconfigured, no memories are produced at all. **Even if you mainly use Claude Code, you have to configure OnePiece first.** See [Personalization](personalization.md).

### I want one Agent to have its own memory

Not possible. Memory is currently a host-level shared pool, and what one Agent records is available to the others. Isolation is only achievable by turning memory off entirely.

## Remote and automation

### Loops do not work in a remote workspace

**A Loop needs its own Git worktree, and remote hosts do not support worktrees.** This is a capability boundary, not a configuration problem.

### A remote workspace will not open in a local editor

> Opening a remote workspace with a local application is not supported yet.

### An IM connector will not start

**The default route configuration must be saved first.** The interface's message is that this must be configured before any connector can be enabled. See [Remote execution and IM connectors](remote-and-im.md).

### A scheduled task did not run while the application was closed

The scheduler runs inside the application, not as a system-level service. **It does not run while closed, but missed runs are made up at the next launch — and only the most recent one.**

A once-daily task with the application closed for three days makes up one run on restart. See [Scheduled tasks and usage statistics](automation.md).

## Observability

### I cannot find logs by trace id

**Logs deliberately contain no execution identifiers**, which is a privacy design. Line the two up by **time**.

### Some nodes in a trace do not expand

That is **Opaque** fidelity — the internal behavior of an external CLI is a black box. The system keeps only the boundary node and does not invent children. For a fully expandable call chain, use OnePiece.

## Development

### Local screenshots differ from the ones in the repository

Documentation screenshots are authoritative from a fixed CI browser environment. Run this **only when deliberately reviewing a UI change**:

```bash
npm run docs:screenshots:update
```

### Startup reports `no such table`

When several worktrees share one database, migration version numbers can collide across branches. See the development environment chapter of the [Developer Guide](../../developer/index.html).

## Still stuck

The [FAQ](faq.md) covers another set of frequent questions, angled at "is this how the feature is meant to work" rather than "something went wrong".
