## Why

Every image in the user guides is broken when the Markdown is read on GitHub.

A chapter at `docs/user-guide/zh-CN/src/automation.md` writes `![…](../assets/screenshots/scheduled-tasks-zh-CN.png)`. Resolved against the repository, that is `docs/user-guide/zh-CN/assets/…`, a directory that does not exist; the file is one level up at `docs/user-guide/assets/`. The path is authored for the **assembled site** layout, where `src/` content lands at `user/<locale>/` and `build-docs.mjs` copies the shared asset directory to `user/assets/`, so `../assets/` is correct there and only there.

Nothing caught it, because `scripts/validate-docs.mjs` carries a special case that maps `../assets/` back to `docs/user-guide/assets/` before checking existence. The validator compensates for the mismatch instead of reporting it, so `npm run docs:check` has always passed while the repository view has always shown broken images. The same is true of the `.html` cross-book links: correct in the built site, 404 on GitHub.

This matters more now than it did. The English guide went from one screenshot to twenty in `align-user-guide-audience-and-media`, so the number of broken images on the repository page went from about twenty to forty.

## What Changes

Each locale's captures move inside that locale's book, under `src/`, and the references drop the `../`:

| Before | After |
| --- | --- |
| `docs/user-guide/assets/screenshots/x-zh-CN.png` | `docs/user-guide/zh-CN/src/assets/screenshots/x-zh-CN.png` |
| `docs/user-guide/assets/screenshots/x-en.png` | `docs/user-guide/en/src/assets/screenshots/x-en.png` |
| `![…](../assets/screenshots/x.png)` | `![…](assets/screenshots/x.png)` |

**Nothing is duplicated.** The Chinese chapters reference only `-zh-CN` captures and the English chapters only `-en` captures — verified, zero cross-references in either direction — so every file has exactly one home. The shared directory existed to serve two locales that never shared a file.

One path then satisfies both layouts. In the repository, `assets/screenshots/x.png` resolves next to the chapter. In the assembled site, mdBook copies non-Markdown files out of `src/` into each book's output, so it resolves there too.

Two pieces of compensating machinery are removed rather than adjusted:

- The `../assets/` special case in `scripts/validate-docs.mjs`, because the authored path now resolves without help. Its removal is what makes a future path mistake fail the check instead of being silently mapped.
- The asset copy in `scripts/build-docs.mjs`, because mdBook now carries the images.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `user-guide-documentation`: gains a requirement that a guide's own media must resolve from the authored Markdown as committed, not only after a build step, and that a validator must not compensate for a path that does not resolve. The existing screenshot determinism, alt-text, and safety requirements are unchanged.

## Impact

**Runtime scope: neither.** Documentation, documentation fixtures, and two build/validation scripts. No application code, no Tauri command, no frontend service, no runtime adapter, no SQLite migration.

Affected surfaces:

- `docs/user-guide/assets/` — removed; its 40 files move into the two books.
- `docs/user-guide/{en,zh-CN}/src/assets/screenshots/` — 20 files each.
- 40 image references across the user-guide chapters.
- `docs/user-guide/screenshots.json` — 40 `path` values.
- `scripts/validate-docs.mjs` and `scripts/build-docs.mjs` — compensating logic removed.

**The cross-book `.html` links are deliberately left alone.** They are broken on GitHub for the same reason, but pointing them at `.md` would break them in the assembled site, where the reader actually is. That is a genuine trade-off between two audiences rather than a mistake, and it is not resolved by moving a file.
