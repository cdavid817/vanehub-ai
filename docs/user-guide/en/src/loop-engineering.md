# Loop Engineering: let the Agent iterate until it gets there

**Status: Implemented — desktop only.**

## Overview

Given a goal and a set of **must-pass checks** (for example `npm run lint` and `npm test`), a Loop drives the Agent through "act → verify → judge" repeatedly until the goal is met or a limit is reached.

What it solves is the manual cost of the cycle "I changed something, now run the tests, the tests failed, change it again".

How it differs from [Multi-Agent group chat](multi-agent-workflow.md): group chat moves the turn around inside a conversation and the Agents decide who gets it; a Loop is a **goal-driven automatic cycle** advanced by the runtime phase by phase, with every limit enforced.

## Create a Loop

The interface has three columns: **definitions** and **run records** on the left, the main area in the middle, and the **inspector** on the right. On first entry the middle says there are no Loop definitions yet.

1. Select **Loops** in the activity bar.
2. Select **+** in the left column to create a definition, and fill in the **goal** — what is to be achieved.
3. Set the **limits**:
   - Maximum iterations (**must be between 1 and 20**)
   - Per-step timeout and total timeout (the total must not be lower than the per-step value)
   - Tolerance for consecutive run errors and consecutive no-progress rounds
4. Add the **verification commands** — the must-pass checks. These are the objective criteria.
5. Save and start.

![The Loop centre three-column layout with no Loop definitions yet](../assets/screenshots/loop-center-en.png)

## The five phases of one iteration

Every round advances in a fixed order: **prepare → act → verify → judge → wrap up**.

The judging step short-circuits in strict priority order:

| Order | Condition | Result |
| --- | --- | --- |
| 1 | A hard termination reason is hit | Completed-pending-acceptance / cancelled / failed, by reason |
| 2 | The verifier returns "blocked" | Failed |
| 3 | **Not all must-pass checks passed** | Next round |
| 4 | The verifier asks for another round | Next round |
| 5 | None of the above | Moves to **awaiting manual acceptance** |

**Must-pass checks rank above the verifier's opinion.** An objective, deterministic check beats a subjective judgement — if the verifier says "this looks fine" while lint is still failing, it still has to be fixed.

## Manual acceptance is mandatory

**This is the Loop's most important safety design: an automatic cycle never declares its own success.**

Even when judged "goal achieved", the run only moves to **awaiting acceptance**, with a note that meeting the goal still requires human confirmation. Until you confirm, it stays there.

## No-progress detection

A Loop compares the goal state across rounds to recognize spinning in place. **Any one of the following counts as progress:**

- The code changes are substantively different
- A different set of checks is failing
- **A check that previously failed now passes**

The third is especially useful: suppose a round only fixed lint and touched nothing else — the code change may look the same as the previous round, but "a check that newly passes" is recognized as real progress.

Only when **all three dimensions are unchanged**, for as many consecutive rounds as your configured tolerance, does it terminate with "no progress".

## Inspect the execution

| What you want to see | Where |
| --- | --- |
| Phase transitions per round | The timeline view |
| One round's actions and check results | The iteration detail |
| Overall run state | The run control bar |

The run control bar also offers **pause** and **cancel**.

## Notes and limits

- **Desktop only**, because it depends on local processes to run the verification commands and Git.
- **Maximum iterations is hard-capped at 20** and cannot be raised by configuration.
- **Manual acceptance is required** — with nobody to confirm, a run sits in awaiting-acceptance indefinitely.
- **A Loop works in its own Git worktree**, so it does not pollute your main working copy — which also means it **does not work with a remote workspace**, since remotes do not support worktrees.
- **No-progress detection can be defeated by fake changes**: if every round produces a meaningless but different code change, the detection never fires.
- A running Loop does not continue after the application exits completely; on restart, interrupted runs are reconciled.
