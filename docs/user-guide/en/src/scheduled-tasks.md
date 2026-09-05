# Scheduled tasks and notifications

Tasks that run automatically on a schedule, long-running operations in the background, and how you are notified when they finish.

Token usage is in [Usage statistics](usage-statistics.md).

## Scheduled tasks

### Overview

Turn repetitive work into a recurring task that automatically creates a session and runs when it is due.

### Create one

Select **Runs** in the activity bar, then switch to the **Scheduled tasks** tab: a filterable list of existing tasks, each opening its own detail with run history. Select **New task** (or **Edit task** on an existing one) to open the task editor:

| Field | Notes |
| --- | --- |
| **Task name** | For example "daily project progress summary" |
| **Task content** | What you want the Agent to do on schedule |
| **Agent tool** | Which Agent runs it |
| **Frequency** | See the table below |

**Five frequencies:**

| Frequency | Parameter |
| --- | --- |
| By minute | Interval in minutes |
| By hour | Interval in hours |
| Daily | Time of day |
| Weekly | Day of week + time |
| Monthly | Day of month + time |

The interval must be positive. A task card shows its **next run time**, and can be **enabled or disabled** at any time without deleting it.

![The scheduled tasks editor: task name, task content, Agent tool, and frequency fields](assets/screenshots/scheduled-tasks-en.png)

### How the time is computed

"9 am daily" computes the next run time in **your local time zone**; the due check uses absolute time, which avoids double runs and skipped runs across a daylight-saving transition.

### What happens while the application is closed

**The scheduler runs inside the application; it is not a system-level service** — nothing fires while the application is not open. But it is **not simply discarded**, and the interface explains:

> Tasks run while VaneHub AI is open. A run missed because the application was closed is **made up at the next launch, and only the most recent one is made up**.

In other words: a once-daily task with the application closed for three days makes up **one** run on restart, not three.

Runs triggered by a scheduled task are marked in the [trace](observability.md) with their source and task identifier, so they can be traced back.

## Long-running operation tracking

Time-consuming operations such as installing an SDK, connecting an MCP server, or installing an extension have explicit queueing and state: **queued → running → succeeded / failed / cancelled**, with output recorded line by line.

That output is **both** shown in the interface and written to the unified log directory.

## Notifications

Notifications come in four kinds: success, error, warning, and info.

There are two scopes:

- **Global** — application-level notices
- **Session-scoped** — relevant only to one session

Session-scoped notifications do not drown in the global notice stream; they are the most relevant context when you switch to that session.

## Notes and limits

- **Desktop only.**
- **A scheduled task needs the application running**; a missed run is caught up once, never replayed in a series.
