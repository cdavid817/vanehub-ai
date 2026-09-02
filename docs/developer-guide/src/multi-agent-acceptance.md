# Multi-Agent group chat acceptance

This chapter is an **acceptance procedure**, not a usage tutorial. It walks an architect → implementer → code-reviewer case step by step, checking that membership, speaking state, `@` handoff, and historical attribution behave as expected in a multi-Agent session, and ends in a test record you can file.

For day-to-day use of multi-Agent group chat see the user guide's [Multi-Agent group chat](../../user-guide/en/src/multi-agent-workflow.md); for how it is built see [Multi-Agent group chat](multi-agent-group-chat.md).

## The case goal

This case only asks the Agents to review a small change proposal; it does not ask them to modify the repository:

> Design a `/health` health-check endpoint for this project. The architect defines the interface and its boundaries, the implementer gives the implementation steps, and code review checks tests, security, and compatibility.

That produces a continuous chain of role handoffs without introducing unrelated code just to exercise the UI.

## Step 1: choose your runtime

**Verifying real Agent replies and automatic handoff requires desktop**, with the CLIs you plan to use installed and authenticated.

With only one CLI available you can bind all three roles to the same Agent; seat identity still comes from the role.

## Step 2: create a session with three roles

1. Select **New**.
2. Choose your project directory.
3. Set the session title to `Multi-Agent health check review`.
4. Set Session Type to **Multi Agent**.
5. Configure these seats:

| Order | Role | Suggested Agent | Responsibility |
| --- | --- | --- | --- |
| 1 | Architect | Claude Code, Codex CLI, or any available Agent | Define the interface, boundaries, and constraints |
| 2 | Implementer | Codex CLI or any available Agent | Give the implementation steps and test plan |
| 3 | Code review | Prefer a different model family | Check for omissions, risks, and regressions |

6. Select **Create**.

### Checkpoint A: the creation dialog

- At least two seats must remain for it to be a multi-Agent session.
- Every seat can choose a role and an Agent.
- Adding or removing a seat does not lose the other seats' selections.
- When only one seat is left, the last remove action should be disabled.

## Step 3: check how members are shown in one session

Once inside the session, look at the top of the chat area and at the info panel on the right.

### Checkpoint B: the collaboration room header

- In the session list on the left, the session title carries a **Multi Agent** tag; a single-Agent session does not.
- Below the title, **Multi Agent** and the member count are shown.
- The member strip lists all three roles rather than only the first Agent.
- Each member shows both the role name and the Agent identity.
- The current speaker has a highlighted border, a status dot, and "working" text; the state must not be conveyed by colour alone.
- Generation state appears in the session header and returns to "ready" once generation stops.

### Checkpoint C: member management on the right

- The **Session members** heading shows the current count.
- Each row shows the role, the Agent, and the model family.
- When adding a member, both the Agent and the role selector have accessible names.
- The leave button has a clear hover hint and never allows the session to reach zero members.

## Step 4: perform the role handoffs

Send this prompt in the input box:

```text
We are only reviewing the design, not changing files.
As the architect, first define the response, failure boundaries, and compatibility constraints of the /health endpoint.
End your reply with @实现者 on its own final line to hand over to the implementer.
```

After the implementer replies, have it hand off again:

```text
Give the minimum implementation steps and the tests that must be added.
End your reply with @代码审查 on its own final line.
```

Finally, have code review close the round:

```text
Review the design above, list blocking issues and suggestions, then end the round with @用户 done.
```

### Checkpoint D: handoff and message identity

- Typing `@` brings up session-member completion, not file completion.
- The turn status bar shows the architect, the implementer, and code review working in turn.
- Agent messages carry the corresponding Agent's brand icon on the left.
- The message header shows "role name · Agent name", and user messages never impersonate a seat.
- All three roles' messages appear in one timeline; three separate chat pages should not open.

> A model does not always emit the requested `@` exactly. If no handoff triggers, restate the requirement that the final line contain only the target handle.

## Step 5: verify joining, leaving, and history attribution

1. Add another member in the **Session members** area on the right.
2. Confirm the member count and member strip in the chat header update immediately.
3. Have the new member produce at least one reply.
4. Select that member's leave button.
5. Expand **Members who left**.

### Checkpoint E: stable identity

- The departed member disappears from the current member strip and appears in the left list.
- Messages sent before leaving still show the original role name and Agent name.
- Other members' message attribution does not change when the member order does.
- After a refresh or reopening the session, historical speakers remain consistent.

This is the most important regression point: a member leaving may change the current line-up, never the history.

## Step 6: verify single-Agent sessions are unaffected

1. Create another session.
2. Set Session Type to **Single Agent**.
3. Send an ordinary message.

### Checkpoint F: compatibility

- No multi-member strip appears.
- No seat switcher or multi-Agent routing state appears.
- Messages keep using the ordinary Agent label.
- Existing Agent Terminal behavior is unchanged.

## Test record template

| Checkpoint | Result | Evidence or notes |
| --- | --- | --- |
| A: creation dialog | Pass / Fail |  |
| B: collaboration room header | Pass / Fail |  |
| C: member management | Pass / Fail |  |
| D: handoff and message identity | Pass / Fail |  |
| E: stable historical identity | Pass / Fail |  |
| F: single-Agent compatibility | Pass / Fail |  |

## Related

- The full description of the mechanism and its limits → [Multi-Agent group chat](../../user-guide/en/src/multi-agent-workflow.md)
- A seat cannot get the turn, or `@` does not trigger → [Troubleshooting](../../user-guide/en/src/troubleshooting.md)
- The matching automated tests and implementation detail → [the Developer Guide's multi-Agent group chat chapter](multi-agent-group-chat.md)
