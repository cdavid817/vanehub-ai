## 1. Dependencies and highlighting

- [x] 1.1 Promote `highlight.js` from a transitive dependency to a declared one at the version already in the lockfile, confirming no lockfile version change results
- [x] 1.2 Add a helper that highlights a file's content and returns it split per line, choosing the language from the file extension and falling back to plain text for an unknown one
- [x] 1.3 Ensure the helper never throws on an unregistered language or malformed content — an unhighlighted preview is acceptable, a crashed dialog is not

## 2. Preview dialog

- [x] 2.1 Build the preview dialog on `ApplicationDialog`, taking a path and rendering the `readSessionFile` result
- [x] 2.2 Render content through the existing virtual list so a 1 MB file opens without stalling, with each row carrying its 1-based line number
- [x] 2.3 Implement anchor-then-extent selection: first click anchors, second click completes, bounds normalise so order does not matter, a third click starts over
- [x] 2.4 Keep the pending selection keyed by line number in dialog state so it survives rows scrolling out of the window
- [x] 2.5 Visually distinguish selected lines and state the pending range in the dialog
- [x] 2.6 Provide "reference whole file" and "reference selection" actions; the latter is enabled only once at least an anchor is set
- [x] 2.7 Render the oversized and binary states in place of content, offering no attach action — such a file contributes nothing to the prompt, and "Reject unsafe reference" already requires refusing it
- [x] 2.8 Render the missing state and request failures with localized feedback and no attach action

## 3. Composer wiring

- [x] 3.1 Add pending-preview state to `use-composer-mention`: selecting a candidate without a typed range records it as pending instead of attaching
- [x] 3.2 Keep the typed-range path attaching directly, with no preview
- [x] 3.3 Render the dialog from `ChatInputBox` only when the hook reports a pending candidate; confirm the composer stays well under the 300-line cap
- [x] 3.4 Route both confirmations through the existing `addFileReference` so identity, deduplication, and the reference ceiling behave identically to the typed path
- [x] 3.5 Restore the composer draft unchanged when the dialog is dismissed

## 4. Localization

- [x] 4.1 Add strings for the dialog title, both actions, the selection hint, the pending-range label, and the oversized/binary/missing/failed states across all locale files
- [x] 4.2 Assert one representative string resolves in each locale so a missed key fails a test rather than surfacing as a raw key

## 5. Tests

- [x] 5.1 Selecting a candidate with no typed range opens the preview and attaches nothing yet
- [x] 5.2 Selecting a candidate with a typed range attaches directly and opens no preview
- [x] 5.3 Two clicks produce the same range regardless of click order; a single click plus confirm yields a one-line range; a third click restarts the selection
- [x] 5.4 Selection anchored on a line, then extended to a line outside the initially rendered window, produces the correct range — the windowing regression this design is most exposed to. **Covered in two parts:** the selection semantics are unit-tested against line numbers directly, which is the whole reason the selection is not held on a rendered row; the scroll-and-click half is not automated, because the virtualizer measures a container jsdom reports as zero-sized, so component tests render all rows. That half is 6.12.
- [x] 5.5 "Reference whole file" attaches a reference with no range; "reference selection" attaches one carrying the selected range
- [x] 5.6 Dismissing attaches nothing and leaves the draft unchanged
- [x] 5.7 Oversized and binary states offer only the whole-file action; missing and failed states offer none
- [x] 5.8 Focus is trapped while open and returns to the composer on close
- [x] 5.9 Highlighting an unknown extension renders content unhighlighted rather than throwing

## 6. Verification

- [x] 6.1 `npm run lint:ci`
- [x] 6.2 `npm run test`
- [x] 6.3 `npm run build`
- [x] 6.4 `npm run contracts:check`
- [x] 6.5 `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`
- [x] 6.6 `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`
- [x] 6.7 `cargo test --manifest-path src-tauri/Cargo.toml`
- [x] 6.8 `cargo check --manifest-path src-tauri/Cargo.toml`
- [x] 6.9 `npx playwright test` — composer behavior change
- [x] 6.10 `openspec validate add-file-reference-preview-picker --strict`
- [x] 6.11 `openspec validate --specs --strict`
- [ ] 6.12 Manual check against a real project root: open the preview on a several-thousand-line source file, select a range spanning a scroll, and confirm the attached chip and the injected prompt agree on the line numbers. **Partly automated since:** `tests/e2e/file-reference-picker.spec.ts` now anchors line 3, scrolls the virtualized list until line 380 enters the DOM, asserts that line carries the content that belongs on line 380, and checks the resulting chip reads L3-380 — the windowing risk this task existed for. What remains genuinely manual is only the *real project root*: the E2E runs against the Web mock workspace.
