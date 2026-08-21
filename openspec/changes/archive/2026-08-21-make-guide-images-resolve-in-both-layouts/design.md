## Context

The user guides are published as mdBook sites assembled by `scripts/build-docs.mjs`, which places each locale at `user/<locale>/` and copies `docs/user-guide/assets/` to `user/assets/`. Image references were authored against that output layout. The repository layout differs by one level, so the same references do not resolve when the Markdown is read where it is committed.

`scripts/validate-docs.mjs` has always known this. Its `resolveAuthoredTarget` maps `../assets/` under `docs/user-guide/` back to the real directory before checking existence, which is why the link check passes on paths that do not resolve. See `proposal.md` for the counts.

## Goals / Non-Goals

**Goals:**

- One authored path that resolves in both the repository and the assembled site.
- No duplicated image files.
- Remove the machinery that hid the mismatch, so the next one fails the check.

**Non-Goals:**

- Changing the cross-book `.html` links. They are broken on GitHub for the same reason, but the alternative is broken in the assembled site — see D4.
- Changing which surfaces are captured, or capture determinism. Only where files live.
- Re-capturing. The move is a rename; the bytes are unchanged, which is what lets `docs:screenshots:check` confirm it.
- Adding images to chapters that have none.

## Decisions

### D1: Move per locale rather than copy the shared directory

**Choice:** Give each locale its own `assets/screenshots/` holding only its own captures, instead of copying the shared directory into both locales.

Copying was the obvious reading of "make it resolve in both layouts", and it is wrong for a reason that only shows up later: `docs:screenshots:update` writes to the paths in `screenshots.json`, so a copy is not maintained. The next capture run updates one location and the other silently rots — a broken image replaced by a stale one, which is worse because it still renders.

Moving works because the sharing was never real. The Chinese chapters reference only `-zh-CN` captures, the English only `-en`, with zero cross-references either way. The shared directory served two locales that never shared a file.

### D2: Put the assets inside `src/`, not beside it

**Choice:** `docs/user-guide/<locale>/src/assets/screenshots/`, referenced as `assets/screenshots/x.png`.

`src/` is what mdBook copies into a book's output, so a file placed there arrives in the site at the same relative position it occupies in the repository. That is what collapses two layouts into one path. Putting the directory beside `src/` would resolve in the repository and require build-script support again in the site — trading one special case for another.

**Consequence:** the reference loses its `../`. All 40 references change, which is mechanical, and `docs:check` verifies every one.

### D3: Delete the compensating logic instead of adjusting it

**Choice:** Remove the `../assets/` branch from `resolveAuthoredTarget` and the asset `cpSync` from `build-docs.mjs`.

Keeping either would preserve the ability to author a path that does not resolve. The validator's job is to report that, and it could not, because it was written to absorb it. Removing the branch is the part of this change that prevents recurrence; moving the files only fixes today's instance.

The build-script copy goes because mdBook now carries the images. If it stayed, the site would gain an orphaned `user/assets/` that nothing references.

### D4: Leave the cross-book `.html` links broken on GitHub

**Choice:** `../../developer/<page>.html` and `../../user/<locale>/<page>.html` stay as they are.

Unlike the images, there is no single path that satisfies both layouts. `.html` resolves in the site and 404s on GitHub; `.md` would do the reverse. The site is where a reader is directed, so it wins. The validator's cross-book resolver — added in `align-user-guide-audience-and-media` — stays, and it is a genuine mapping between two layouts rather than a substitution that hides a mistake: the authored path is correct for its target audience.

Stating this is the point. The two cases look identical and are not, and conflating them would either break the published site or leave the images broken.

## Risks / Trade-offs

- **[The move could change bytes and invalidate the captures]** → it is a rename; `docs:screenshots:check` compares the moved files against a fresh capture and confirms the bytes.
- **[mdBook might not copy `src/assets/` as expected]** → verified by `docs:build` and by checking the assembled output, not assumed.
- **[Removing the validator branch could surface other paths that relied on it]** → that is the intended effect; `docs:check` names any that do.
- **[Two asset directories instead of one is more places to look]** → each is next to the chapters that use it, which is the reason it resolves; a shared directory that no file was actually shared from was the weaker arrangement.
- **[A future locale must create its own asset directory]** → the added requirement states the scoping rule, so the expectation is written down rather than inferred from the existing layout.
