## 1. Move the captures into their locale's book

- [x] 1.1 Confirm no chapter references a capture from the other locale, so the move duplicates nothing — zero cross-references in either direction.
- [x] 1.2 `git mv` the 20 `-zh-CN` captures to `docs/user-guide/zh-CN/src/assets/screenshots/`.
- [x] 1.3 `git mv` the 20 `-en` captures to `docs/user-guide/en/src/assets/screenshots/`.
- [x] 1.4 Confirm `docs/user-guide/assets/` is empty and remove it.

## 2. Repoint the references

- [x] 2.1 Rewrite all 40 chapter references from `../assets/screenshots/` to `assets/screenshots/`.
- [x] 2.2 Update the 40 `path` values in `docs/user-guide/screenshots.json` to the new per-locale locations.
- [x] 2.3 Confirm no `../assets/` reference survives anywhere under `docs/user-guide/`.

## 3. Remove the compensating machinery

- [x] 3.1 Remove the `../assets/` branch from `resolveAuthoredTarget` in `scripts/validate-docs.mjs`, keeping the cross-book resolver per design D4.
- [x] 3.2 Remove the `docs/user-guide/assets` copy from `scripts/build-docs.mjs`.
- [x] 3.3 Verify the removal has teeth: a deliberately wrong media path now fails `docs:links:check` with exit 1 and names the chapter.

## 4. Sync the spec

- [x] 4.1 Sync this change's `user-guide-documentation` delta into `openspec/specs/user-guide-documentation/spec.md`.

## 5. Verification

- [x] 5.1 `npm run docs:check` passes.
- [x] 5.2 `npm run docs:screenshots:check` passes — 40 passed, confirming the moved files are byte-identical to a fresh capture.
- [x] 5.3 `npm run docs:test` passes.
- [x] 5.4 `npm run docs:build` passes; each book's output carries its own `assets/screenshots/` with 20 files and the orphaned `user/assets/` is gone.
- [x] 5.5 `npm run lint:ci` passes — both edited scripts are JavaScript.
- [x] 5.6 `openspec validate "make-guide-images-resolve-in-both-layouts" --strict` and `openspec validate --specs --strict` pass.
- [x] 5.7 Confirm every image path resolves in both layouts: 43 references from the committed Markdown and 172 in the assembled site, zero missing in either.
