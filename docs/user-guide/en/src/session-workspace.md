# Session workspace

The nine tabs inside a session, what the Agent changed, run state and evidence, panel visibility, session recovery, and OnePiece's Plan mode.

The main window's own layout and navigation are in [User interface](user-interface.md).

## Workspace tabs

![The session workspace: session list on the left, workspace in the middle, info panel on the right, nine tabs across the top](assets/screenshots/session-workspace-en.png)

Once a session is open, nine tabs sit across the top of the workspace:

| Tab | What it does |
| --- | --- |
| **Workspace** | The conversation with the Agent; the default tab |
| **Changes** | Which files the Agent changed, with a diff view (unified/split toggle, per-file review, Git status) |
| **Documents** | Browse documents inside the workspace |
| **Files** | Browse workspace files |
| **Terminal** | Commands the Agent ran, and their output |
| **Shell** | An interactive terminal for your own use |
| **Logs** | Logs for this session; searchable and seekable by time |
| **Traces** | Execution tracing (run list + span tree + per-seat tracing) — see [Observability](observability.md) |
| **Report** | Token usage (input/output/character count), a token distribution bar, and counts by message state |

**The Terminal tab and the Shell tab are not the same thing**: the first records what the Agent did, the second is a terminal for you to type in. The Agent also has a **dedicated terminal** separate from your Shell. The numeric badge on a tab is the record count; when there is a lot of data, loading is bounded and only part of the results may be shown, which the interface tells you.

The **Logs** tab is searchable and seekable by time:

![The Logs tab of the session workspace](assets/screenshots/session-logs-en.png)

The **Traces** tab shows this execution's span tree, answering "what exactly did this step call, and how long did it take":

![The Traces tab of the session workspace, showing the execution span tree](assets/screenshots/session-traces-en.png)

## See what the Agent changed

The **Changes** tab shows the Agent's file edits:

- A file list with Git status (added/modified/deleted)
- Select a file to see its diff
- Toggle between **unified** and **split** diff views
- Review file by file

## Session information and run state

The info panel on the right of the workspace is the session's "dashboard" — a glance tells you what state the session is in, who's driving it, and what it has cost. Field by field:

| Field | Meaning |
| --- | --- |
| **Session info** | Session title, type (Single Agent / Multi Agent), category, pinned and archived state |
| **CLI tool** | Which CLI (or OnePiece) the session's bound Agent uses, and its availability status |
| **Run state** | Five states: **Idle / Starting / Running / Failed / Stopped**; the interface disables repeat submission during `Starting`/`Running` so a double-click can't open two tasks |
| **Model for this run** | The model actually used in this round of conversation; shows "No model configured" when none is set |
| **Token usage** | Input / output / cache read / cache write / total; the two cache figures are recorded separately, and the panel's total is their sum |
| **Workspace path** | The current workspace directory (a local path, a worktree, or a remote SSH path) |

The info panel also carries two in-place tabs, so you do not have to jump to the settings center:

- **Skill** — view and manage the Skills bound to this session, in the session
- **Code Index** — view the workspace code index status, in the session

> Token usage is reported by each CLI itself; VaneHub AI does not meter it independently. Read the [Usage statistics](usage-statistics.md) page's methodology note before using these numbers for cost accounting.

## Show and hide panels

The **overflow menu** (⋯) at the top right of the workspace toggles panel visibility: the session list, the info panel, and the display switch for each workspace tab.

## Session recovery

When you reopen a session after a crash or an abnormal exit, a **recovery banner** appears at the top explaining that the session was reconciled, quarantined, or needs your explicit acknowledgement.

## OnePiece Plan mode

OnePiece sessions expose **Plan** and **Agent** in the conversation bar. Plan mode is for read-only exploration and planning: it can inspect project context but cannot run shell commands, write files, call effectful MCP tools, or delegate work.

When the plan is ready, OnePiece can request `exit_plan_mode`. Approving the request changes the session to Agent mode for a later turn; declining keeps Plan mode active. The left activity bar has no separate Plan execution destination, and planning does not create a task graph or worktree.

Use **Loop** when you need durable autonomous iteration with verification and acceptance controls. Goal-level tracking is covered in [Goal management](goals-and-work-board.md).

## Related

- Main window layout → [User interface](user-interface.md)
- Traces and logs → [Observability](observability.md)
- Session terminal → [User interface](user-interface.md)
