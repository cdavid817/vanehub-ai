## Context

The repository currently carries four mdBook-shaped doc surfaces (`docs/developer-guide`, `docs/user-guide/en`, `docs/user-guide/zh-CN`, `docs/zh`) plus an orphaned `docs/architecture/` directory and dated working artifacts under `docs/superpowers/`. Root entry points (`AGENTS.md`, three README localizations, `CONTRIBUTING.md`) link into these inconsistently. The two governing specs (`user-guide-documentation`, `native-developer-documentation`) were created by archiving `establish-multilingual-documentation` but their `Purpose` was left as a `TBD` placeholder and never filled in. See `proposal.md` for the full motivation and the concrete violations.

Existing validation infrastructure is intact and is the enforcement surface this change leans on:

- `npm run docs:check` → `docs:unit:test` (readme-parity, screenshot-port, validate-docs unit tests) + `docs:readme:check` + `docs:links:check` (`scripts/validate-docs.mjs`)
- `npm run docs:test` / `docs:build` → `scripts/test-docs.mjs` / `scripts/build-docs.mjs`, mdBook pinned at `docs/toolchain.json` = `0.5.4`
- `docs/developer-guide/native-boundaries.json` drives the native-boundary documentation inventory check

There is no application code, no runtime adapter, and no SQLite change in this work. The "runtime boundary" is intentionally untouched: no React component, no `tauri-agent-client`/`web-agent-client` adapter, no Tauri command is in scope. The only Rust-adjacent file touched is `src-tauri/ARCHITECTURE.md`, a Markdown ADR document, not Rust source.

## Goals / Non-Goals

**Goals:**

- Collapse the documentation topology to one English developer guide plus one English and one Simplified-Chinese user guide, with the Chinese user guide as the authoritative complete set.
- Remove `docs/zh/` after migrating its still-valuable content.
- Make the English user guide's incompleteness an *explicit, declared* gap rather than a silent divergence, governed by the spec's staged-equivalence requirement.
- Fill the two `TBD` `Purpose` sections and fix the `docs/architecture/` and `docs/superpowers/` misplacements.
- Extend the validation scripts so the collapsed topology is enforced going forward (no `docs/zh/` references survive; README parity covers the surviving localization set; migrated chapters are link-covered).

**Non-Goals:**

- Writing the missing English user-guide chapter *content*. The English guide is rebuilt to the right chapter topology and marks gaps; filling content is a follow-up change. This keeps the review bounded and lets the spec's transition clause be the contract for the gap.
- Translating the developer guide to Chinese. The developer guide stays English-only (its spec scopes it to English).
- Adding a Japanese user guide. Japanese remains an application UI locale only.
- Touching the OpenSpec `specs/` capability catalog or the archive governance. Only the two documentation specs are modified.
- Any change to `docs/toolchain.json` mdBook pin or to the screenshot/fixture infrastructure beyond link/path reconciliation.

## Decisions

### D1: Chinese user guide is the authoritative complete set; English rebuilt to matching topology, content deferred

**Choice:** Declare `docs/user-guide/zh-CN/` (22 chapters) the authoritative complete set. Rebuild `docs/user-guide/en/SUMMARY.md` to mirror the ZH-CN chapter topology, and for chapters whose English content does not yet exist, create the chapter file with a single explicit "known-gap" notice rather than leaving a 404 or a silent stub.

**Why not "expand English to 22 chapters now":** content authoring is the long pole and would make this change unbounded and unreviewable. Topology alignment + explicit gap markers is what closes the *spec violation* (silent divergence); it makes the gap honest and reviewable. The spec's staged-equivalence requirement is written so this is a compliant steady state of a declared transition, not a lingering violation.

**Why not "trim Chinese to 9 chapters to match English":** the Chinese guide is the more complete book; trimming it throws away reviewed content. The user chose Chinese-as-authoritative explicitly.

**Alternative considered:** rewrite the English requirement to drop equivalence entirely. Rejected — equivalence is the right steady state and is preserved in the spec; only a declared transition is permitted.

### D2: Migrate `docs/zh/` content, then delete the book

**Choice:** Walk `docs/zh/src/02-architecture/*` and `docs/zh/src/03-development/*`. For each chapter:

- if it is architectural/ADR material → fold into `src-tauri/ARCHITECTURE.md` or a new "Historical architecture notes" section of the developer guide, deduplicating against existing content;
- if it is user-facing task material → verify it already has a counterpart in `docs/user-guide/zh-CN/` (most does) and drop the duplicate;
- if it is unique developer-onboarding material → fold into `docs/developer-guide/src/`.

Then delete `docs/zh/` entirely (book.toml, src/, SUMMARY).

**Why delete rather than keep as a redirect:** a third Chinese book that overlaps two others is the root cause of the drift; keeping it as a redirect still leaves a competing narrative the link validator must carry. A clean removal, enforced by the updated validator, is what prevents recurrence.

**Alternative considered:** keep `docs/zh/` but mark "deprecated" in its README. Rejected — the link/parity validators would still have to treat it as a live book, and the deprecation would rot the same way the `TBD` Purpose did.

### D3: `docs/architecture/` reconciliation

**Choice per file:**

- `cli-chat-runtime-v1.md` → superseded by multi-agent group chat; fold any decision-level content into `src-tauri/ARCHITECTURE.md`, then remove the file. Do not keep a competing v1 narrative in `docs/`.
- `workspace-modularization-follow-up.md` → if it records a completed decision, migrate the decision to `src-tauri/ARCHITECTURE.md` and remove; if it is a status note, remove outright.
- `agent-execution-observability.md`, `im-connectors-smoke.md`, `type-contracts.md` → evaluate each: move surviving content into the developer guide's relevant chapter or `src-tauri/ARCHITECTURE.md`; remove what is stale.

The goal is that `docs/architecture/` either no longer exists or contains only clearly-labeled historical references that the developer guide explicitly points at — never a competing narrative.

**Alternative considered:** keep `docs/architecture/` and link it from the developer guide as a reference dir. Rejected — it already overlaps the developer guide's `native-contexts.md` and `runtime-boundaries.md` chapters, and the v1 file actively misleads.

### D4: Relocate `docs/superpowers/`

**Choice:** Move `docs/superpowers/` to a top-level `.superpowers/` (or `working-artifacts/`) directory outside the published `docs/` tree. The link validator (`scripts/validate-docs.mjs`) is scoped to `docs/` and will naturally stop covering it.

**Why a new top-level dir rather than `openspec/changes/`:** those artifacts are dated plans/specs, not OpenSpec change artifacts; putting them under `openspec/changes/` would make `openspec validate` try to parse them. A plain non-published directory keeps them as working notes.

**Alternative considered:** delete outright. Rejected until the user confirms none of the four plans (multi-agent manual QA, turn-coordinator handoff, onepiece tool catalog, vector search phase 1) is still referenced; relocation is reversible, deletion is not.

### D5: README / entry-point reconciliation

**Choice:** Update the "Documentation" section of `README.md`, `README.zh-CN.md`, `README.ja.md`, and `CONTRIBUTING.md` so every link targets the collapsed topology and no link points at `docs/zh/`. For the Japanese README, state explicitly that user guides exist in EN/ZH-CN and Japanese is an application UI locale only — making the boundary in D/spec the reader-facing truth.

`scripts/check-readme-parity.mjs` is extended to assert no README references `docs/zh/`, and to cover the surviving README localization set (EN, ZH-CN, JA) for structural parity of the Documentation section.

### D6: Spec `Purpose` fix is a direct main-spec edit during apply

Per OpenSpec's delta rules, a delta for an existing capability must not carry a `## Purpose` block; fixing the leftover `TBD` placeholder is done by editing `openspec/specs/user-guide-documentation/spec.md` and `openspec/specs/native-developer-documentation/spec.md` directly during the apply phase. The requirement *changes* travel in this change's delta files; the `Purpose` rewrite is a paired direct edit done in the same apply session and noted in `tasks.md`.

## Risks / Trade-offs

- **[External links to `docs/zh/` break]** → accepted, documented as BREAKING in the proposal; no in-repo migration target is left dangling because the validator forbids `docs/zh/` references.
- **[English user guide ships with explicit gap markers, which looks incomplete]** → that is the point: an honest declared gap is spec-compliant under the transition clause, where a silent gap was a violation. The gap markers link to the ZH-CN chapter so a reader is never blocked.
- **[Migration of `docs/architecture/` content into `src-tauri/ARCHITECTURE.md` could bloat that file]** → mitigate by migrating only decision-level content, not narrative; status notes are dropped, not relocated.
- **[A migrated chapter referenced by a screenshot fixture changes the screenshot contract]** → `docs/user-guide/screenshots.json` and `docs-screenshot-port` checks are re-run; screenshot scenarios tied to removed chapters are updated in the same change.
- **[Validator changes could hide a future legitimate `docs/zh/` reintroduction]** → the validator asserts the *current* intended topology; a future reintroduction would be a deliberate spec change and would update the validator in the same proposal.
