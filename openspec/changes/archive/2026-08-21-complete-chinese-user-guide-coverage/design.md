## Context

`docs/user-guide/zh-CN/` is the authoritative complete set established by `rebuild-project-documentation-topology`. Its 22 chapters are structurally sound and substantively written; the defect is that twelve delivered capabilities have no coverage or only incidental mentions. See `proposal.md` for the audit.

The enforcement surface is unchanged and already strong for what it measures: `npm run docs:check` covers README parity, link/media validity, and the bold-punctuation rule that CommonMark flanking makes a real rendering hazard in Chinese text. None of it can detect a missing subject.

Two source-of-truth constraints shape how the chapters are written. `openspec/specs/<capability>/spec.md` establishes behavior; `src/i18n/locales/zh-CN.json` establishes the strings a reader will actually see. Neither alone is sufficient — the spec does not know what the button says, and the string table does not know what happens when you press it.

## Goals / Non-Goals

**Goals:**

- Close the coverage gap for every delivered user-facing capability found by the audit.
- Keep the English guide's declared transition honest as the Chinese guide grows.
- Make the audit repeatable by writing the coverage obligation into the spec.

**Non-Goals:**

- Writing English content. The English guide gains known-gap chapters only; content remains the deferred follow-up.
- Documenting `agent-evaluation`. See D2.
- Documenting capabilities whose specs live in `openspec/changes/` rather than `openspec/specs/`. Five `add-skill-evolution-*` changes are proposals, not delivered behavior; only `skill-evolution-evidence` is implemented and only it is documented.
- Adding a mechanical coverage validator. See D5.
- Screenshots for the new chapters. The existing chapters carry screenshots from the deterministic CI browser environment; adding them is a separate pass against `docs/user-guide/screenshots.json`.

## Decisions

### D1: Standalone chapter versus fold-in, decided by subject ownership

**Choice:** A capability gets a standalone chapter when no existing chapter owns its subject, and gets folded in when one does.

Six capabilities had no owner: goals, the board, slash commands, code review, memory/context, and updates. Four had a clear one:

| Capability | Host chapter | Why that chapter owns it |
| --- | --- | --- |
| `agent-notebook-editing` | `native-agent.md` | It is an OnePiece tool capability, and its spec binds it to the workspace boundary and Plan mode — both already established there |
| File references | `first-session.md` | It is part of composing a message, which that chapter already walks through |
| `session-recovery` | `troubleshooting.md` | It surfaces to users as a symptom after an unclean shutdown |
| `skill-evolution-evidence` | `skill-management.md` | It is a panel inside Skill detail, alongside Overlay |

**Why not a chapter each:** four thin chapters covering subjects their neighbours already establish would force every reader to reconstruct context that the host chapter states once. The board and goals are genuinely separate destinations; notebook editing is a tool inside an Agent the reader is already reading about.

**Why memory and context compaction share one chapter:** they are separate capabilities but a single user concern — what the model carries between and within sessions. Splitting them would duplicate the enablement-toggle and Web-parity discussion in both.

### D2: `agent-evaluation` is excluded and the exclusion is recorded

**Choice:** The evaluation and benchmark platform is not documented in the user guide, and this decision is the record the new spec requirement demands.

Its audience is a contributor assessing Agent quality against benchmarks, not a user performing a task. `user-guide-documentation` scopes the guides to task-oriented user goals; `native-developer-documentation` owns the English architectural and contributor narrative. Documenting it here would put contributor tooling in front of users and blur the audience boundary the two specs draw between them.

**Alternative considered:** cover it briefly for completeness. Rejected — "brief coverage of a contributor tool" is how a user guide starts becoming a feature list.

### D3: Facts come from specs and the string table, not from reading behavior out of components

**Choice:** Behavior claims trace to `openspec/specs/<capability>/spec.md`; every rendered label traces to `src/i18n/locales/zh-CN.json`.

This caught real errors that guessing would have introduced. The board's rendered name is 任务看板, not the 待办看板 its `unified-todo-board` capability id suggests. Code review is the 变更 tab inside a session, not a standalone centre. Goals render as 目标中心 in the activity bar but 目标 as the page title.

Where a claim was load-bearing and the spec was ambiguous, the implementation settled it: `participatesInDerivation` in `src/services/web-goal-progress.ts` establishes that Session **and Run** links are excluded from goal acceptance derivation, and `MAX_FILE_REFERENCES` is 5 in both `src/types/chat.ts` and `src-tauri/src/contexts/sessions/domain/message.rs`.

**Why this matters beyond accuracy:** a guide that names a control the application does not render is worse than no guide, because the reader concludes the feature is missing rather than that the guide is wrong.

### D4: English gap stubs extend the declared transition rather than deferring the navigation

**Choice:** Every new Chinese chapter ships with its English known-gap counterpart and both `SUMMARY.md` entries in the same change.

The transition clause permits a partial English guide only while every missing chapter is explicitly marked. Adding six Chinese chapters without English entries would convert six declared gaps into six silent omissions — moving from the compliant state to the exact violation the previous change was raised to fix.

### D5: A spec requirement, not a validator

**Choice:** Enforce coverage through a reviewable spec requirement rather than a check in `scripts/validate-docs.mjs`.

A mechanical check would need to know which capabilities are user-facing. That judgement is not derivable from `openspec/specs/` — nothing there distinguishes `goal-management` from `contract-and-task-foundation`. A keyword check would be worse than nothing: it would pass on an incidental mention, which is precisely the state four of these capabilities were already in.

The requirement instead makes the omission *stateable*: a capability is covered, or its exclusion is recorded, and an unstated gap is a violation. That is enforceable by review at the point where it is cheapest — when a capability is archived.

**Alternative considered:** require a `user-guide` coverage field in each capability's spec. Rejected as a change to the OpenSpec capability catalog, which the previous change explicitly scoped out and this one has no better claim to.

## Risks / Trade-offs

- **[The coverage requirement has no mechanical enforcement]** → accepted per D5; the mitigation is that the obligation is now stateable and reviewable, where before it was invisible.
- **[Chapters describe behavior the reader cannot verify in Web/mock]** → every new chapter carries a runtime label naming what is real in each runtime, and the memory, update, and review chapters state explicitly that Web/mock writes are simulated.
- **[Folding four capabilities into existing chapters grows those chapters]** → `native-agent.md`, `first-session.md`, `troubleshooting.md`, and `skill-management.md` each gained one section; none approaches a size where splitting would help, and each section sits under the context it depends on.
- **[Six more English gap stubs make the English guide look emptier]** → it is emptier, and the transition clause exists so that this is declared rather than concealed. The alternative is not a fuller English guide, it is a dishonest navigation.
- **[Documented behavior drifts when a capability changes]** → the same exposure every chapter already carries; the coverage requirement narrows it by making a *new* capability's absence detectable, not by preventing drift in an existing one.
