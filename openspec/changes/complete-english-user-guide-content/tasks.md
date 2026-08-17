Each chapter task means: render the authoritative Chinese chapter into English per design D1/D2, preserving its section structure, tables, runtime label, and feature-state label, with identifiers reproduced verbatim and UI labels taken from `src/i18n/locales/en.json`.

## 1. Entry path

- [x] 1.1 `quick-start.md`
- [x] 1.2 `core-concepts.md`
- [x] 1.3 `user-interface.md`

## 2. Getting to a first session

- [x] 2.1 `getting-started.md` — expand from 25% to equivalence.
- [x] 2.2 `first-session.md` — expand from 15% to equivalence, including the file-reference section.

## 3. Workflow chapters

- [x] 3.1 `multi-agent-workflow.md` — expand from 10% to equivalence; the largest single gap.
- [x] 3.2 `multi-agent-testing-tutorial.md`
- [x] 3.3 `loop-engineering.md`
- [x] 3.4 `goal-management.md`
- [x] 3.5 `todo-board.md`
- [x] 3.6 `slash-commands.md`
- [x] 3.7 `code-review.md`
- [x] 3.8 `memory-and-context.md`
- [x] 3.9 `permissions.md`
- [x] 3.10 `personalization.md`

## 4. Capability chapters

- [x] 4.1 `skill-management.md` — add the evolution-evidence section that the Chinese chapter gained; the rest is already equivalent.
- [x] 4.2 `tooling.md`
- [x] 4.3 `native-agent.md`
- [x] 4.4 `observability.md`
- [x] 4.5 `remote-and-im.md`
- [x] 4.6 `automation.md`
- [x] 4.7 `app-updates.md`

## 5. Reference

- [x] 5.1 `use-cases.md`
- [x] 5.2 `faq.md`
- [x] 5.3 `runtime-labels.md` — expand from 18% to equivalence.
- [x] 5.4 `troubleshooting.md` — expand from 12% to equivalence, including the session-recovery section.

## 6. Close the transition

- [x] 6.1 Rebuild `index.md` on both sides: the Chinese index was missing eight chapters and `user-interface.md`, and claimed the English guide was partial; both now list all 28 chapters and the Status column is retired per design D5.
- [x] 6.2 Confirm no `Known gap` marker survives anywhere under `docs/user-guide/en/` — zero remain.
- [x] 6.3 Confirm both `SUMMARY.md` files still match in chapter set and order — 29 chapters, identical order.

## 7. Verification

- [x] 7.1 `npm run docs:check` passes.
- [x] 7.2 `npm run docs:test` passes.
- [x] 7.3 `npm run docs:build` produces both user books with no known-gap content.
- [x] 7.4 `openspec validate "complete-english-user-guide-content" --strict` passes; `openspec validate --specs --strict` reports 133 passed, 0 failed.
- [x] 7.5 Compare section headings chapter by chapter between the two guides — 29/29 match, with English at 96–124% of the Chinese character count.
- [x] 7.6 Report any Chinese-source defect found while translating, per design D4 — eight were found, reported, and fixed on both sides.
