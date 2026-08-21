## Context

`complete-english-user-guide-content` closed the declared transition: 29 chapters, both locales, structurally equivalent. Its design recorded two things it deliberately did not do — decision D6 (the developer content in `multi-agent-workflow.md` stays put) and a stated non-goal (screenshots are a separate pass). This change is that pass.

The relevant boundary is drawn by two specs. `user-guide-documentation` scopes the guides to task-oriented user goals with truthful runtime and feature-state labeling. `native-developer-documentation` requires the developer guide to be the single English architectural narrative and places ADR-style content in `src-tauri/ARCHITECTURE.md` or the developer guide, never in a competing narrative.

The capture harness is `tests/docs/documentation-screenshots.spec.ts` driven by `scripts/run-doc-screenshots.mjs`, which allocates its own port through `docs-screenshot-port.mjs` rather than reusing a dev server. It already types `Locale = "en" | "zh-CN"` and resolves selectors through a `text(locale, zh, en)` helper.

## Goals / Non-Goals

**Goals:**

- Put each piece of content in the book whose spec governs it, in both locales at once.
- Give the English guide the same media set as the Chinese guide.
- Remove the stale `openspec validate` invocation the tutorial tells readers to run.

**Non-Goals:**

- Changing what either guide claims about the product. This moves and prunes; it does not revise behavior descriptions.
- Restructuring any other chapter. Only the two chapters D6 named are in scope, and the audit that found them did not find a third.
- Changing the capture harness beyond the one selector that blocks an English run.
- Adding screenshots to chapters that never had one. The English set mirrors the Chinese set; where the Chinese guide has no capture, neither does the English.

## Decisions

### D1: Split by "can the reader act on it", not by how technical it reads

**Choice:** A statement stays in the user guide when a user can act on it or needs it to interpret what they see; it moves to the developer guide when acting on it requires the repository.

That test keeps `MAX_CHAIN_DEPTH` at 15 in the user guide — it is the number behind the `handoff 1/15` counter on screen — while moving `src-tauri/src/contexts/agent_runtime/application/seat_turn.rs:29-30`, which is only actionable with a checkout. It keeps "an `@` inside a fenced code block does not trigger a handoff" and moves `strip_fenced_code` at `seat_turn.rs:120-133`.

**Why not split by section:** the two audiences are interleaved inside single paragraphs — the five handoff defenses are each one sentence of observable behavior followed by one code coordinate. Splitting by section would either drag the behavior into the developer guide or leave the coordinates behind.

### D2: The tutorial keeps its walkthrough and loses its harness

**Choice:** `multi-agent-testing-tutorial.md` remains a user-facing walkthrough — the case goal, the six steps, the six checkpoints, and the record template. What leaves is the `lint:ci` / `cargo fmt` / `cargo clippy` / `playwright` command blocks, the mapping from checkpoints to named Playwright specs, and the `openspec validate` line.

**Why not move the whole chapter:** verifying that several Agents really do appear in one session, that a departed member does not rewrite history, and that a single-Agent session is unaffected is a legitimate thing for a user to check. What is not is running the repository's CI commands. Moving the chapter wholesale would remove a usable tutorial from both guides to fix a problem confined to three code blocks.

**Consequence:** the stale `openspec validate complete-multi-agent-session-presence --strict` line is deleted rather than corrected, because the developer guide states the verification approach without naming a change that has since been archived — which is how it went stale in the first place.

### D3: The developer guide chapter grows to hold what it receives

**Choice:** `multi-agent-group-chat.md` goes from a 1.6 KB orientation note to the chapter that actually carries the design: seat storage and its degradation behavior, handle derivation, the five parsing defenses with their coordinates, chain limits, human-handoff intents, model-family resolution, context delivery, the mirrored frontend/native implementation, and the code-location index.

It keeps pointing at `openspec/specs/multi-agent-group-chat/spec.md` as authoritative for requirements. The chapter explains how the requirements are met and where; the spec remains what must be true.

### D4: Declare English scenarios by mirroring, and fix the one selector that blocks them

**Choice:** Every Chinese scenario in `screenshots.json` gains an `en` twin with the same `scenario`, `runtime`, and `featureState`, differing only in `locale`, `id`, and `path`.

The harness needed one change: the IM scenario selected the Feishu connector by the Chinese-only pattern `/飞书/`, which cannot match in an English run. It becomes `/飞书|Feishu/`, matching the bilingual pattern the file already uses in three other places.

**Why mirror rather than curate:** a curated English subset would reintroduce exactly the divergence the previous change closed, and the equivalence requirement covers media as much as prose.

### D5: Screenshots are captured, not hand-placed

The captures come from `npm run docs:screenshots:update` against the deterministic Web/mock fixtures, which is what makes `runtime-labels.md`'s claim honest — that every screenshot in the guide is a browser-preview capture and therefore shows what a surface looks like, not that a native side effect occurred.

This requires the worktree's dependencies to be installed; the capture harness is a Playwright suite, unlike the rest of the documentation toolchain, which runs on plain Node scripts and the pinned mdBook binary.

## Risks / Trade-offs

- **[Content moved between books could be lost in the move]** → the task list verifies each migrated claim arrives, and the two chapters are compared before and after rather than rewritten from memory.
- **[The user guide chapter gets shorter, which can read as lost detail]** → it loses no statement about product behavior; what it loses is where the code implementing that behavior lives.
- **[English captures could drift from Chinese captures as the UI changes]** → they are regenerated by the same command in the same run, and `npm run docs:screenshots:check` compares both sets.
- **[Installing dependencies in this worktree has a cost]** → roughly 630 packages, and on Windows a deep `node_modules` makes the worktree harder to remove later. Accepted: the captures cannot be produced without it.
- **[Deleting the stale `openspec validate` line loses a verification step]** → the developer guide states how the design is verified; naming a specific change is what rotted, and the same wording would rot again.
