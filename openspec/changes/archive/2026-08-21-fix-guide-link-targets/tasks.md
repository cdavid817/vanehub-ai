## 1. Repoint the cross-book links

- [x] 1.1 Rewrite the 17 `../../developer/<page>.html` links in the user guides to `../../../developer-guide/src/<page>.md`.
- [x] 1.2 Rewrite the developer guide's `../../user/<locale>/<page>.html` link to `../../user-guide/<locale>/src/<page>.md`.
- [x] 1.3 Confirm no `.html` cross-book link survives under `docs/` — zero, including the four locale-switcher `<a href>` links in the index pages, which were broken the same way and were not in the original count.

## 2. Keep the assembled site correct

- [x] 2.1 Add a post-build rewrite to `scripts/build-docs.mjs` mapping the authored Markdown paths to site paths in the generated HTML, per design D1.
- [x] 2.2 Verify by resolving every cross-book link in the built output: 38 links (19 authored, doubled by mdBook's `print.html`), zero unresolved, zero authored paths left unrewritten. The first attempt matched `.md` and rewrote nothing, because mdBook already swaps the extension even on a path that leaves the book.

## 3. Validate anchors

- [x] 3.1 Implement mdBook's `normalize_id` in `scripts/validate-docs.mjs`, including duplicate-heading suffixes, skipping fenced code and stripping inline markup.
- [x] 3.2 Check every link fragment under `docs/` against the target document's heading ids.
- [x] 3.3 Remove the two cross-book resolvers from `resolveAuthoredTarget` per design D2.
- [x] 3.4 Verify the new check has teeth: deliberately wrong same-file, cross-file, and cross-book fragments each fail `docs:links:check` with exit 1 and name the target document.
- [x] 3.5 Add unit coverage for id normalisation, CJK, duplicate suffixes, fenced-code skipping, and inline-markup stripping — 32 tests pass.

## 4. Fix the broken anchor

- [x] 4.1 Correct the `Plan-Agent` fragment in `docs/VaneHub-AI-技术架构深度解析.md`.

## 5. Make the orphans reachable

- [x] 5.1 Link `docs/cli-agent-global-configuration.md` from the developer guide as current reference.
- [x] 5.2 Link `docs/VaneHub-AI-技术架构深度解析.md` as a point-in-time snapshot, naming the revision it was written against.
- [x] 5.3 Confirm no Markdown file under `docs/` is referenced from nowhere.

## 6. Sync the specs

- [x] 6.1 Sync the `user-guide-documentation` delta into the main spec.
- [x] 6.2 Sync the `native-developer-documentation` delta into the main spec.

## 7. Verification

- [x] 7.1 `npm run docs:check` passes, now including anchors.
- [x] 7.2 `npm run docs:test` passes.
- [x] 7.3 `npm run docs:build` passes and every cross-book link resolves in the built site.
- [x] 7.4 `npm run lint:ci` passes.
- [x] 7.5 `openspec validate "fix-guide-link-targets" --strict` and `openspec validate --specs --strict` pass.
- [x] 7.6 Confirm all 158 anchored links resolve and all 19 cross-book links resolve from the committed Markdown, counted directly.
