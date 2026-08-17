## Context

The Chinese guide is the authoritative complete set: 29 chapters, ~140k characters, every claim traced to `openspec/specs/<capability>/spec.md` for behavior and `src/i18n/locales/zh-CN.json` for rendered labels. The English guide mirrors its topology but carries content for only three chapters at equivalence.

Enforcement is unchanged. `npm run docs:check` covers README parity, link and media validity, and the bold-punctuation rule; `npm run docs:test` runs mdBook navigation over all three books. None of it measures translation fidelity, so equivalence here is a review property, not a checked one.

See `proposal.md` for the gap counts and why the five thin chapters are the more urgent half.

## Goals / Non-Goals

**Goals:**

- Give every English chapter content equivalent to its Chinese counterpart in task coverage, runtime labeling, and feature-state labeling.
- End the declared transition so the unconditional equivalence requirement applies again.
- Keep technical identifiers exact so an English reader can act on the text without cross-checking the Chinese.

**Non-Goals:**

- Editing the Chinese guide. It is the source; a defect found while translating is raised, not patched in the target. See D4.
- Adding screenshots to the newly written chapters. The Chinese chapters carry captures from the deterministic CI environment; the English equivalents are a separate pass against `docs/user-guide/screenshots.json`.
- Adding a translation-fidelity validator. Equivalence of prose is not mechanically checkable, and a check that only compared file sizes would pass on padding.
- Translating the developer guide. Its spec scopes it to English only; it has no Chinese counterpart to translate from or to.

## Decisions

### D1: Render from the Chinese source, do not re-author from the specs

**Choice:** Each English chapter is produced from its Chinese counterpart, preserving its section structure, its tables, and the specific gotchas it chose to surface.

**Why not re-author from `openspec/specs/`:** the Chinese chapters already encode editorial judgement that the specs do not — which of a capability's twenty requirements a user actually trips over, which two behaviors are counterintuitive enough to need a callout, what to put in a table versus prose. Re-authoring from specs would discard that judgement and produce two guides that drift apart immediately, which is exactly the state this transition exists to end.

**Consequence:** structural equivalence is reviewable. A reviewer can diff section headings between the two files and see whether anything was dropped.

### D2: Translate prose, preserve identifiers exactly

**Choice:** Prose, headings, table headers, and callouts are rendered into natural English. These are reproduced verbatim: command names (`/mode`, `/longcontext`), file and directory paths, configuration keys, stable Agent ids (`claude-code`, `codex-cli`), constants, npm scripts, and version numbers.

Rendered UI labels are a third category: they are given in the English the application actually renders, taken from `src/i18n/locales/en.json` — not translated freehand from the Chinese label. The activity-bar entry the Chinese guide calls 任务看板 is `Todo Board` in the English UI, and 目标中心 is `Goal Center` while the page title is `Goals`. A guide that invents a plausible English label for a control describes a product that does not exist.

**Why this matters more than usual here:** the Chinese guide's value is largely in precise, checkable claims — that `MAX_FILE_REFERENCES` is 5, that Plan failure is not terminal while Loop failure is. A translation that softens those into approximations keeps the word count and loses the point.

### D3: Order by reader dependency, not by file size

**Choice:** Translate in the order a reader meets the chapters — entry path first (`index`, `quick-start`, `core-concepts`, `user-interface`), then the workflow chapters they depend on, then reference material last.

**Why:** cross-references resolve as they are written, and a partially complete pass leaves the guide usable from the front rather than usable in scattered patches. It also front-loads the chapters most likely to be read by someone evaluating the product.

### D4: A defect found in the Chinese source is raised, not silently corrected

**Choice:** If translating surfaces an error in the Chinese chapter — a stale label, a claim the spec does not support — the English chapter is not quietly written "correctly". The finding is reported so both sides are fixed together.

**Why:** silently diverging to fix a bug reintroduces divergence, and leaves the authoritative set wrong. The two guides being wrong in the same way is a smaller problem than them disagreeing, because only the second one is invisible.

### D5: Retire `index.md`'s Status column rather than fill it with "Translated"

**Choice:** Once no chapter is a known gap, the Status column is removed rather than set to `Translated` for all 29 rows.

A column whose every value is identical carries no information, and leaving it invites the next contributor to add a row without one. Its replacement is the "What it covers" column, which stays useful after the transition ends. The known-gap explanation line below the table goes with it.

### D6: `multi-agent-workflow.md` keeps its developer-guide shape in both languages

**Choice:** Translate the chapter faithfully, including its source-code coordinates, its embedded Rust struct, and its 14-row code-location index, and raise the audience mismatch as a separate finding rather than restructuring it here.

The chapter is written as developer documentation sitting in the user guide: nearly every claim is anchored to a file and line range (`seat_turn.rs:139-183`), it embeds a `SessionSeat` struct definition, and it explains implementation decisions such as why seats live in a JSON column rather than a join table. `user-guide-documentation` scopes the guides to task-oriented user goals, and `native-developer-documentation` owns the architectural narrative and code pointers — so this content is in the wrong book.

**Why not fix it here:** splitting it means deciding what belongs to each book, editing both the Chinese and English user guides, and adding the migrated material to the developer guide. That is a documentation-topology change, and doing it midway through a translation pass would leave this change with no reviewable boundary — the same unbounded-scope failure that `rebuild-project-documentation-topology` avoided by deferring content authoring in the first place.

**Consequence, stated rather than hidden:** completing this change leaves both language versions carrying the same misplacement. That is the deliberate trade-off — the two guides agree, and the misplacement is now recorded as a known finding instead of an unexamined habit.

## Risks / Trade-offs

- **[Translation fidelity is not mechanically enforced]** → accepted; D1's structural preservation makes heading-level equivalence reviewable by diff, which is the part most likely to hide an omission.
- **[The two guides drift as capabilities change]** → unchanged by this work and not solvable by it; the coverage requirement added by `complete-chinese-user-guide-coverage` narrows the case where a *new* capability goes undocumented.
- **[Rendering 20 chapters in one change is large to review]** → mitigated by D3's ordering and by chapter-level task granularity, so review can proceed per chapter rather than per change.
- **[English UI labels may themselves be stale or missing in `en.json`]** → a missing English label is a product defect, reported per D4 rather than papered over with a translated guess.
- **[Ending the transition removes the safety net]** → intended. The clause remains in the spec for a future declared transition; what ends is this one.
