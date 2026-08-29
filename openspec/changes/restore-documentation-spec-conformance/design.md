## Context

Four documentation defects were found by auditing the repository against its own specifications at `33356058`. All four survive `npm run docs:check` and `openspec validate --specs --strict`, so the design question is not only what to change but why the existing gates did not object.

Two of the four are specification drift: a deliberate product decision (PR #178) and a shipped deliverable (the Simplified Chinese developer guide) that were never written back into `openspec/specs/`. One is content debt: sixteen English developer-guide chapters frozen at a fraction of their Chinese counterparts. One is a missing gate: a requirement about reachability with no check behind it.

## Goals / Non-Goals

**Goals**

- Make the specifications describe the repository that exists, in the direction the product owner already chose.
- Give the reachability requirement a gate, so the class of defect cannot silently return.
- Bring the English developer guide to the section structure the Chinese one already carries.
- Leave `docs/` containing only material a reader can reach.

**Non-Goals**

- Rewriting the Chinese guides. They are the fuller and more current side of both books; this change moves English toward them, not the reverse.
- Changing the documentation build entry point, its pinned tooling, or the assembled output layout.
- Introducing a translation-completeness metric. Section-structure equivalence is the enforceable property; sentence-level parity is not.

## Decisions

### The user guides reconcile toward Simplified Chinese

PR #178 removed the status lines and Web/mock content from the Chinese guide at the product owner's explicit request. The English guide kept both. Reconciling toward English would reverse a decision that was made deliberately; reconciling toward Chinese preserves it. The specification is amended to match, rather than the repository being reverted to match the specification.

What is removed from English is the chapter-level `**Status: Implemented — ...**` line, the dedicated Web/mock sections, the Web/mock sentences and FAQ entries, and `runtime-labels.md`, which existed only to explain the labels being removed. Environment dependencies a reader can act on — an authenticated CLI, a granted permission, a reachable host — stay, moved into prose at the step they affect. That distinction is what keeps this a labeling change rather than a content reduction.

### Reachability is computed from roots, not from mutual links

The naive check — "does any document link to this one" — passes for a pair of documents that link only to each other. `docs/reports/` is exactly that shape today: `desktop-client-verification-2026-08-20.md` links `e2e-test-report-2026-08-19.md`, and nothing links either.

The check therefore performs a graph traversal from a fixed root set rather than an inbound-link count:

- Roots are the four `SUMMARY.md` navigation files, the repository entry-point documents (`README*.md`, `CONTRIBUTING.md`, `AGENTS.md`, `SUPPORT.md`, `SECURITY.md`, `CODE_OF_CONDUCT.md`), and `openspec/specs/**/spec.md`.
- Edges are Markdown links to repository-relative `.md` targets, resolved from the linking document's directory, with fragments stripped and percent-encoding decoded.
- Every committed `docs/**/*.md` not reached from a root is reported.

Specs count as roots because a specification that mandates a documentation location — as `provider-plugin-sdk` does for `docs/provider-sdk/` — is a legitimate entry point. Prose that names a directory in backticks is not a link and does not count; that is why `docs/provider-sdk/` is unreachable today despite being mandated, and why this change also links it from the developer guide.

### Long-term reference material is linked, not promoted to chapters

`docs/provider-sdk/` (5 documents), `docs/desktop-release-verification.md`, and `docs/runtime-performance-budgets.md` carry durable value. Promoting them to `SUMMARY.md` chapters would oblige a Simplified Chinese counterpart for each under the bilingual-equivalence requirement this change introduces, which is disproportionate for reference material that is already accurate.

They are instead linked from a reference section in both developer-guide `index.md` files — the same mechanism `docs/VaneHub-AI-技术架构深度解析.md` already uses, where a single Chinese document is linked from both books and labeled with the revision it was written against.

### Dated working artifacts are removed rather than relocated

`docs/agent-platform-roadmap/` (14 documents and a `manifest.json` delivery descriptor), `docs/reports/` (2 dated verification reports), `docs/ux-audit-report.md`, and `docs/ux-optimization-summary.md` are the artifacts the existing "working artifacts are kept out of the published documentation tree" requirement already forbids.

Relocating them to a non-published directory would satisfy the letter of that requirement while preserving material that is stale in a different place: the roadmap package predates the capabilities it proposes, several of which have shipped, and the UX audit references `.ux-audit/` capture files that `.gitignore` excludes, so its own evidence is unreadable to any reader. They are removed. Git history remains the record, which is what the requirement means by a non-published location.

### `docs/architecture/` is retired by moving its one document into the guide

`docs/architecture/skill-tool-runtime-security.md` is reachable — both `tool-registry.md` chapters link it — so the reachability check would not object. The existing requirement forbidding "a competing `docs/architecture/` narrative directory" is what it violates. The document becomes a developer-guide chapter in both languages, which resolves the violation and brings the sandboxed Skill Tool runtime's dependency review, rollout, and rollback record into the navigation where a maintainer looks for it.

## Risks / Trade-offs

**The reachability check can fail a legitimately new document before its link is written.** That is the intended failure mode — the requirement treats an unreferenced document as a defect — but it makes commit order matter. The check runs in `npm run docs:check`, so the feedback arrives before review rather than in CI.

**Section-structure equivalence is enforced by review, not by a check.** A heading-count comparison between languages produces false positives whenever a language legitimately splits or merges a section, so this change does not add one. What the reachability check does cover is the failure mode that actually recurred here — silently unlinked documents — and the bilingual obligation is stated in the specification for review to apply.

**Removing 18 documents is not reversible from the working tree.** All are recoverable from Git history and none is referenced by any live document, verified by the same traversal the new check performs.
