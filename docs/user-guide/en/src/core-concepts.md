# Core concepts

Understanding these terms makes the later chapters much easier to read.

## Session

**A session is VaneHub AI's core unit of work.** One session binds one workspace directory and one or more Agents, and carries the conversation, the terminal, file changes, and execution tracing.

A session has five lifecycle states:

| State | Meaning |
| --- | --- |
| `idle` | Idle; you can submit a task |
| `starting` | Starting up |
| `running` | The Agent is producing output |
| `failed` | Execution errored |
| `stopped` | Stopped |

While `starting` or `running`, the interface disables repeat submission.

Sessions can be **categorized**, **pinned**, and **archived**. An archived session can still be viewed and exported, but **cannot be activated, messaged, or used to start a generation**.

## Agent

**An Agent is what actually performs the task.** VaneHub AI ships with six:

| Agent | Provider | Form |
| --- | --- | --- |
| Claude Code | Anthropic | External CLI |
| OpenCode | OpenCode | External CLI |
| Codex CLI | OpenAI | External CLI |
| Gemini CLI | Google | External CLI |
| Antigravity CLI | Google | External CLI |
| OnePiece | VaneHub | **Native API Agent** |

The first five are **external CLIs** — VaneHub AI starts their processes and manages everything around the process (launch flags, permission interception, output capture), while the actual code generation is done by the CLI itself.

**OnePiece** is different: it calls a model provider over HTTP directly and runs entirely inside the application. See [Native API Agent](native-agent.md).

## Seat and Expert Role

**A seat is the pairing of one Agent with one expert role.** A session can hold several seats; they share one conversation thread and hand the turn over with `@`.

An **expert role** defines that seat's responsibility, its instructions, which Skills it can bind, and whether it can act as a reviewer. The role name derives the **handle** used to `@` it.

See [Multi-Agent group chat and `@` handoff](multi-agent-workflow.md).

## Workspace

**The workspace determines which files an Agent can see and where its commands run.** There are three forms:

| Form | Notes |
| --- | --- |
| Local project directory | The most common |
| Git worktree | A separate working copy for a different branch of the same repository |
| Remote path | A directory on a remote host reached over SSH |

If the project you pick when creating a session is a **Git project**, you can tick **Create new Git worktree**; a plain folder disables the option.

> **A remote workspace does not support worktrees** — it can only point at a path that already exists on the remote host.

## Policy template

**The permission system is the single gate in front of an Agent's dangerous operations.** Four templates decide what an Agent may do on its own and what it must ask you about:

| Template | Run commands / write files |
| --- | --- |
| Read-only | Denied |
| Standard | Ask every time |
| Trusted | Allowed |
| Yolo | Allowed |

**Reading files and writing memories are allowed under every template** — even Read-only permits reading, which is exactly what the name means.

See [Permission approvals](permissions.md).

## Loop

**A Loop is a goal-driven automatic cycle.** You supply a goal and a set of must-pass checks, and it repeats "act → verify → judge" until the goal is met or a limit is hit — and **it always requires your manual acceptance**.

See [Loop Engineering](loop-engineering.md).

## MCP server

**An MCP (Model Context Protocol) server gives Agents additional tools.** Register one centrally in VaneHub AI and it can be handed to each Agent, instead of being configured separately inside every CLI.

See [Tools and extensions](tooling.md).

## Skill

**A Skill is a reusable capability package.** Skills have either **global** or **workspace** scope and can be bound to specific Agents. A Skill is an entity on the filesystem and may be modified outside the interface, so the system performs **drift detection**.

See [Manage Skills](skill-management.md).

## Execution tracing and spans

**Everything from submitting a task to finishing it is recorded as a span tree** with four layers: session → Agent → tool/MCP boundary → process execution.

Each node carries a **fidelity** annotation stating how much the record can be trusted:

| Fidelity | Meaning |
| --- | --- |
| Native | First-hand record from the runtime |
| Relayed | Observed through the relay layer |
| Inferred | Derived from other signals |
| **Opaque** | **Only the boundary is known; the inside is not visible** |

What happens inside an external CLI is a black box, so only the boundary nodes are kept and **no child nodes are invented**. See [Observability and logs](observability.md).

## Web/mock preview

**The same interface also runs in a browser**, backed by deterministic mock data. It is good for looking at the interface, but it **starts no process, writes no database, and touches no filesystem**.

Any operation labeled **Web/mock only** is not evidence that anything really happened on your machine. See [Runtime and feature labels](runtime-labels.md).

## Looking for implementation detail?

This guide only covers how to use the product. For **why** these mechanisms are designed the way they are and how they work internally, see the [VaneHub AI Developer Guide](../../../developer-guide/src/index.md) — it is written for developers and contributors and points at the code.
