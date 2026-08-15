## 1. Files tab as a source

- [x] 1.1 Make file rows draggable and set both the application file-path clipboard type and `text/plain` on drag start; leave directory rows undraggable
- [x] 1.2 Add a copy-path action to file rows that writes the session-relative path as `text/plain` plus the application file-path type
- [x] 1.3 Give the copy action visible confirmation that the path was copied
- [x] 1.4 Define the clipboard type name in one shared module so the writer and the reader cannot drift

## 2. Composer as a target

- [x] 2.1 Handle drop on the composer: read the application file-path type and attach a whole-file reference through the existing `addFileReference`
- [x] 2.2 Handle paste on the composer: attach when the application file-path type is present, and otherwise let the paste proceed as text untouched
- [x] 2.3 Show a drop affordance while a draggable file is over the composer, tracking drag depth so crossing into a child element does not make it flicker
- [x] 2.4 Guard both handlers with the same disabled/streaming condition submission uses
- [x] 2.5 Keep the handlers alongside the rest of composer input handling rather than inline on the textarea, and confirm `ChatInputBox.tsx` stays under the 300-line cap

## 3. Localization

- [x] 3.1 Add strings for the copy action, its confirmation, and the drop affordance across all locale files
- [x] 3.2 Assert one representative string resolves in each locale so a missed key fails a test rather than surfacing as a raw key

## 4. Tests

- [x] 4.1 Dropping a file row attaches a whole-file reference with no range
- [x] 4.2 Directory rows are not draggable
- [x] 4.3 Pasting content carrying the application file-path type attaches a reference; the text is not inserted
- [x] 4.4 Pasting ordinary text inserts it unchanged and attaches nothing — including text that happens to look like a workspace path
- [x] 4.5 The drop affordance appears on drag enter, survives entering a child element, and clears on leave and on drop
- [x] 4.6 Neither gesture attaches while the composer is disabled or streaming
- [x] 4.7 A reference attached by drop or paste deduplicates against an identical typed reference and counts toward the reference ceiling
- [x] 4.8 The copy action writes both clipboard representations

## 5. Verification

- [x] 5.1 `npm run lint:ci`
- [x] 5.2 `npm run test`
- [x] 5.3 `npm run build`
- [x] 5.4 `npm run contracts:check`
- [x] 5.5 `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`
- [x] 5.6 `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`
- [x] 5.7 `cargo test --manifest-path src-tauri/Cargo.toml`
- [x] 5.8 `cargo check --manifest-path src-tauri/Cargo.toml`
- [x] 5.9 `npx playwright test` — composer and Files tab behavior change
- [x] 5.10 `openspec validate add-file-reference-drag-and-paste --strict`
- [x] 5.11 `openspec validate --specs --strict`
- [ ] 5.12 **Partly automated since:** `tests/e2e/file-reference-picker.spec.ts` dispatches a real dragstart from a Files tab row and a drop on the composer in Chromium, and asserts the reference gets attached — so the drag half is covered in a browser. What remains manual is the clipboard half in the Tauri WebView2 runtime specifically, where custom clipboard formats are the one behavior that can differ from Chromium. Original wording: manual check in the desktop runtime, not just the browser: drag a file from the Files tab onto the composer, and copy-paste a path, confirming clipboard behavior in the Tauri webview matches what the Web-adapter tests assert
