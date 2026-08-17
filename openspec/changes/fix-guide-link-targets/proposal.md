## Why

Nineteen cross-book links in the user guides are 404 for every reader, and the reason is a premise that turns out to be false.

`align-user-guide-audience-and-media` decided (D4) to author cross-book links as assembled-site paths — `../../developer/index.html` — on the grounds that "the site is where a reader is directed, so it wins". `make-guide-images-resolve-in-both-layouts` repeated the reasoning.

**There is no published site.** `.github/workflows/` contains ci, codeql, dependency-review, labeler, and package; none of them deploys documentation. CI's `Documentation` job builds and validates, and uploads nothing. Meanwhile the three READMEs carry 30 links that point directly at the Markdown source:

```
[From installing a CLI to working in a workspace](docs/user-guide/en/src/getting-started.md)
```

So the only surface a reader reaches is the Markdown on GitHub, and that is exactly where `.html` cross-book links fail. The earlier decision optimised for the layout nobody reads at the expense of the one everybody does.

Two further defects surfaced while checking this:

- **Anchors have never been validated.** `cleanTarget` in `scripts/validate-docs.mjs` does `split("#")[0]`, so all 158 anchored links pass regardless of whether the anchor exists. Auditing them against mdBook's `normalize_id` rules found one real break: `docs/VaneHub-AI-技术架构深度解析.md` links `#第-24-章-onepiece-原生-planagent-循环` where the heading yields `…-plan-agent-…`. A wrong anchor lands the reader at the top of a 4169-line document with no indication anything went wrong.
- **Two documents are referenced from nowhere.** `docs/VaneHub-AI-技术架构深度解析.md` and `docs/cli-agent-global-configuration.md` appear in no SUMMARY, no README, and no spec. `rebuild-project-documentation-topology` collapsed the two book trees and never looked at loose files at `docs/` root.

## What Changes

- The 19 cross-book links become repository-relative Markdown paths — `../../../developer-guide/src/index.md` from a user-guide chapter, `../../user-guide/<locale>/src/<page>.md` from a developer-guide chapter — so they resolve where readers actually are.
- `scripts/build-docs.mjs` rewrites those paths to site paths in the built HTML, so the assembled site stays correct rather than being knowingly shipped broken. Authoring targets the surface that is read; the build adapts to the surface that is generated.
- `scripts/validate-docs.mjs` gains anchor validation using mdBook's id rules, and loses both cross-book resolvers added by the previous change — the new paths resolve natively, so the compensating code has nothing left to compensate for.
- The broken anchor is corrected.
- Both unreferenced documents are linked from the developer guide: the CLI global-configuration note as current reference, the architecture survey as a clearly-labeled point-in-time snapshot pinned to the commit it was written against.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `user-guide-documentation`: gains a requirement that a link's anchor must resolve, not merely its file, and that authored links must target the surface readers are directed to. This supersedes the reasoning behind the assembled-site link convention without weakening the media requirement added by the previous change.
- `native-developer-documentation`: gains a requirement that a documentation file under `docs/` must be reachable from a guide or an entry point, so an unreferenced document is a defect rather than an invisible orphan.

## Impact

**Runtime scope: neither.** Documentation and two build/validation scripts.

Affected surfaces:

- 10 user-guide chapters and 1 developer-guide chapter — 19 link targets.
- `docs/VaneHub-AI-技术架构深度解析.md` — one anchor corrected.
- `docs/developer-guide/src/` — two previously unreachable documents linked.
- `scripts/validate-docs.mjs` — anchor validation added, two resolvers removed.
- `scripts/build-docs.mjs` — cross-book link rewrite added.

**This reverses a decision made two changes ago.** D4 of `align-user-guide-audience-and-media` is superseded, and the reasoning recorded there should be read as wrong rather than as a trade-off: it rested on a published site that does not exist.
