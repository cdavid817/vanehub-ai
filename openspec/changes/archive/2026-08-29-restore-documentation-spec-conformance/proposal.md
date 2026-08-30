## Why

A documentation audit run against `33356058` found that `npm run docs:check` and `openspec validate --specs --strict` both pass while the repository contradicts its own documentation specifications in four independent ways. Every finding below survives the existing gates, so none of them is visible to CI.

**The two user guides diverged on purpose and the specification was never told.** PR #178 (`26e66038`, 2026-08-19) removed the `**状态:已实现——**` line from every Simplified Chinese chapter, removed all Web/mock browser-preview content, and deleted `runtime-labels.md` — 31 files, 204 deletions. The English guide kept all of it: 23 of its 36 chapters still open with a `**Status: Implemented — ...**` line. `user-guide-documentation` still requires that the English guide "SHALL NOT silently diverge from the Simplified Chinese guide in navigation structure, runtime labeling, or truthful feature-state labeling", and still requires that every workflow "identify whether it is delivered, preview, Web/mock-only, desktop-only, or planned". The specification and the repository now demand opposite things, and the declared transition period that once excused a partial English guide ended when `2026-08-21-complete-english-user-guide-content` was archived.

**A published, first-class deliverable has no governing requirement.** `scripts/build-docs.mjs:22` builds `docs/developer-guide/zh-CN` into `developer/zh-CN`, and the documentation landing page at `scripts/build-docs.mjs:119` links it as 开发者指南 — 简体中文. `native-developer-documentation` describes "a single English mdBook developer guide" that "SHALL be the single English architectural narrative for the project", and no requirement anywhere in `openspec/specs/` mentions `developer-guide/zh-CN`. Forty published Chinese chapters are ungoverned.

**The English developer guide is a fraction of the Chinese one.** Sixteen chapters hold between 18% and 33% of their Chinese counterpart's content, and the gap is structural rather than stylistic: `关键类型与常量` appears in six Chinese chapters and in no English one, and `repository-orientation.md` carries five Chinese `##` sections — including the seven bounded contexts and the request path through the layers — against zero in English. Those English chapters have been frozen since 2026-08-17 while the Chinese ones kept moving.

**A requirement exists with no gate behind it.** `native-developer-documentation` states that an unreferenced document under `docs/` "SHALL be treated as a defect, not as archived material". `scripts/validate-docs.mjs` contains no reachability logic at all, and 25 of the 196 committed Markdown files under `docs/` are reachable from no navigation, no README, and no other document — including all 14 `agent-platform-roadmap/` files, whose own `00-START-HERE-ROADMAP.md` contains no Markdown link. The requirement was satisfied once by hand in `2026-08-21-fix-guide-link-targets` and has been silently regressing since.

## What Changes

- Remove the per-chapter status line and every Web/mock browser-preview section, sentence, and cross-reference from the English user guide, and delete `docs/user-guide/en/src/runtime-labels.md`, matching the decision already applied to the Simplified Chinese guide. Both guides then describe the desktop application a reader installs, and neither is subordinate to the other.
- Recognize the Simplified Chinese developer guide as a governed deliverable, and require the two developer guides to carry the same chapters and the same section structure.
- Rebuild the sixteen thin English developer-guide chapters to the section structure their Chinese counterparts already carry, including the key-types-and-constants sections and the bounded-context inventory that English lacks entirely.
- Add a reachability check to `scripts/validate-docs.mjs` so an unreferenced document under `docs/` fails `npm run docs:check` instead of accumulating silently, and resolve the 25 existing offenders: reference material that carries long-term value is linked from the developer guide, and dated working artifacts are removed from the published tree.
- Move `docs/architecture/skill-tool-runtime-security.md` into the developer guide, retiring the `docs/architecture/` directory that the specification already forbids.
- Correct `CONTRIBUTING.md`, which quotes `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check` as "what CI enforces" while `AGENTS.md` requires the `--workspace` and `--manifest-path` forms — the weaker variants the same sentence warns against.
- Expand both user guides and both developer guides with additional screenshots and diagrams, and add the screenshot scenarios that produce them deterministically.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `user-guide-documentation`: Replace the authoritative-plus-transition model with two complete, equivalent guides; remove the per-chapter runtime and feature-state labeling requirement and the Web/mock runtime-path requirement; extend the coverage requirement to both guides symmetrically.
- `native-developer-documentation`: Govern the developer guide as a bilingual deliverable with equivalent chapters and sections; require reachability to be enforced by an automated check rather than by review.

## Impact

- Affects `docs/user-guide/en/src/` (36 chapters plus `SUMMARY.md`), `docs/developer-guide/src/` (16 rebuilt chapters plus `index.md` and `SUMMARY.md`), `docs/developer-guide/zh-CN/src/` (`index.md` and `SUMMARY.md`), and `CONTRIBUTING.md`.
- Removes `docs/agent-platform-roadmap/` (14 chapters and a delivery manifest), `docs/reports/` (2 dated verification reports), `docs/ux-audit-report.md`, `docs/ux-optimization-summary.md`, and `docs/architecture/`. All remain recoverable from Git history; none is referenced by any live document.
- Extends `scripts/validate-docs.mjs` with a reachability pass and `scripts/validate-docs.node-test.mjs` with its unit coverage, so `npm run docs:check` gains a gate it did not have.
- Extends `tests/docs/documentation-screenshots.spec.ts` with new capture scenarios and `docs/user-guide/screenshots.json` with their inventory entries; new PNG assets are generated by `npm run docs:screenshots:update` and verified by `npm run docs:screenshots:check`.
- No frontend, native, or database change. No new runtime dependency. The documentation build entry point and its pinned tooling are unchanged.
