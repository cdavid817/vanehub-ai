# Personalization

**Status: Implemented — desktop only.**

## Overview

Store "facts about you", "style preferences", and "knowledge accumulated in a project" in one place, and have them injected automatically before an Agent runs, instead of explaining it all again in every new session.

Three layers cover different ground: **Custom Instructions** (written by hand), **Agent memory** (accumulated automatically), and **expert roles** (switchable personas).

## Custom Instructions

Fill in two fields under **Settings → AI Personalization → Instructions → Custom Instructions**:

| Field | What to write | Limit |
| --- | --- | --- |
| **About you** | Identity, background, long-term preferences | 3000 characters |
| **Response style** | Output format, language, level of detail | 3000 characters |

For example, put "Always answer in Chinese. Lead with the conclusion." under **Response style**, and "I'm a backend engineer working mainly in Rust and TypeScript." under **About you**.

Setting **How this layer combines** to **Apply no instructions here** turns injection off — **new sessions stop applying it, and what you saved is not lost**.

> Edits are written only when you press **Save**; leaving a field does not save it, and **Discard** puts the text back the way it is stored.

![The Instructions view of the AI Personalization settings page, showing scope selection and the instruction editor](assets/screenshots/personalization-en.png)

## Agent memory

Project conventions an Agent discovers during a session are recorded automatically and are available the next time you open a session. The memory section on the same page lets you review and manage them.

Two switches:

| Switch | What it does |
| --- | --- |
| **Enable memory** | The master switch. **Turning it off also stops using memories already saved**, not just adding new ones |
| **Remember from tool-assisted chats** | Controls whether OnePiece extracts automatically in sessions that used shell, file, or MCP tools. Explicitly asking it to remember something is never affected |

> **The second switch does not affect CLI Agents.** The interface says so explicitly: Claude Code, Codex CLI, Gemini CLI, OpenCode, and Antigravity CLI extract memories independently of this toggle.

Saved memories are reviewed and managed in the **Saved memories** section on the same page.

### Two prerequisites you have to know

**1. Memory is shared host-wide.** What one Agent records is available to the others. There is currently **no way to isolate it per Agent** — if you need isolation, the only option is to turn memory off entirely.

**2. Memory extraction for CLI Agents is performed by OnePiece.**

This constraint is invisible in the interface but will affect you in practice:

> **With no provider configured for OnePiece, Claude Code, Codex CLI, Gemini CLI, OpenCode, and Antigravity CLI produce no memory extraction at all.**

The reason is that those CLIs expose no reusable model credentials, so the extraction has to go through OnePiece's provider.

**So: even if you mainly use Claude Code, you must configure OnePiece before memory works.** See [Native API Agent](native-agent.md) for how.

### When memories are written

Memory extraction rides on **context compaction** — when a conversation grows long enough to trigger compaction, extraction happens alongside it. It is best-effort and does not guarantee that every valuable fact is recorded.

Writing a memory is itself governed by the permission system (it is the "write memories" action, which every template allows).

## Expert roles

Custom Instructions and memory are **cross-session and about you personally**; an expert role instead describes **a job**, bound to an Agent by a seat within a single session. The two can be combined.

Managed under **Settings → Expert roles**, with three built-in roles: architect, implementer, and code review. Field meanings, review policy, and how to use them are covered in [Expert roles](expert-roles.md).

## Notes and limits

- **Desktop only**; both memory and settings depend on local storage.
- **Injection does not rewrite any CLI's own configuration files** — it does not touch `CLAUDE.md`, `AGENTS.md`, or the like.
- **OnePiece's own core instructions cannot be edited**; express customization through Custom Instructions or an expert role instead.
- If reading settings fails, behavior falls back to "inject no custom instructions, keep memory enabled" — a transient error does not silently disable something that was working.
