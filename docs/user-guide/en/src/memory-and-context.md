# Memory and context: what carries between sessions, and what happens when context fills up

## Overview

Two related but distinct things:

- **Memory** solves "I have to explain this again in every new session" — facts worth carrying between sessions are stored and brought along automatically.
- **Context compaction** solves "this conversation grew past the model's limit" — earlier turns are condensed so the conversation can continue.

## Memory

### One shared pool, not isolated per Agent

**OnePiece and every CLI Agent share the same memories**: `claude-code`, `codex-cli`, `gemini-cli`, `opencode`, and `antigravity-cli` all read the same pool.

A fact recorded during a Claude Code session is available in a Codex CLI session next time.

The system records **which agent produced each memory and in which workspace**, but **this is provenance only and is not used as a read filter**. In other words, the producing agent and directory do not restrict who can read it. A memory saved with no workspace folder in scope is still saved rather than rejected.

### Memories are files, not database rows

Each memory is **one Markdown file** in a host-level memory directory. Its frontmatter carries a `name`, a one-line `description` used to judge relevance without reading the body, and a `type`; the body is the memory content.

**The point of this design is addressability**: the model can correct or retract **one** memory instead of only ever appending. Each memory is addressed by its directory-relative path, and no two share a path.

**If one file's frontmatter is missing or unparseable, the system skips it and continues enumerating**, rather than failing the generation, the injection, or the management view.

### Four types

| Type | Purpose |
| --- | --- |
| **User** | Who you are, and your preferences |
| **Feedback** | Corrections you have given about how to work |
| **Project** | Ongoing work and constraints |
| **Reference** | Pointers to external resources |

A missing or unrecognized type **degrades to untyped** and remains enumerable, injectable, and manageable — so migrated files and hand-written files keep working.

### Two toggles

Under **Settings → AI Personalization → Memory**:

**Enable memory** — turning it off **also stops already-saved memories from being used**, not just from being saved. This is easy to misread: it is not a read-only mode, it is off.

**Remember from tool-assisted chats** — this governs exactly one thing: whether OnePiece extracts memories automatically **in sessions that used shell, file, or MCP tools**.

Two things it does not affect:

- **Explicitly asking it to remember something is never affected**, whatever the toggle
- **CLI Agents extract memories independently of this toggle**; it only governs OnePiece

### Manage saved memories

The list shows each memory's type, source (Extracted automatically / Remembered on request), and scope (a project, or All projects).

| Action | Effect |
| --- | --- |
| **Delete** | Deletes one memory. **Cannot be undone**, and revokes its retrieval index |
| **Reset all** | Deletes every memory. **Shared across OnePiece and all CLI agents**; cannot be undone |

The Reset all confirmation says explicitly that this is shared — because what it clears is not just the current agent's memories.

### Injection carries age and a staleness caveat

When a memory is injected into the system prompt, **what accompanies it is a human-readable elapsed time, not a raw timestamp**. The reason is practical: a bare timestamp is not enough to make the model recognize that content may be out of date.

A memory older than the staleness threshold **carries an additional caveat**: memories are point-in-time observations, claims about code or file locations may be outdated, and they should be verified against current state before being asserted as fact.

**A memory within the threshold does not carry that caveat** — a caveat on fresh content is just noise.

### Memory recall has its own context budget

Memory recall is **ranked and budgeted separately from code evidence**, each consuming its own versioned source allocation without crowding the other out.

**Memory bodies do not appear in selection diagnostics or in persisted manifest metadata.**

## Automatic context compaction

### The switch and what it applies to

Turn on **Automatic context compaction** in OnePiece settings: it lets OnePiece compact older context when the active model approaches its context limit.

**Changes apply to subsequent generations only; an active generation keeps the preference it started with.** So flipping the switch during a long generation does not affect that one.

### When it triggers

The priority is explicit:

1. **When verified model capacity and a token measurement are available, the token-aware decision is authoritative**
2. **When that evidence is unavailable or analysis fails, it falls back to the fixed character-count threshold**

The key is that the system **never invents capacity or token values**. There are two fallback situations: the active model has no verified context-window metadata, or this request's snapshot cannot produce a token measurement.

There is one counterintuitive consequence: **when token evidence reports occupancy below the threshold, no compaction happens even if the character threshold has been crossed.** When the token measure is available it is the authority, and the character count is only its backstop.

### What you see when it happens

**A visible notice is inserted into the conversation**, stating clearly that earlier turns were condensed. It uses the same persistence and rendering path as other rich blocks, so it does not disappear on the next refresh.

### When compaction is skipped

Even when the trigger conditions are met, these mechanisms suppress automatic compaction:

| Mechanism | Effect |
| --- | --- |
| **Request-level suppression** | Skip for this request |
| **Generation-scoped cooldown** | Recently compacted, so not again for a while |
| **Failure circuit breaker** | Stops trying automatically after repeated failures |
| **User preference** | You turned the switch off |

These suppressions leave **content-free evidence records**, visible under context health, containing no prompts or tool content.

## Context policy health

This page shows **content-free compaction outcomes and measurement coverage**:

| Metric | Meaning |
| --- | --- |
| **Evaluated decisions** | How many context decisions were assessed |
| **Outcomes** | The distribution of results |
| **Paths** | Which decision paths were taken |
| **Policy versions** | The policy versions in effect |
| **Measurement quality** | How much the measurement itself can be trusted |
| **Characters saved** / **Measured tokens saved** | How much compaction saved |
| **Token coverage** | What proportion could be measured in tokens |

**Three things it explicitly is not**, as stated in the interface itself:

- **Not billing records**
- **Not provider cache metrics**
- **Not proof of semantic answer quality**

What it measures is operational estimates — not your bill, and not whether the answers were good.

The retention policy is equally explicit: **only approved counters, enums, versions, timestamps, and correlations are retained; prompts and tool content are excluded.**

## Notes and limits

- **Memory is shared across every Agent.** Deleting and resetting affect all of them; there is no "clear only OnePiece's memories".
- **Turning off Enable memory also stops existing memories being used**, not just new ones being added.
- **"Remember from tool-assisted chats" does not govern CLI Agents**, nor anything you explicitly asked to be remembered.
- **Deleting a memory cannot be undone**, and revokes its retrieval index along with it.
- **Changing the compaction switch does not affect a running generation.**
- **The token measure outranks the character threshold** when it is available; the character count is only a fallback.
- **Context health figures are not billing data**; for usage and billing see [Usage statistics](usage-statistics.md).

## Related

- The settings page the memory toggles live on → [Personalization](personalization.md)
- Toggling long context with `/longcontext` → [Slash commands](slash-commands.md)
- How "memory extraction" and "context compaction" are counted in usage → [Usage statistics](usage-statistics.md)
- OnePiece's own execution modes and switches → [Native API Agent](native-agent.md)
- The retrieval technology itself: indexing pipelines, semantic-versus-keyword retrieval trade-offs, hybrid retrieval and reranking → [RAG technical architecture](../../../agent-infrastructure/patterns/rag.md) (Simplified Chinese)
