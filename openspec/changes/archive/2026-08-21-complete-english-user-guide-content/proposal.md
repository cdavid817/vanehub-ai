## Why

`user-guide-documentation` requires English and Simplified Chinese guides with equivalent navigation, commands, runtime applicability, prerequisites, results, and troubleshooting coverage. It permits a partial English guide only "during a declared transition period recorded in an OpenSpec change", and states that outside such a period "the unconditional equivalence requirement applies in full".

That transition has been open across two changes. `rebuild-project-documentation-topology` opened it, rebuilding the English guide to the Chinese chapter topology and marking every untranslated chapter as a known gap. `complete-chinese-user-guide-coverage` extended it, adding six chapters to the authoritative Chinese set and six more known gaps to English.

The transition is doing its job — the gaps are declared, every English chapter resolves, and no reader hits a dead link. But it is a transition, not a destination, and it is now covering twenty of twenty-nine chapters:

| State | Count |
| --- | --- |
| Known-gap stub | 20 |
| Present but materially thinner than its Chinese counterpart | 5 |
| Equivalent | 3 |
| Navigation index, stale against the six new chapters | 1 |

The five thin chapters are the less visible problem. They are not marked as gaps, so they read as complete, while carrying between 10% and 25% of their Chinese counterpart's content — `multi-agent-workflow.md` covers 10% of a chapter that is the product's flagship workflow. A declared gap is honest; an undeclared shortfall is the silent divergence the spec forbids.

## What Changes

- The twenty known-gap chapters gain full English content rendered from their authoritative Chinese counterparts.
- The five thin chapters — `getting-started`, `first-session`, `multi-agent-workflow`, `runtime-labels`, `troubleshooting` — are brought to equivalence rather than left as undeclared shortfalls.
- `index.md` gains the six chapters added by `complete-chinese-user-guide-coverage` and its Status column is retired, because once every chapter is translated the column records only that the transition is over.
- The declared transition ends. No known-gap markers remain, and `user-guide-documentation`'s unconditional equivalence requirement applies in full from this change forward.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

None. This change fulfills `user-guide-documentation` as already specified — the equivalence requirement, the transition clause, and the coverage requirement are all unchanged. It ends a declared transition rather than altering the contract, so it carries no spec delta.

## Impact

**Runtime scope: neither.** Documentation only. No application code, no Tauri command, no frontend service, no runtime adapter, no SQLite migration, no spec edit.

Affected surfaces:

- `docs/user-guide/en/src/` — twenty chapters written, five expanded, `index.md` updated.

No Chinese chapter changes. `docs/user-guide/zh-CN/` remains the authoritative source and this change renders from it rather than editing it; a correction needed on the Chinese side is a finding to raise, not something to fix silently in translation.
