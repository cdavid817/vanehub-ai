## Why

The documentation topology has drifted into three overlapping books plus orphaned reference files, and the two documentation specs that were supposed to govern it never had their `Purpose` filled in after the archive that created them. Concrete violations today:

- The English user guide has 9 chapters; the Simplified Chinese user guide has 22. `user-guide-documentation` requires "equivalent navigation, commands, ... coverage" between the two — the gap is not a gap, it is an active spec violation.
- `docs/zh/` is a third book (overview + 20 architecture chapters + 3 development chapters) that overlaps both the developer guide and the Chinese user guide, covers only Chinese, and is referenced from no `README` documentation section. It contradicts the same equivalence requirement.
- `openspec/specs/native-developer-documentation/spec.md` and `openspec/specs/user-guide-documentation/spec.md` both still read `Purpose: TBD - created by archiving change establish-multilingual-documentation. Update Purpose after archive.` The archive completed; the update did not.
- `docs/architecture/` holds `cli-chat-runtime-v1.md` (superseded by multi-agent group chat) and `workspace-modularization-follow-up.md`, whose relationship to `src-tauri/ARCHITECTURE.md` (the spec-designated ADR source of truth) is unstated.
- `docs/superpowers/` (dated `plans/` and `specs/` working artifacts) sits inside the published `docs/` tree.
- `README.md` advertises delivered Japanese UI, but the user-guide spec scopes guides to EN and ZH-CN only, with no Japanese guide — the boundary between "UI locale" and "user guide locale" is never stated, so the gap reads as a broken promise.

The rebuild collapses the topology to a single, spec-governed structure and closes the equivalence violation through a deliberately staged Chinese-as-authoritative-source policy, rather than leaving both books in a half-aligned state.

## What Changes

- The Chinese user guide (`docs/user-guide/zh-CN/`) is declared the authoritative complete set; the English user guide (`docs/user-guide/en/`) is rebuilt to reach the same chapter topology and is allowed to carry an explicit partial/known-gap state until a follow-up change completes its content.
- The `user-guide-documentation` spec gains a staged-equivalence requirement: during a declared transition the English guide MAY be partial, but it MUST mark every missing chapter explicitly and MUST NOT silently diverge in navigation, runtime labeling, or truthful feature-state labeling. The unconditional equivalence requirement survives for the steady state.
- `docs/zh/` is removed after its architecture and development content is folded into the developer guide (English) and the Chinese user guide. Its removal is a **BREAKING** documentation-structure change for any reader or link targeting that path; the affected chapters are migrated, not dropped.
- `docs/architecture/` superseded entries are either folded into `src-tauri/ARCHITECTURE.md` (where they record a decision), relabeled as historical ADRs, or removed. The ADR source of truth stays `src-tauri/ARCHITECTURE.md`.
- `docs/superpowers/` is moved out of the published `docs/` tree into a non-published working-artifacts location.
- The Japanese locale boundary is made explicit: Japanese is an application UI resource locale only; user guides remain scoped to EN and ZH-CN. README localization claims are aligned to that boundary.
- The `Purpose` sections of `native-developer-documentation` and `user-guide-documentation` are written for the first time.
- Root documentation entry points (`AGENTS.md`, `README.md`/`zh-CN`/`ja`, `CONTRIBUTING.md`) are reconciled so their "Documentation" sections point at the collapsed topology and no stale links to `docs/zh/` survive.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `user-guide-documentation`: gains a staged-equivalence requirement that lets the English guide be explicitly partial during a declared transition while keeping unconditional equivalence as the steady-state requirement; gains an explicit statement that the user-guide locale set is EN and ZH-CN and that application UI locales (e.g. Japanese) are not user-guide locales.
- `native-developer-documentation`: gains a requirement that the developer guide is the single English architectural narrative and that historical/ADR-style content lives in `src-tauri/ARCHITECTURE.md` or a clearly labeled historical section, not in a competing `docs/architecture/` narrative; the spec's `Purpose` is fixed from TBD.

## Impact

**Runtime scope: neither.** This is a documentation- and spec-only change. No application code, no Tauri command, no frontend service, no runtime adapter, no SQLite migration. The frontend/backend isolation boundary and the runtime adapter boundaries are untouched.

Affected surfaces:

- `docs/` tree — `docs/zh/` removed; `docs/architecture/` reconciled; `docs/superpowers/` relocated; `docs/user-guide/en/` chapter topology rebuilt; `docs/user-guide/zh-CN/` confirmed as authoritative.
- `openspec/specs/user-guide-documentation/spec.md` and `openspec/specs/native-developer-documentation/spec.md` — `Purpose` written, requirements amended.
- Root docs — `AGENTS.md`, `README.md`, `README.zh-CN.md`, `README.ja.md`, `CONTRIBUTING.md` link and prose reconciliation.
- Validation scripts — `scripts/validate-docs.mjs`, `scripts/check-readme-parity.mjs` extended to cover the collapsed topology (no `docs/zh/` references, parity across the surviving README localization set, link coverage for any migrated chapters).
- `src-tauri/ARCHITECTURE.md` — receives migrated ADR-style content from `docs/architecture/` where applicable.

**BREAKING for documentation structure:** any external link or reader workflow targeting `docs/zh/` will break. Within the repository the migration replaces those targets; external references cannot be migrated and are an accepted consequence of removing a redundant book.
