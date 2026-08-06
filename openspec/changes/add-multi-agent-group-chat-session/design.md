## Context

VaneHub drives coding agents as CLI processes. Each holds its own isolated context and cannot see another's conversation, so "a shared thread" has to be constructed rather than assumed. `clowder-ai` solves the same problem against the same class of agents, and reading its implementation settled several questions that would otherwise have been guesses.

Today a session binds one `agentId: string`, `ChatMessage` carries a `role` but no speaker, `AgentRegistryEntry.provider` holds free-form display text such as `"OpenAI"`, and the create-session dialog already renders a disabled `multi` mode. The nine workspace tabs all assume a single Agent.

## Goals / Non-Goals

**Goals:**
- Make multi-Agent collaboration emerge conversationally, with nothing planned before the first message.
- Keep role identity durable across context compaction.
- Keep the human a participant and an approval authority, never a dispatcher.
- Bound autonomous handoff so it cannot loop or fan out without limit.

**Non-Goals:**
- Turn liveness probing and zombie reclamation. `clowder-ai` needed an eight-state custody machine with heartbeats, but it earned that complexity from observed failures. Shipping it before the failures exist would be speculative.
- Parallel fan-out. `clowder-ai` explicitly bounds its parallel route so it never chains, because parallelism plus chaining breaks both depth accounting and the continuity of prior replies.
- Companionship, personas-as-product, or role-scoped memory.

## Decisions

### Roles are settings assets; seats are the session-time binding

A role is reusable and describes a job. A seat binds one role to one Agent for one session. `clowder-ai` bakes the role into the cat because its team is fixed; VaneHub's agents are user-installed tools, so the same `claude-code` must be able to review in one session and architect in another.

*Alternative — role as an Agent attribute:* rejected. It forces one role per installed tool and makes a second reviewer impossible without installing a second CLI.

### Seats are an initial line-up, not a fixed roster

Seats can be added and removed while a session runs. Fixing them at creation would repeat, at a
smaller scale, the mistake that killed the DAG approach: forcing the user to plan the collaboration
path before any work has happened. A path emerges — you start with an architect and an implementer
and only then discover you want a reviewer.

`clowder-ai` goes further: it has no per-thread roster at all. Its roster is global, and
participation is *derived from message history* — `concierge-target-cats-resolver` reads the last
few non-system messages to decide who is in the conversation, and any cat can be pulled in by being
mentioned.

That is not adopted wholesale here because VaneHub's Agents are CLI processes. Each seat is a real
process with a real context budget, so a fully emergent model would leave the user unable to see how
many CLIs a session is burning. Creating a session with an explicit initial line-up keeps that cost
visible; allowing seats to change afterwards keeps the collaboration path open.

### Role text injects through the CLI's native system-prompt channel

`--system-prompt-file` for Claude, `-c developer_instructions` for Codex. `clowder-ai` calls this the compression-immune layer and deliberately keeps per-invocation content out of it.

This matters more than it looks. If the role were prepended to each user message, a long session would compact it away and the Agent would quietly stop being the reviewer. Where an Agent exposes no such channel, the seat degrades to per-turn injection and says so, rather than pretending the role is durable.

### The roster is injected alongside the role

An Agent cannot hand off to a teammate it does not know exists. Each seat receives the other seats' role names, mentions, and model families. `clowder-ai` builds exactly this list and tags entries by family and lead status — which is also why a role's one-line responsibility is required rather than decorative: it is the text other Agents read when choosing whom to mention.

### Prior turns reach an Agent by resume when possible, by injection otherwise

Two strategies, mirroring `clowder-ai`'s `incrementalMode`:

- The seat's Agent has a provider session id → resume it; the history is already there and must not be re-injected.
- Otherwise → inject preceding replies as attributed text, trimmed to a per-seat context budget.

VaneHub already tracks provider/runtime session ids for resume, so the cheaper path is usually available.

### Handoff is serial, line-leading, and bounded

Routing happens only after a reply completes, only for mentions at the start of a line, and only outside fenced code blocks. Chain depth, mentions per message, and self-mention are all bounded.

*Why line-leading:* `clowder-ai` simplified to this rule (dropping action-word detection) because any mention anywhere makes ordinary prose route unpredictably — writing "ask @reviewer about this" should not dispatch anyone.

*Why bounded:* depth limits only matter when agents mention each other autonomously. Their existence in `clowder-ai` is itself evidence that unbounded A2A chains occur in practice.

### Handing to the human carries an intent

Three intents with different blocking behaviour: informational (work continues), blocking (the round pauses and a waiting duration accumulates), completion (the round ends).

The failure mode this avoids is subtle: with a single "notify the human" action, every notification blocks, so agents learn to avoid notifying and the human loses visibility. Separating the informational case keeps the channel cheap enough to use.

### Model family is normalized, and cross-family preference degrades openly

`provider` is display text, so a normalization step maps built-in agents to families and infers families for custom API agents from their endpoint type. When a review-eligible role prefers a different family and none is available, same-family Agents are still offered with an explicit notice — `clowder-ai` shipped that degradation as a fix after the strict version left users unable to assign a reviewer at all.

### Tabs gain a scope, not a multiplier

Terminal transcript, Shell, and logs are seat-scoped with an in-tab switcher. Workspace, changes, documents, files, and report stay session-scoped. The execution trace stays session-scoped and distinguishes entries by seat. Multiplying nine tabs by the seat count would be unusable at three seats.

## Risks / Trade-offs

- **Context cost grows with seats** → Resume-first keeps the common path cheap; per-seat budgets bound the fallback. Still, a five-seat session will cost more than five single-Agent sessions, and that should be measured before widening limits.
- **Agents may not reliably emit line-leading mentions** → The roster injection and role responsibilities are the mitigation, but this is genuinely uncertain across CLI vendors and should be validated with real agents early rather than assumed.
- **A paused round can stall silently** → The waiting duration is displayed. This change deliberately stops short of automatic reclamation.
- **Seat model touches the session entity** → Existing sessions migrate to a one-seat representation; the single-seat path must stay visually and behaviourally identical or every existing user pays for a feature they did not enable.
- **Roles without a consumer repeat a known failure** → The two capabilities ship together. This is the same trap as the retired coordination backend, one noun later.

## Migration Plan

1. Persist expert roles and ship the settings page with built-in starter roles.
2. Extend the session entity to seats, migrating existing sessions to one seat, and add speaker identity to messages.
3. Enable the `multi` mode, seat assignment, and cross-family recommendation.
4. Add role and roster injection, then handoff parsing with its safety rails.
5. Add the turn-status surface and the three human intents.
6. Scope the three seat-level tabs.

**Rollback:** the `multi` mode returns to disabled. Seats and speaker identity are additive and can remain persisted; single-seat sessions continue to work unchanged.

## Open Questions

- Should roles carry `global`/`workspace` scopes like Skills, or remain global only? Global-only is simpler and is assumed here.
- May a session hold two seats with the same role, for example two reviewers? The seat list allows it; whether the UI should encourage it is unresolved.
- What are the right initial values for chain depth and mentions per message? `clowder-ai` runs 15 and 2; whether those transfer is unknown until measured.
