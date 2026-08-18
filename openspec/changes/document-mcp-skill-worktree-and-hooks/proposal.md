## Why

Five capabilities were documented at a depth that told a reader what controls exist without telling them what the thing is, what it is made of, or where its edges are. Measured against the specs that govern them:

| Capability | Spec material | Guide coverage before |
| --- | --- | --- |
| MCP | 25 requirements across `mcp-client-management` and `agent-mcp-tools`, ~500 spec lines | 27 lines, a section of `tooling.md` |
| Prompt Hook | 11 requirements, 181 spec lines | 11 lines, a section of `tooling.md` |
| Git worktree | 8 requirements, 132 spec lines | **No chapter and no named section** — incidental mentions in 8 files |
| Skill | Deep chapter already, 130 lines | No statement of what a Skill is or how it differs from custom instructions and MCP |
| Loop Engineering | 12 requirements | Mechanism explained; no definition, no composition, no boundary |

**The worktree case is a coverage violation under this repository's own requirement.** `user-guide-documentation` requires coverage to be "a dedicated chapter or a named section of the chapter that already owns the subject" and states that "an incidental mention SHALL NOT count as coverage". Git worktrees had only incidental mentions, which is precisely the state that requirement was added to forbid — and it went unnoticed because the requirement was added in the same session that the audit stopped at capability names rather than depth.

Documenting them surfaced facts a reader needed and could not get:

- MCP environment variables and headers are **stored and exported as plaintext**. Nothing said so.
- **Every MCP tool call requires explicit approval**, with no automatic path, and the target server's visibility is re-validated immediately before connecting.
- A Prompt Hook's template variables come from a **backend-owned allowlist**, unknown variables are rejected **at publication**, and substitutions are **inert text** that is never executed.
- A Loop is **two roles in two sessions** — a Worker and a read-only Verifier — not one Agent looping. The Verifier's writes are denied so that "review passed" stays independent.
- A Loop's worktree is **never cleaned up automatically** after success, failure, cancellation, rejection, or restart recovery.

## What Changes

- Three new chapters: `mcp.md`, `prompt-hooks.md`, and `worktree.md`, each opening with what the thing is, what it can and cannot do, and what it is composed of.
- `tooling.md`'s MCP and Prompt Hook sections become pointers, so the material lives in one place; the two screenshots they carried move into the new chapters rather than becoming orphans.
- `skill-management.md` gains an opening that states what a Skill is and distinguishes it from custom instructions (which describe *you*) and from MCP (**MCP supplies tools, a Skill supplies method**), plus the enabled-versus-assigned distinction as an explicit premise rather than an inference.
- `loop-engineering.md` gains a definition, its composition, and its boundaries — including the Worker/Verifier split, which the chapter had never mentioned.
- `multi-agent-workflow.md`'s three-row pattern comparison becomes the seven-topology classification, with VaneHub's position marked in each row.
- Both language editions change together.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

None. This deepens coverage of capabilities that are already delivered and already governed; no requirement text changes, so it carries no spec delta. It brings the guide into compliance with the coverage requirement rather than altering it.

## Impact

**Runtime scope: neither.** Documentation only.

Affected surfaces:

- `docs/user-guide/{en,zh-CN}/src/` — 3 new chapters, 3 chapters expanded, `tooling.md` reduced, `SUMMARY.md` and `index.md` updated. The guides go from 29 to 32 chapters.

**Five label mismatches were corrected along the way**, all of the same class found in earlier changes: a documented name that the interface does not render. In `loop-engineering.md`, phases `行动` and `判定` are actually **执行** and **决策**, and the control labelled `取消` is **停止**. In `tooling.md`, the MCP state set included `已断开`, which does not exist.
