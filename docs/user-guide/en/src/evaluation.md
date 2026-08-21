# Agent evaluation

## Overview

**Agent evaluation** in the left activity bar runs multiple Agents against **the same task, on the same starting code**, then lines the results up side by side.

It answers "**who should get this kind of task**" — not by feel, but by head-to-head comparison: the same prompt, the same project, the same verification commands, and who passed, how many tokens they burned, and how long it took.

**It runs a local benchmark, not your project.** Every attempt runs in its own directory copied from a built-in fixture — it never touches your workspace, and it never produces a commit.

## The shape of one evaluation

| Concept | What it is |
| --- | --- |
| **Benchmark task** | One problem: a prompt + a starting project + a set of acceptance rules |
| **Arena** | One run: one task × the Agents you check |
| **Attempt** | One cell in the arena: one Agent's pass at the task |

An arena has **at most 8 attempts**, at least 1.

## Three built-in benchmarks

The first version ships three tasks, covering three typical categories of work:

| Task | Category | Requirement | Timeout | Verification |
| --- | --- | --- | --- | --- |
| `fix-null-auth-token` | Bug fix | Fix null authentication-token handling while preserving existing behavior | 120s | `npm test`, static file checks, diff rules |
| `add-parser-test` | Tests | Add deterministic tests for a bounded parser's edge cases | 120s | `npm test`, diff rules |
| `refactor-search` | Refactor | Refactor search without changing its public result ordering | 180s | `cargo test`, diff rules |

**Each task carries a forbidden pattern**, and triggering it is an automatic fail:

- `fix-null-auth-token` forbids `eval(`
- `add-parser-test` forbids `.only(` — using it to skip the rest of the test suite is enough to turn the tests "all green"
- `refactor-search` forbids `unsafe {`

**All three patterns target the same category of cheating**: satisfying the letter of the task while evading its actual intent.

## How to run one

1. Open **Agent evaluation** in the left activity bar.
2. Pick a **benchmark task** from the top dropdown (shown as `task-id vVersion`).
3. Check the **Agents** you want to enter.
4. Click **Run arena**.

The running list refreshes every second until every attempt reaches a terminal state. Selecting a row lets you **cancel** it — this only works on attempts that haven't reached a terminal state yet.

> **Only onepiece and codex-cli can be selected right now.** The Agent checkboxes in the UI are fixed to these two; the other three CLIs cannot enter yet.

## Reading the results table

The results table has five columns:

| Column | Meaning |
| --- | --- |
| **Agent** | Which Agent's attempt |
| **Outcome** | One of nine terminal states, see below |
| **Tests** | `passed/total`, referring to acceptance checks, not the project's own test-case count |
| **Tokens** | Input tokens |
| **Time** | Elapsed time |

### Nine outcomes

| Outcome | Meaning |
| --- | --- |
| **Queued** / **Running** | Not yet terminal |
| **Succeeded** | All checks passed, and the repeated verification agreed |
| **Task failed** | The Agent finished running but didn't meet the acceptance bar |
| **Agent failed** | The Agent itself errored out — not that it got the task wrong |
| **Timed out** | Exceeded the task's time limit |
| **Stuck** | No progress |
| **Cancelled** | You cancelled it manually |
| **Benchmark error** | The evaluation framework itself failed — unrelated to the Agent |

**"Task failed" and "Agent failed" must be read separately**: the former means this Agent couldn't solve the task; the latter means it never got a fair run at all — the latter is not evidence about capability. **"Benchmark error" counts against the Agent even less.**

## The evidence panel

Selecting a row shows every piece of evidence for that attempt on the right:

| Section | Contents |
| --- | --- |
| **Identity** | Provider / model, and the **configuration fingerprint** |
| **Verification** | `PASS`/`FAIL` and a note for each check |
| **Bounded diff artifact** | Files this attempt changed (up to 20 listed) |
| **Metrics and provenance** | Each metric's value, unit, **quality**, and **source** |
| **Context and tool timeline** | Four event categories: lifecycle, tool calls, context, verification |

**The configuration fingerprint is a prerequisite for comparability.** Swap the model or change parameters for the same Agent and the fingerprint changes — comparing two attempts with different fingerprints is meaningless, which is why it sits in the most prominent spot.

### Metric quality: three tiers, don't mix them

Every metric is tagged with a quality level, and **this is a units declaration, not decoration**:

| Quality | Meaning |
| --- | --- |
| **reported** | A real value reported by the Agent itself |
| **estimated** | An estimate |
| **unavailable** | Not obtainable this time |

Costing an `estimated` token count produces a wrong conclusion. Check this column before deciding what the numbers mean.

## The judgment rules

Two hard rules decide whether the results table can be trusted:

**1. A disagreement on the repeat verification is a fail.** Evaluation reruns the checks once; if the two runs disagree, the attempt is marked flaky and **judged "task failed" outright, never "succeeded."** A lucky one-time pass doesn't count.

**2. The judge can never overturn a result.** Even with a model judge configured and giving a favorable assessment, **it cannot turn a deterministic check's failure, or a flaky result, into a success.** The judge's opinion is additional context only; the deterministic checks have the final word.

## Export

Every arena row has an **Export JSON** action at the end; the export carries the schema version and every attempt, check, metric, and timeline in the whole arena, for offline comparison or record-keeping.

## Notes and limits

- **Desktop only.**
- **Only the three built-in benchmarks can run** — no custom tasks, and no importing your own project as a benchmark.
- **The Agents that can enter are currently fixed to onepiece and codex-cli.**
- **An arena holds at most 8 attempts.**
- **The fixture copy has a ceiling**: over 2000 files or 32 MB fails.
- **Each attempt uses an independent copy directory**, isolated from the others and cleaned up after evaluation; **your real workspace is unaffected.**
- **Evaluation results only describe "performance on these three tasks."** They are not an overall capability rating for an Agent, and they cannot be extrapolated to your actual codebase.
- **Cancel only works on attempts that haven't reached a terminal state.**

## Related

- Activity bar and main-interface navigation → [User interface](user-interface.md)
- Getting multiple Agents to collaborate in one session, rather than compete → [Multi-Agent group chat](multi-agent-workflow.md)
- The full semantics of execution traces and logs → [Observability](observability.md)
- Token usage accounting → [Scheduled tasks and usage](automation.md)
- The evaluation methodology itself → [Multi-Agent systems technical architecture](../../../agent-infrastructure/multi-agent-architecture.md) (Simplified Chinese)
