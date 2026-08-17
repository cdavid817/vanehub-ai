# Todo Board: manual work and Agent activity side by side

**Status: Implemented — the interface is identical on desktop and in Web/mock; Web/mock does not use SQLite, and reloading the page clears it.**

## Overview

Sessions, plans, and scheduled tasks each have their own list, and manual to-dos have nowhere to live at all. The Todo Board collects all four into one board: **what you wrote down by hand and what the Agents produced sit side by side**, organized by the same stages, priorities, and filters.

The key design is that **the board stage and the source status are kept completely separate**: which column a card is in is your decision, while how far the source has got is projected by the runtime. Neither disturbs the other — a plan finishing does not move its card to "Done" by itself.

## Open the board

Select **Todo Board** in the activity bar. There are two views at the top, **Active board** and **Archive**; you land on the active board, where archived items are not shown.

## The five stages

| Stage | Purpose |
| --- | --- |
| **Inbox** | Where new work items land by default |
| **Planned** | Scheduled to be done |
| **In Progress** | Being worked on |
| **Review** | Done, awaiting confirmation |
| **Done** | Finished |

**Stages are entirely under your control.** Nothing moves a card automatically.

There are two ways to move a card: drag it, or use the **Move to previous stage** / **Move to next stage** buttons on the card. **Both paths produce exactly the same persisted result** — the explicit buttons are not a degraded substitute for dragging, they are an equivalent operation. Keyboard and screen-reader users can work the whole board, and nothing gets clipped on a narrow screen.

## Create a manual to-do

Select **New work item** and fill in:

| Field | Notes |
| --- | --- |
| **Title** | Required |
| **Description** | Optional |
| **Project path** | Optional; used for filtering by project |
| **Priority** | No priority / Low / Medium / High / Urgent |
| **Due date** | Optional; shown on the card as "Due …" |

A work item created without a source lands in the **Inbox** and survives a restart.

## Sources and automatic reconciliation

Beyond hand-written to-dos, the board **reconciles** existing and future execution objects into work items automatically:

| Source | Notes |
| --- | --- |
| **Session** | Top-level sessions |
| **Plan** | The plan itself |
| **Plan run** | A single run |
| **Scheduled task** | A scheduled task |

Reconciliation is **idempotent**: the first time you open the board after an upgrade, every eligible unlinked source gets **exactly one** work item; sources created later are picked up on the next reconciliation. Repeated reconciliation does not produce duplicate cards.

Three rules are easy to miss:

- **A child session does not get its own card.** A session created for a plan attempt or a scheduled task run appears as activity under the owning work item, and does **not** become an independent top-level card. Otherwise a plan running ten rounds would spray out ten cards.
- **An archived work item is not recreated by reconciliation.** What you archived stays recognized on the next pass; no replacement card appears.
- **One card can carry several sources.** A card linked to both a session and a plan matches both the Session and the Plan source filters, but **it remains one card** and is never split into two by filtering.

When a source is deleted or cannot be resolved, **the work item is still there**, marked **Unavailable** — the card does not disappear with it.

## Search and filters

The top of the board offers a search box and four filters, which **combine**:

| Filter | Options |
| --- | --- |
| **Filter by source** | All sources / Session / Plan / Plan run / Scheduled task |
| **Filter by stage** | All stages / one of the five stages |
| **Filter by priority** | All priorities / No priority / Low / Medium / High / Urgent |
| **Filter by project** | All projects / each project path |

When a filter matches nothing you get "No items match the current filters", which is a different message from "No work items" shown when the board itself is empty.

## Archive and delete

| Action | Effect | Effect on sources |
| --- | --- | --- |
| **Archive work item** | Leaves the active board | None |
| **Restore** | Returns to its stage and ordering position | None |
| **Delete permanently** | Deletes the work item and its source links | None |

**None of the three touch the source.** Deleting permanently removes only the work item itself and its link records; the linked sessions, plans, plan runs, and scheduled tasks are **left exactly as they were**. The board is an organizing layer, not the owner of those things.

Restoring returns the item to **the stage and ordering position it had before archiving**, not to the Inbox.

## Notes and limits

- **Web/mock does not use SQLite.** Creation, reconciliation, filtering, movement, and archiving all behave, but reloading the page clears everything.
- **Stages never advance on their own.** A change in source status only updates the source projection on the card; it does not move the card.
- **Reconciliation is not live sync**; a new source appears at the next board reconciliation.
- **Archiving is not deleting.** Archived items remain in the Archive view and protect themselves from being recreated by reconciliation.
- Deleting a work item cannot be undone, but **it affects none of the linked execution objects**.

## Related

- Group work items under a larger objective → [Goal management](goal-management.md)
- One of the card sources, the automatic cycle → [Loop Engineering](loop-engineering.md)
- How scheduled tasks themselves are configured → [Scheduled and usage](automation.md)
