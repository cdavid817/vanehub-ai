# Git worktrees: let an Agent edit code in its own working copy

**Status: Implemented — desktop only; available for Git projects, and not supported for a remote workspace.**

## What a Git worktree is

**A Git worktree is a second working copy of the same repository**, with its own directory and its own branch, sharing one Git history.

It is not a second copy of the repository. `git clone` copies all history; a worktree only checks out another branch into another directory — small on disk, and both sides commit into the same repository.

## What it solves in VaneHub AI

**It lets an Agent edit code without touching the branch you are using.**

Without a worktree, the Agent and you work in the same directory: you cannot switch branches while it is half-way through, saving a file while it runs tests interferes with it, and if it gets something wrong your working copy is dirty.

With one:

- **Your main working copy is untouched**, so you can switch branches and run your own things at any time
- **Several sessions can work on different tasks in the same repository in parallel**, each in its own worktree, with no conflicting file changes
- **If it goes wrong, throw that worktree away** and your main working copy is unaffected

That is what makes "two Agents working on unrelated tasks in one repository" in [Use cases](use-cases.md) possible.

## When it is available

When you pick a directory while creating a session, the interface first inspects whether that directory belongs to a Git repository and marks it:

| Marker | Meaning |
| --- | --- |
| **Git** | A Git project; a worktree can be created |
| **Folder** | A plain folder; **the worktree option is hidden or disabled** |

**That inspection starts no Agent and opens no interactive session** — it only looks at the directory.

A non-Git project can still create a session normally; it just has no worktree option.

## How the path and branch are derived

Tick **Create new Git worktree** and fill in a **Worktree name**, and the path and branch follow a fixed rule:

| | Rule | Example (project `C:\code\app`, name `feature-a`) |
| --- | --- | --- |
| **Path** | The project's **sibling directory** + `projectName-worktreeName` | `C:\code\app-feature-a` |
| **Branch** | `vanehub/worktreeName` | `vanehub/feature-a` |

Once the session is created, **that worktree path is the session's effective folder** — what the Agent sees, where commands run, and what file browsing shows.

## Three cases rejected up front

All are rejected **before any Git command runs**, so no half-made worktree is left behind:

| Case | Result |
| --- | --- |
| **The name is empty or unsafe** | Session creation is rejected |
| **The resolved target path already exists** | Rejected before `git worktree add` |
| **Git cannot be executed** | The interface gets a concise unavailable message |

**Failure information is layered**: the interface gets one concise sentence, while the full stdout, stderr, and diagnostics of `git worktree add` go to the unified logs. So the interface does not dump a screen of Git output at you, and nothing is lost for investigation — see [Observability](observability.md).

## A Loop's worktree is a separate arrangement

[Loop Engineering](loop-engineering.md) creates **its own dedicated worktree and branch for every run it starts**, and does so **before** creating role sessions or modifying any project file.

It differs from a worktree you make by hand in three ways:

- **The branch name is collision-safe**, rather than a fixed `vanehub/name`
- The run persists the canonical project path, worktree path, worktree name, and branch
- **All Worker and Verifier sessions, and every verification command, use that worktree as their bounded root**

If the proposed path or branch conflicts with an existing target, **preparation fails** before any role session is created or any file is mutated, keeping concise failure context and detailed redacted diagnostics.

### A Loop's worktree is never cleaned up automatically

**After a run succeeds, fails, is cancelled, is rejected, or is recovered on restart, the worktree is retained** until you manage it outside this capability.

The system **does not run `git worktree remove`, delete the branch, merge, or commit** on its own. It only exposes the path for you to review.

This is deliberate: the output of an automatic run should not be cleaned up before you have looked at it.

## Notes and limits

- **Desktop only**, because it depends on a local Git executable.
- **Available for Git projects only**; a plain folder has no such option.
- **A remote workspace does not support worktrees** — it can only point at a path that already exists there. Which is why [Loop Engineering](loop-engineering.md) **does not apply to a remote workspace** either.
- **An existing target path is rejected**, never overwritten or reused.
- **A Loop's worktree is never cleaned up automatically**, so accumulated directories are yours to manage.
- **VaneHub AI does not commit, merge, or push** the changes in a worktree for you.

## Related

- Where to tick it while creating a session → [Create your first session](first-session.md)
- The automatic cycle that depends on worktrees → [Loop Engineering](loop-engineering.md)
- A full walkthrough of parallel work in one repository → [Use cases](use-cases.md)
- Where Git failure detail goes → [Observability](observability.md)
- Where execution isolation sits in multi-Agent orchestration → [Multi-Agent systems technical architecture](../../../agent-infrastructure/multi-agent-architecture.md) (Simplified Chinese)
