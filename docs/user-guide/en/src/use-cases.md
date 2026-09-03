# Use cases

Five end-to-end scenarios. Each uses only **implemented** functionality.

---

## Case 1: two Agents working on unrelated tasks in one repository

**Situation**: one repository has two unrelated jobs — fixing a bug and filling in a batch of tests. You want both moving at once without the two Agents interfering with each other.

**Goal**: two Agents working independently, with no cross-contamination of context.

### Steps

1. Select **New**, set **Session Type** to **Single Agent**, and choose **Claude Code**.
2. Set **Workspace** to **Local** and pick the project directory. Since it is a Git project, tick **Create new Git worktree** and name it `fix-bug`.
3. Title it "Fix login timeout" and create it.
4. Select **New** again, this time choosing **Gemini CLI**.
5. Same project directory, tick **Create new Git worktree** again, and name it `add-tests`.
6. Title it "Add tests for the auth module" and create it.

### What happens

The two sessions work in **separate Git worktrees**, so their branches do not affect each other and their file changes cannot conflict.

By default a worktree is created **in a sibling directory of the project**, with a branch name of the form `vanehub/<worktree-name>`.

You can switch between them at any time in the session sidebar, and each session's **Changes** tab shows the Git status of its own worktree.

> **Note**: the two sessions share one policy template and one set of personalization settings. To relax permissions for only one of them, configure it per Agent under **Agent Policies**.

---

## Case 2: automating "change code → run tests" with a Loop

**Situation**: a refactor left several tests failing, and you would rather not run rounds of "ask the Agent to fix it → run the tests myself → paste the failures back".

**Goal**: let the Agent iterate until the tests are green, with you making the final call.

### Steps

1. Select **Loops** in the activity bar and create a definition.
2. Set the **goal**: "Fix the failing tests under `src/auth` without changing the test files themselves."
3. Set the **limits**:
   - Maximum iterations `8`
   - Per-step timeout `600` seconds, total timeout `3600` seconds
   - No-progress tolerance `2` rounds
4. Add two **verification commands**:
   - `npm run lint:ci`
   - `npm run test`
5. Save and start.
6. Watch the phase transitions per round in the timeline view, and each round's check results in the iteration detail.
7. Once the run reaches **awaiting manual acceptance**, review the output and confirm.

### What happens

Each round advances through "prepare → act → verify → judge → wrap up". **While a must-pass check is failing, the next round starts even if the Agent believes it is done.**

If for two consecutive rounds the code changes, the set of failing checks, and the set of passing checks are **all three unchanged**, it terminates with "no progress" — so it does not spin in a dead end.

**A Loop never declares its own success.** Even when it judges the goal met, it stops at awaiting acceptance for you to confirm.

> **Limits**: a Loop works in a separate worktree, so it **does not apply to a remote workspace**; the iteration cap is hard-limited to 20.

---

## Case 3: configuring approvals for sensitive operations, and handling one

**Situation**: you want an Agent working in a production configuration repository, free to read anything, but with every write and command execution confirmed by you.

**Goal**: configure least privilege and walk one approval end to end.

### Steps

1. Open **Settings → Agent Policies**.
2. Set the target Agent's template to **Standard**.
   - For something stricter, choose **Read-only** — but note that Read-only **denies** writing files and running commands, so the Agent cannot complete any task that changes something.
3. Return to the session and give the Agent a task that requires changing a file.
4. When the Agent tries to write, execution stops and a "needs approval" notification appears at the bottom right.
5. Find the **tool call block** in the conversation whose state is `awaiting_approval` and **expand it** — the approval area is inside the collapsed content.
6. Review **Agent / Action / Resource**, pick a scope under **Remember my choice:**, and select **Approve**:
   - **Just once** — you will be asked again on the next write
   - **This project** — equivalent actions in this project are no longer asked about

### What happens

Under Standard, **reading files is always allowed**, while writing files and running commands are asked about every time.

Every decision is written to an audit record, including those allowed or denied outright, so you can go back later and see who approved what and when.

> **Two things that are easy to misread**:
> - **"Read-only" does not forbid everything** — reading files and writing memories are still allowed.
> - **"Trusted" and "Yolo" have identical policy in practice**; they differ only in how firmly you have to confirm when switching.

---

## Case 4: a daily automated health check with a scheduled task plus IM

**Situation**: you want a code-quality pass to run automatically every morning and the result pushed to the team chat, without watching it yourself.

**Goal**: run the check unattended and get notified.

### Steps

1. Configure IM first: **Settings → IM Connectors**, choose Feishu (or DingTalk / WeCom), and enter the application credentials you created on the open platform.
2. Select **Scheduled tasks** in the activity bar and create a task.
3. Set the frequency to **Daily** and the time to `09:00`.
4. Configure what the task should do — the target repository and the check.
5. Save and **enable** it.

### What happens

The task fires at 9 am **in your local time zone**, creating a session and running automatically.

The result can come back to the chat through the IM connector; you can also drive an Agent by messaging the chat directly, and sessions created by a connector are marked with their source.

In the **Traces** tab, such a run is annotated "triggered by a scheduled task" with the task identifier, so you can trace which task produced it.

> **The key limit**: the scheduler runs inside the application, so **nothing fires while the application is closed**. A missed run is made up at the next launch, but **only the most recent one** — three days closed does not make up three runs. For a dependable daily check, keep the application running.

---

## Case 5: investigating a failed run with the trace view

**Situation**: an Agent run failed, and the output contains only one vague error line.

**Goal**: pin down which stage failed.

### Steps

1. Open that session's **Traces** tab.
2. Read down the four layers from the root: session → Agent → tool/MCP boundary → process execution.
3. Find the node in a failed state and check its duration and error classification.
4. If the failed node has **Opaque** fidelity, it is internal behavior of an external CLI and the trace stops there — switch to the **Terminal** or **Logs** tab to continue.
5. In the **Logs** tab, search by the failing node's `runId`, `traceId`, or `spanId`; when those fields are absent, fall back to **seek** to jump near the time of the failure.

### What happens

A trace tells you **which stage was slow and which one broke**; the logs tell you **what it actually said**.

> **Line the two up by identifier first**: when the source supplies them, a log entry carries `runId`, `traceId`, and `spanId` in its context, so traces and logs correlate on the same set of ids. Only an external CLI's internal behaviour, an older record, or a degraded path that carries no context lacks these fields — that is when you fall back to time. Logs still never persist raw prompts or Agent output.

If you are using [OnePiece](native-agent.md), the trace carries considerably more information — its tool calls are native fidelity and expand layer by layer, whereas an external CLI shows only its boundary.
