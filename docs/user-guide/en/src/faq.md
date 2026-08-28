# FAQ

## Does VaneHub AI manage my API keys?

**Not for the external CLIs.** Provider authentication for Claude Code, Codex CLI, Gemini CLI, OpenCode, and Antigravity CLI is managed by each CLI, with credentials stored in their own locations, and VaneHub AI never asks you for a provider password. Antigravity CLI goes further — it signs in through Google and stores its credential in the system keychain, so its configuration profile has no key field at all.

**Yes for OnePiece.** The native API Agent's API key is stored by VaneHub AI — see [Native API Agent](native-agent.md).

## Why does my Agent have no memory?

Most likely because **no provider is configured for OnePiece**.

Memory extraction for CLI Agents is performed by OnePiece, and with it unconfigured no memories are produced at all. Even if you mainly use Claude Code, you have to configure OnePiece first. See [Personalization](personalization.md#when-memories-are-extracted).

## Can one Agent have its own separate memory?

**Yes, per memory.** Each memory has an audience: every Agent by default, or only the ones you name. Set it when you approve a proposal or from the memory's detail view. See [Personalization](personalization.md#scope-source-and-audience-of-a-single-memory).

## Can recall search my project code?

**No.** It retrieves the **memories** you have accumulated; it does not index repository files.

## Does the Read-only template forbid everything?

No. **Reading files and writing memories are allowed under every template**, Read-only included. What it denies is **writing files** and **running commands**.

## What is the difference between Trusted and Yolo?

**The policy rules are identical in practice.** They differ only in how firmly you have to confirm when granting them: both require explicit confirmation, but Yolo's prompt is stronger.

## Is multi-Agent mode not finished yet?

**It works.** Choose **Multi Agent** in the create-session dialog to assign seats. It was disabled in an earlier version, and that part of the documentation is out of date.

Worth knowing: the early "dependency graph / DAG orchestration" design **has been removed**, and the current shape is [group chat seats with `@` handoff](multi-agent-workflow.md).

## Are there session checkpoints or cross-CLI session migration?

**No.** Migrating one session's context to another CLI is not supported, and there is no session checkpoint feature. A session is bound to the Agent chosen when it was created.

## Why do some nodes in a trace not expand?

Because they have **Opaque** fidelity — that is internal behavior of an external CLI, which is a black box. The system keeps only the boundary node and **does not invent child nodes**.

For a fully expandable call chain, use OnePiece. See [Observability](observability.md#fidelity-why-some-nodes-do-not-expand).

## Can I search the logs by a trace id?

**No.** Logs deliberately contain no execution identifiers, which is a privacy design. The two are lined up by **time**.

## Can a Loop run in a remote workspace?

**No.** A Loop works in a separate Git worktree, and **remote hosts do not support worktrees**.

## Do scheduled tasks run after the application is closed?

**They do not run while it is closed, but they are not all discarded.** The scheduler runs inside the application, not as a system-level service.

The interface explains: a run missed because the application was closed is **made up at the next launch, and only the most recent one is made up**. So a once-daily task with the application closed for three days makes up one run on restart.

## Are the usage statistics accurate?

**The data comes from each CLI's own reporting**; VaneHub AI does not meter tokens independently. Read the explanation on the page before using it for cost accounting.

## Does configuring MCP once cover every Agent?

**Only Claude Code and Codex CLI go through the relay today.** Gemini CLI, OpenCode, and Antigravity CLI need their own configuration, and their MCP calls do not appear in the execution trace.

## Is opening it in a browser the same as the desktop application?

The interface is the same; **the capabilities are not**. The browser preview (Web/mock) uses deterministic mock data and **starts no process, writes no database, and touches no filesystem**.

See [Runtime and feature labels](runtime-labels.md) for how to tell.

## Which languages does the interface support?

Five: Simplified Chinese, English, Traditional Chinese, Japanese, and Korean. Switch under **Settings → Basic Configuration**.

## Where do I look to understand the internals?

This guide only covers how to use the product. For **why** these mechanisms are designed the way they are and how they work internally, see the [VaneHub AI Developer Guide](../../../developer-guide/src/index.md) — written for developers and contributors, with architecture notes and code references.

One layer deeper still is **the protocols and technologies themselves**: MCP, LSP, Function Calling, Agent Skills, RAG, Tree-sitter, multi-Agent orchestration, A2A, and more — see [Agent infrastructure technical documentation](../../../agent-infrastructure/README.md) (Simplified Chinese).

## What if my question isn't here?

Questions about feature behavior belong here; something broken belongs in [Troubleshooting](troubleshooting.md). If neither has an answer, [file an issue](reporting-issues.md) — that chapter covers what to fill in, where to find logs, and how to redact before submitting.
