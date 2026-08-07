# Multi-Agent group chat

**Status: Delivered — desktop runtime; seat assignment is visible in the normal create-session dialog.**

A session can hold several Agent **seats**. Each seat pairs one Agent with one expert role, and every seat reads the same shared thread.

## Assign seats

1. Open VaneHub AI and select **New**.
2. In the seat area, add a seat for each Agent you want in the session.
3. Give each seat an expert role. The role name becomes the handle other seats type after `@`.
4. Choose the project directory, then create the session.

## Hand the turn over

A seat hands the turn to another seat by mentioning its handle in the reply, for example `@reviewer`. Typing `@` in the composer opens seat completion.

- Mentions inside fenced code blocks are ignored, so pasting sample code does not trigger a handoff.
- A seat cannot hand the turn to itself, and repeating a handle counts once.
- A round ends normally when a reply mentions nobody.

## Hand back to the human

A reply can address you with `@用户`:

| Written as | Effect |
| --- | --- |
| `@用户 handoff` | You take the turn; the round waits for you |
| `@用户 done` | The round is complete |
| `@用户` alone | Informational only; the flow continues |

Only `handoff` interrupts. A bare mention is deliberately cheap so Agents keep using it.

## Limits

The runtime bounds both the number of mentions in one reply and the depth of a handoff chain. When either bound stops a chain, the reason is shown rather than the chain stopping silently.

Multi-Agent execution requires the desktop runtime. In Web/mock the seat controls render, but no CLI process runs.

> The earlier dependency-graph coordination runtime described in previous revisions of this chapter has been removed. Group chat replaces it.
