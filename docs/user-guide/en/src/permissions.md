# Permission approvals

## Overview

When an Agent wants to run a command, write a file, call a tool, or write a memory, the operation passes through a gate first. When the verdict is "ask", execution pauses and an approval appears; your decision can be remembered by scope so you are not interrupted repeatedly.

What it solves is that each CLI has its own confirmation mechanism, and they are hard to bring under one roof.

## The four policy templates

Choose a template under **Settings → Agent Policies**. **A template affects only two kinds of action:**

| Action | Read-only | Standard | Trusted | Yolo |
| --- | --- | --- | --- | --- |
| Read files | Allow | Allow | Allow | Allow |
| Write memories | Allow | Allow | Allow | Allow |
| **Run commands** | Deny | **Ask** | Allow | Allow |
| **Write files** | Deny | **Ask** | Allow | Allow |

**Each Agent gets its own row**, so different Agents can run under different templates. The default is **Standard**.

Two things are easy to misread:

- **"Read-only" does not forbid everything** — reading files and writing memories are still allowed.
- **"Trusted" and "Yolo" give the same answers inside VaneHub AI** (both allow commands and file writes); they differ in how firmly you have to confirm when granting them. What each CLI is *launched with* is a separate projection of the template and can differ per CLI — see [What each CLI is launched with](#what-each-cli-is-launched-with) below.

![The Agent Policies settings page, one row per Agent with four selectable templates](assets/screenshots/permissions-en.png)

## Raising privilege needs confirmation

Switching to **Trusted** or **Yolo** requires one explicit confirmation; switching back to **Standard** or **Read-only** does not.

The test is whether that template automatically allows running commands and writing files — not the name itself.

## Handling an approval

**An approval is not a modal dialog.** When "ask" is hit, the corresponding **tool call block** in the conversation stops at `awaiting_approval` and a notification appears at the bottom right telling you approval is needed.

**Tool call blocks are collapsed by default** — you have to expand one to see the approval area, which reads "This tool call needs your approval before it runs" and shows the risk level (for example **High risk**).

Expanded, it lists three pieces of information:

| Field | Meaning |
| --- | --- |
| **Agent** | Which Agent initiated it |
| **Action** | For example `shell.exec` |
| **Resource** | What it acts on |

Then pick a scope under **Remember my choice:** and select **Approve** or **Deny**:

| Scope | Effect |
| --- | --- |
| **Just once** | Not remembered; you are asked again next time |
| **This session** | No longer asked within the current session |
| **This project** | No longer asked within the current project |
| **Always** | Never asked again |

With any scope other than "Just once", equivalent actions are allowed outright within that scope.

## Decision priority

The system resolves in a fixed order and returns on the first match:

1. **The MCP tool floor** — unconditional, highest priority, unaffected by templates
2. **An authorization you remembered earlier**
3. **The rules of that Agent's template**
4. Nothing matched → **ask**

**An action that matches no rule falls through to "ask", not to allow.** Any internal error also falls back to "ask" — the system does not allow something because it failed.

## What each CLI is launched with

The template you pick is always enforced by VaneHub AI's own policy layer (approval cards, remembered grants, audit records). In addition, VaneHub AI passes the template to each CLI at launch so the CLI's own approval mode matches. How much of that is *guaranteed by VaneHub AI* depends on the transport; the full per-agent table is in the [agent capability matrix](../../../reference/agents/capability-matrix.md).

| Transport | Agents | Read-only | Standard | Trusted / Yolo | What VaneHub AI guarantees |
| --- | --- | --- | --- | --- | --- |
| Headless CLIs | Claude Code, Codex CLI, Gemini CLI, OpenCode, Antigravity CLI | The CLI's plan / read-only mode | The CLI's own ask-first mode (OpenCode via an environment variable) | The CLI's accept-edits / auto mode | Flags are chosen by VaneHub AI from an audited catalog; no bypass flag is ever used |
| ACP agents (managed conversation) | Qwen Code, Kimi Code CLI, Qoder CLI, CodeBuddy Code, GitHub Copilot CLI, Cursor Agent CLI | Plan flag where the CLI has one | No launch flag: every tool call is asked of VaneHub AI per call | No launch flag: VaneHub AI answers per call from your template | Per-call decisions by VaneHub AI; an unattended "ask" is rejected, never approved |
| ACP agents in the Agent Terminal | same six | Plan flag; **Qoder CLI has no read-only mode, so a read-only terminal launch is refused** | Qwen, CodeBuddy: default flag; Kimi, Copilot, Cursor: the CLI's own ask-first default (**provider-delegated**, VaneHub AI passes and verifies nothing) | The CLI's auto-approve flag | Only the launch flag; the CLI's internal approvals are its own |
| Legacy terminal | iFlow CLI | `--plan` | `--default` | `--autoEdit` / `--yolo` | Only the launch flag; no managed conversation |

**Claude Code is special** in one more way: the launch flag only sets its permission mode. The real gate is a **permission hook** that asks VaneHub AI on every intercepted tool call, which is more precise than flags fixed at launch. When the hook is unavailable it falls back to an offline decision based on risk classification, rather than failing the whole chain.

**Antigravity CLI does have native flags** (`--mode` and `--sandbox`); VaneHub AI projects the template into them like the other headless CLIs, and never uses its `--dangerously-skip-permissions` bypass.

"Provider-delegated" or "unverified" in the UI means VaneHub AI passed no restriction and checked none — the CLI is following its own defaults. "Not managed by VaneHub AI" never means the CLI itself lacks the feature.

## Notes and limits

- **Desktop only.**
- **Templates are action-level and do not distinguish paths or command content.** There is no way to configure a rule like "may only write files under `src/`".
- **Each CLI's own confirmation mechanism still exists.** VaneHub AI's gate is an additional layer; it does not replace a CLI's own sandbox or confirmation logic.
- **Desktop is authoritative for approval state.** On reload the interface actively fetches the pending-approval list and reconciles, rather than depending on events not being lost.
- Every resolved decision is written to an audit record, including those allowed or denied outright.
