## Context

Two prior changes authored cross-book links as assembled-site paths on the stated grounds that the site is the reader-facing surface. That premise was never checked. There is no deployment workflow, and the READMEs link 30 times directly into `docs/user-guide/<locale>/src/*.md`. See `proposal.md`.

`scripts/validate-docs.mjs` checks that a link's file exists and, since the previous change, resolves two cross-book path shapes. It has never checked a fragment. `scripts/build-docs.mjs` assembles the three books plus Rustdoc into `.docs-build/`.

## Goals / Non-Goals

**Goals:**

- Cross-book links resolve where readers are.
- The assembled site keeps working, without authoring against it.
- A wrong anchor fails the check instead of silently landing at the top of a page.
- No document under `docs/` is reachable from nowhere.

**Non-Goals:**

- Publishing the site. Whether to deploy is a product decision; this change makes the documentation correct given that it currently is not deployed, and the build rewrite means deploying later would not reintroduce the problem.
- Consolidating the 4169-line architecture survey into the developer guide, or translating it. It is linked and labeled; folding in 538 KB of Chinese prose written against an old revision is its own piece of work.
- Changing the image paths. `make-guide-images-resolve-in-both-layouts` already made those resolve in both layouts, and its approach is unaffected.
- Validating anchors in `openspec/` or `.superpowers/`. Scope is `docs/`.

## Decisions

### D1: Author for GitHub, transform when building

**Choice:** Cross-book links are repository-relative Markdown paths. `build-docs.mjs` rewrites them to site paths in the generated HTML.

The direction matters. Authoring against the site and hoping readers build it locally inverts the dependency: it makes the common surface wrong to keep the rare one right, and it cannot be validated without the compensating resolvers the previous change added. Authoring against the repository is checkable by ordinary path resolution, and the build — which already assembles, copies, and rewrites — absorbs the difference.

**Rewrite performed:** in the built HTML, `../../../developer-guide/src/X.md` becomes `../../developer/X.html`, and `../../user-guide/<locale>/src/X.md` becomes `../user/<locale>/X.html`. It runs over the generated output, so no source file is mutated and the repository stays the single authored form.

### D2: Delete both cross-book resolvers rather than keep them

**Choice:** Remove the `../../developer/<page>.html` and `../../user/<locale>/<page>.html` branches from `resolveAuthoredTarget`.

They exist only to make an unresolvable authored path pass. With the new paths, ordinary resolution suffices. Keeping them would leave the door open to authoring `.html` again and having it validate — the same shape of defect as the `../assets/` branch removed in the previous change, and it would hide exactly the regression this change fixes.

### D3: Validate anchors with the toolchain's own id rules

**Choice:** Implement mdBook's `normalize_id` — keep alphanumeric, `_`, `-`, and space; lowercase ASCII uppercase; space to `-`; drop everything else — and resolve duplicate headings with the `-1`, `-2` suffixes mdBook appends.

**Why the toolchain's rules and not GitHub's:** they differ, and a checker built on the wrong one produces false results in both directions. mdBook's is the stricter of the two for this content, so a link that satisfies it also lands correctly on GitHub for every case in this repository — verified across all 158 anchored links, which produced exactly one disagreement, and that one is a genuine break under both.

Headings inside fenced code blocks are skipped, and inline markup is stripped before normalising, because `## **Bold** heading` and `` ## `code` heading `` both produce ids without the markup.

### D4: Link the orphans rather than delete or fold them

**Choice:** `cli-agent-global-configuration.md` is linked from the developer guide as current reference. The architecture survey is linked as a snapshot, naming the revision it was written against.

Deleting either destroys content on the strength of "nothing links it", which is the weakest possible evidence — being unreferenced is what made them invisible, not what makes them worthless. Folding the survey in is a real piece of work: 4169 lines, 164 headings, Chinese, anchored to `文件:行号` against commit `bb3d28d8`, inside a developer guide the spec scopes to English.

Labeling it as a snapshot with its revision is what keeps it honest. Without that, a reader cannot tell a survey of an old commit from maintained narrative — and its `文件:行号` anchors make it look authoritative precisely where it is most likely to have drifted.

## Risks / Trade-offs

- **[The build rewrite could miss a shape and ship a broken site link]** → the rewrite is verified against the built output by resolving every cross-book link in the generated HTML, not by inspecting the regex.
- **[Anchor validation could produce false failures on unusual headings]** → it was run across all 158 existing anchored links before being wired in; one failure, and that one is real.
- **[This reverses a decision from two changes ago]** → stated as a reversal in the proposal rather than presented as a refinement. The earlier reasoning was wrong, not differently weighted.
- **[Linking a 538 KB Chinese snapshot from an English guide is incongruous]** → accepted and labeled; the alternative was leaving it unreachable, which is what let its broken anchor survive.
- **[Deploying the site later would change which form is primary]** → the build rewrite means both forms already work, so a deployment would not require re-authoring.
