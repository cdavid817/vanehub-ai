# Create your first session

A session is VaneHub AI's basic unit of work: it binds one workspace and one or more Agents, and carries the conversation, the terminal, file changes, and execution tracing.

## Create

1. Open VaneHub AI and select **New**.
2. For **Session Type**, choose **Single Agent** (several Agents working together is covered below).
3. Pick an available Agent under **Agent**.
4. For **Workspace**, choose **Local** or **Remote**.
5. Under **Project Folder**, browse to a directory or pick one from **Recently opened projects**.
6. Fill in the **Session name** (left blank, it becomes "New session").
7. Select **Create**.

![English create-session dialog using synthetic VaneHub Demo project data](assets/screenshots/create-session-en.png)

## Two cases when picking a directory

**Whether the directory you selected is a Git repository decides whether you get the worktree option.** The interface marks it:

| Marker | Meaning |
| --- | --- |
| **Git** | A Git project; a worktree can be created |
| **Folder** | A plain folder; worktree will be disabled |

Tick **Create new Git worktree** and fill in a **Worktree name**, and VaneHub AI derives the path and branch by a fixed rule:

> Default path: project sibling directory + `projectName-worktreeName`; branch: `vanehub/worktreeName`

**Why use a worktree:** it lets the Agent edit code in a separate working copy without touching your current branch.

## Remote workspace

Switch **Workspace** to **Remote**, then fill in **Host**, **Port**, **User**, and **Remote path** — or pick a saved **SSH connection** directly.

Tick **Save as SSH connection** to store what you entered for reuse. The first connection asks you to confirm the host key; see [Remote execution and IM connectors](remote-and-im.md) for detail.

## Several Agents working together

Choose **Multi Agent** for **Session Type** to assign seats:

> Several Agents in one thread, handing off by @mention

Each **seat** is one Agent plus one role. **Role** can be set to **No role**. Use **Add seat** and **Remove seat** to adjust.

If no Agent from a different model family is available, the interface says so and shows same-family options instead. For the full mechanism, see [Multi-Agent group chat and `@` handoff](multi-agent-workflow.md).

## The nine session workspace tabs

Once created, you land in the session workspace with nine tabs across the top:

| Tab | What it holds |
| --- | --- |
| **Workspace** | The conversation with the Agent; the default tab |
| **Changes** | Which files the Agent changed |
| **Documents** | Documents inside the workspace |
| **Files** | Workspace file browsing |
| **Terminal** | Commands the Agent ran, and their output |
| **Shell** | An interactive terminal for your own use |
| **Logs** | Logs for this session |
| **Traces** | Execution tracing — see [Observability and logs](observability.md) |
| **Report** | The session report |

**The Terminal tab and the Shell tab are not the same thing**: the first records what the Agent did, the second is a terminal for you to type in.

The numeric badge on a tab is the record count. The first release uses bounded loading, so with a lot of data only part of the results may appear; the interface tells you when that happens.

## Reference files in a message

Rather than pasting code into the conversation, you can reference files from the workspace.

In the file preview, **click one line and then another** to select a range, then:

| Action | Effect |
| --- | --- |
| **Attach selection** | Sends only the selected lines, marked as a line range |
| **Attach whole file** | Sends the whole file |

References appear as entries attached to the input box; **Remove file reference** takes one off.

**A message can reference at most 5 files.** Beyond that you get "reference limit reached" — the extras are not silently dropped.

**Referenced content is sent to the Agent along with the message**, so it counts toward this request's tokens too. It is worth checking the size before attaching a large file whole.

Three cases where preview is unavailable are stated rather than shown as a blank: **the file is binary and cannot be previewed**, **the file is too large to preview**, and **the file is unavailable**.

## Run state

**Run state** in the info panel on the right has five values: **Idle**, **Starting**, **Running**, **Failed**, **Stopped**.

The info panel also shows the **CLI tool**, the **model for this session**, the **workspace**, and this session's **token usage** (input / output / cache read / cache write / total). With no model configured it shows "No model configured".

## When creation fails

The dialog reports four kinds of error: **Agent unavailable**, **Project unavailable**, **Git worktree failed**, and **Command failed**.

"Agent unavailable" usually means the CLI is not installed properly or not authenticated — go back to [Install and authenticate a CLI](getting-started.md) and check. For the rest, see [Troubleshooting](troubleshooting.md).

## Next

- Unfamiliar terminology → [Core concepts](core-concepts.md)
- Want a full scenario → [Use cases](use-cases.md)
- Worried the Agent will change files it should not → [Permission approvals](permissions.md)
