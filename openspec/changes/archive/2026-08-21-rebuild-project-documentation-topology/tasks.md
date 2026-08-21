## 1. Spec artifacts

- [x] 1.1 Sync this change's `user-guide-documentation` and `native-developer-documentation` delta requirements into the main specs during apply (`openspec/specs/user-guide-documentation/spec.md`, `openspec/specs/native-developer-documentation/spec.md`).
- [x] 1.2 Fill the `TBD` `Purpose` in both main specs by direct edit (deltas must not carry `## Purpose` for existing capabilities): `openspec/specs/user-guide-documentation/spec.md` and `openspec/specs/native-developer-documentation/spec.md`.
- [x] 1.3 Run `openspec validate "rebuild-project-documentation-topology" --strict` and `openspec validate --specs --strict`; both must pass.

## 2. English user-guide topology alignment

- [x] 2.1 Rebuild `docs/user-guide/en/src/SUMMARY.md` to mirror the `docs/user-guide/zh-CN/src/SUMMARY.md` chapter topology (same chapter set, same order, English titles).
- [x] 2.2 For each English chapter that has no content yet, create `docs/user-guide/en/src/<chapter>.md` containing an explicit "known gap" notice that links to the corresponding `docs/user-guide/zh-CN/src/<chapter>.md` chapter.
- [x] 2.3 Confirm the chapters that already exist in English (`getting-started`, `first-session`, `skill-management`, `code-indexing`, `lsp-code-intelligence`, `multi-agent-workflow`, `runtime-labels`, `troubleshooting`, `index`) align in navigation order, runtime labels, and feature-state labels with their ZH-CN counterparts; reconcile any divergence.

## 3. Migrate and remove `docs/zh/`

- [x] 3.1 Walk `docs/zh/src/02-architecture/*`: migrate decision-level content into `src-tauri/ARCHITECTURE.md` or the developer guide's relevant chapter; drop duplicates already covered in `docs/user-guide/zh-CN/`.
- [x] 3.2 Walk `docs/zh/src/03-development/*`: fold unique developer-onboarding material into `docs/developer-guide/src/`; drop duplicates.
- [x] 3.3 Verify `docs/zh/src/01-overview.md` and `docs/zh/src/README.md` content is fully represented in the surviving guides; drop duplicates.
- [x] 3.4 Delete `docs/zh/` entirely (`book.toml`, `src/`, `SUMMARY.md`).

## 4. Reconcile `docs/architecture/`

- [x] 4.1 `cli-chat-runtime-v1.md`: migrate any decision-level content to `src-tauri/ARCHITECTURE.md`, then remove the file (superseded by multi-agent group chat).
- [x] 4.2 `workspace-modularization-follow-up.md`: migrate surviving decision content or remove outright.
- [x] 4.3 `agent-execution-observability.md`, `im-connectors-smoke.md`, `type-contracts.md`: evaluate each; move surviving content into the developer guide or `src-tauri/ARCHITECTURE.md`; remove stale files.
- [x] 4.4 Result: `docs/architecture/` either no longer exists or contains only clearly-labeled historical references that `docs/developer-guide` explicitly links.

## 5. Relocate `docs/superpowers/`

- [x] 5.1 Move `docs/superpowers/` to a top-level `.superpowers/` directory outside the published `docs/` tree.
- [x] 5.2 Confirm no `docs/`-scoped validator or SUMMARY references the relocated path; the artifacts remain working notes.

## 6. Root documentation entry points

- [x] 6.1 Update `README.md` "Documentation" section: links target the collapsed topology; no `docs/zh/` references remain.
- [x] 6.2 Update `README.zh-CN.md` "Documentation" section to match.
- [x] 6.3 Update `README.ja.md`: state user guides exist in EN/ZH-CN only and Japanese is an application UI locale; remove any implied Japanese guide.
- [x] 6.4 Update `CONTRIBUTING.md` documentation pointers to the collapsed topology.
- [x] 6.5 Review `AGENTS.md` for any documentation-path references and align if needed.

## 7. Validation scripts

- [x] 7.1 Extend `scripts/validate-docs.mjs` to assert no surviving file under `docs/` references `docs/zh/`, and to cover migrated chapter link targets.
- [x] 7.2 Extend `scripts/check-readme-parity.mjs` to assert no README references `docs/zh/` and to cover the surviving README localization set (EN, ZH-CN, JA) for structural parity of the Documentation section.
- [x] 7.3 Reconcile `docs/user-guide/screenshots.json` with any chapters whose path changed during migration; update screenshot scenario bindings.

## 8. Verification

- [x] 8.1 `npm run docs:check` passes (unit tests + readme parity + links).
- [x] 8.2 `npm run docs:test` passes (mdBook test on all surviving books).
- [x] 8.3 `npm run docs:build` produces the assembled site for the collapsed topology with no `docs/zh/` output.
- [x] 8.4 `npm run lint:ci` passes (validator script changes are TS/JS).
- [x] 8.5 `openspec validate "rebuild-project-documentation-topology" --strict` passes.
- [x] 8.6 `openspec validate --specs --strict` passes after main-spec sync in 1.1/1.2.
