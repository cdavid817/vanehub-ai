## Why

Two findings were recorded rather than fixed while the user guides were being completed, on the explicit grounds that fixing them mid-translation would have left that change with no reviewable boundary. Both are now the only things standing between the guides and the audience boundary their specs draw.

**The user guide contains developer documentation.** `multi-agent-workflow.md` anchors nearly every claim to a file and line range (`seat_turn.rs:139-183`), embeds a `SessionSeat` Rust struct definition, explains why seats live in a JSON column rather than a join table, and closes with a fourteen-row index of code locations. `multi-agent-testing-tutorial.md` is a manual QA script: it runs `npm run lint:ci`, `cargo clippy`, and `npx playwright test`, and maps its checkpoints onto named Playwright specs.

`user-guide-documentation` scopes the guides to task-oriented user goals. `native-developer-documentation` requires the developer guide to be the single English architectural narrative and owns code pointers. So this content is not merely dense — it is in the wrong book, and it is in **both** language editions because the translation preserved it faithfully.

The developer guide already has a `multi-agent-group-chat.md`, but it is 1.6 KB of orientation that points at the spec. The implementation narrative that belongs there is sitting in the user guide instead.

**The user guide's English media set is one screenshot against the Chinese set's twenty.** `docs/user-guide/screenshots.json` declared one English scenario and twenty Chinese ones, so nineteen English chapters describe surfaces the English reader cannot see. The capture harness already supports both locales through a `text(locale, zh, en)` selector helper; nothing was missing but the scenario declarations.

A third, smaller defect rides along: the tutorial tells readers to run `openspec validate complete-multi-agent-session-presence --strict`, and that change was archived with PR #119, so the command fails.

## What Changes

- Implementation-level material moves from `multi-agent-workflow.md` into the developer guide's `multi-agent-group-chat.md`: the Rust struct, every source coordinate, the code-location index, and the rationale for storage and mirroring decisions. The user guide keeps the behavior a user observes — seats, handles, the handoff rules, `@用户` intents, the chain limits behind the `handoff 1/15` counter, and the model-family caveat.
- `multi-agent-testing-tutorial.md` keeps its walkthrough and checkpoints and loses its build-and-test apparatus: the `lint:ci` / `cargo` / `playwright` command blocks, the automated-spec mapping, and the stale `openspec validate` line. That apparatus moves to the developer guide alongside the design it verifies.
- Both language editions change together, so the guides stay equivalent.
- `docs/user-guide/screenshots.json` gains the nineteen missing English scenarios, and the English captures are generated. One Chinese-only selector in the capture harness is made bilingual so the English run can reach the same surface.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

None. This change moves existing content between two books, each already governed by its own spec, and fills a media set that `user-guide-documentation` already requires. No requirement text changes, so it carries no spec delta.

## Impact

**Runtime scope: neither.** Documentation, documentation fixtures, and one test-harness selector. No application code, no Tauri command, no frontend service, no runtime adapter, no SQLite migration.

Affected surfaces:

- `docs/user-guide/zh-CN/src/` and `docs/user-guide/en/src/` — two chapters each, reduced to their user-facing content.
- `docs/developer-guide/src/multi-agent-group-chat.md` — receives the migrated implementation narrative and verification apparatus.
- `docs/user-guide/screenshots.json` and `docs/user-guide/assets/screenshots/` — nineteen English scenarios declared and captured.
- `tests/docs/documentation-screenshots.spec.ts` — one selector made bilingual.

**No behavior claim is dropped in the move.** Anything true of the product that the user guide stated remains stated in one of the two books; what leaves the user guide is where the code lives and how to run the repository's checks.
