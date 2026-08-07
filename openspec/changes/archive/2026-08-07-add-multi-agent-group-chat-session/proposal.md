## Why

Users who keep several coding agents installed want them working the same problem together, not one at a time. The create-session dialog already ships a `SessionAgentMode` selector whose `multi` option is rendered but disabled with a "coming soon" hint — the product promised this and never delivered it.

The first attempt, `multi-agent-coordination`, is being retired in a separate change. It failed for a specific reason worth naming: it was a DAG. Before anything could run, the user had to author a plan graph — nodes, prerequisites, primary and fallback Agents. That is a planning surface, and a session is a conversational one. Nobody wanted to draw a diagram to start a conversation, so no UI was ever built for it and the backend sat unreachable.

A study of `clowder-ai` (a shipping multi-Agent platform driving the same class of CLI agents) shows a paradigm that fits a conversational surface: a shared thread where agents hand off to each other by `@mention`, with the human as a first-class participant who can be handed the ball but never routes traffic. Nothing is planned up front; collaboration emerges one turn at a time. That is the shape this change adopts.

## What Changes

- Add expert roles as reusable settings assets: identity (name, avatar, color), a one-line responsibility, the role instruction injected into the Agent, optional Skill references, and peer-review eligibility.
- Open the existing `multi` session mode. Creating a multi-Agent session assigns **seats**, each pairing one expert role with one Agent, so the same Agent can play different roles in different sessions.
- Add a shared session thread where every message carries speaker identity, rendered with the role's avatar, colour, and a `role · Agent` label.
- Add Agent-to-Agent handoff: after an Agent's reply completes, a **line-leading** `@role` routes the turn to that seat. Safety rails bound the behaviour — maximum chain depth, maximum mentions per message, self-mention filtering, and fenced-code-block stripping.
- Add three handoff intents toward the human — `fyi` (informational, work continues), `handoff` (work pauses, the human holds the turn, a staleness timer starts), and `done_notify` (the round is complete). Only `handoff` interrupts.
- Add a persistent turn-status bar showing who currently holds the turn, the chain position against its limit, and how long a paused turn has been waiting.
- Recommend a reviewer from a **different model family** when assigning a peer-review seat, degrading to a same-family recommendation with an explicit notice when no cross-family Agent is available.
- Scope the nine workspace tabs: terminal transcript, Shell, and logs become seat-scoped with an in-tab seat switcher; workspace, changes, documents, files, and report stay session-scoped; the execution trace stays session-scoped but colours entries by seat.
- **BREAKING** A session's single `agentId` becomes a seat list, and chat messages gain a speaker field. Existing single-Agent sessions migrate to a one-seat session with no role assigned.

## Capabilities

### New Capabilities

- `expert-role-management`: Defining, editing, and storing reusable expert roles in settings, including their instruction text, visual identity, Skill references, and peer-review policy.
- `multi-agent-group-chat`: Seat assignment, the shared thread with speaker identity, Agent-to-Agent handoff with its safety rails, the three human handoff intents, turn ownership and its status surface, and cross-family reviewer recommendation.

### Modified Capabilities

- `session-management`: A session carries an ordered list of seats instead of a single Agent id, and single-Agent sessions become the one-seat case rather than a separate concept.
- `chat-experience`: Messages carry and render speaker identity, and the composer supports `@` seat completion with line-leading routing semantics.
- `session-workspace-tabs`: Tabs declare whether they are seat-scoped or session-scoped, and seat-scoped tabs expose an in-tab seat switcher.
- `settings-center-ui`: Adds an Expert Roles navigation entry.

## Impact

- **Desktop and Web runtimes:** Both. Role storage, seat persistence, and turn state go through the Rust SQLite layer on desktop and the Web/mock adapter in the browser, which must stay interface-identical.
- **Frontend:** Extends `MessageItem` with speaker identity (its avatar slot and header already exist), adds the turn-status bar and `@` completion to the chat surface, adds a seats view to the session info panel, adds the Expert Roles settings page, and adds a seat switcher to three workspace tabs.
- **Backend:** New role and seat persistence, turn-ownership state, and `@mention` parsing with its safety rails. Role instructions inject through each CLI's native system-prompt mechanism so they survive context compaction, reusing the existing CLI parameter plumbing.
- **Architecture:** No change to frontend/backend isolation. React continues to depend only on `AgentService`.
- **Dependencies:** Cross-family recommendation needs a normalised model family. `AgentRegistryEntry.provider` exists but holds free-form display text such as `"OpenAI"`, so a normalisation step is required rather than direct string comparison.
- **Reuse, not duplication:** Skills remain capability bundles that roles may reference; Prompt Hooks remain lifecycle injection and are orthogonal to role identity; per-Agent authorization reuses the existing `agent-tool-trust` capability rather than introducing a parallel approval system.
- **Sequencing constraint:** Expert roles MUST NOT ship without the session surface that consumes them. Shipping a producer with no consumer is precisely the failure being cleaned up in `remove-multi-agent-coordination`, and repeating it here would be the same mistake with a different noun.
- **Not in scope:** Turn liveness probing, zombie-turn reclamation, and parallel fan-out. `clowder-ai` bounds its own parallel route so it never chains; this change ships serial handoff only and leaves liveness to a later change once real failure modes are observed.
