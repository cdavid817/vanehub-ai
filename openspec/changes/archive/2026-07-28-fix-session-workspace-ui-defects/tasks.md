## 1. i18n duplicate key removal

- [x] 1.1 Identify duplicate key blocks in zh-CN.json and en.json (211 lines each, commit e2ff3e28)
- [x] 1.2 Remove duplicate blocks; verify 7 missing keys (`sessionTabs.logs.list`, `.timestamp`, `.locate`, `.seeking`, `.seek.continue`, `.seek.invalid`, `.seek.not-found`) only exist in first block
- [x] 1.3 Add `findDuplicateKeys` regression test in i18n-resource-parity.test.ts
- [x] 1.4 Run `npm run test` — 305 tests pass

## 2. files-tab silent expand error

- [x] 2.1 Write failing test: error must appear in tree section (first `<section>`), not only preview panel
- [x] 2.2 Write failing test: directory must stay collapsed (ChevronRight) when load fails
- [x] 2.3 Add error notice in tree section following `PartialNotice` pattern
- [x] 2.4 Refactor `toggleDirectory`: only add to expanded set after successful `loadDirectory`; early return on failure
- [x] 2.5 Run full test suite — 307 tests pass

## 3. remote-terminal-panels dead code

- [x] 3.1 Confirm 7 components never imported outside own test file
- [x] 3.2 Confirm `RemoteTerminalStatus` type still used by live `remote-terminal-client.ts` service layer
- [x] 3.3 Delete `remote-terminal-panels.tsx` and `remote-terminal-panels.test.tsx`
- [x] 3.4 Run full test suite — 306 tests pass, 94 files

## 4. changes-tab bug fixes

- [x] 4.1 Write test: diff truncation shows `PartialNotice` (RED)
- [x] 4.2 Fix: `setSelected` only on initial load via `useRef(false)` guard
- [x] 4.3 Fix: add `PartialNotice` when `diff.truncated` is true
- [x] 4.4 Refactor: remove redundant `status` state, use `statusQuery.data` directly
- [x] 4.5 Refactor: extract `FileRow`, `DiffBody`, `Toggle` components
- [x] 4.6 Run full test suite — 312 tests pass, 95 files

## 5. i18n guardrail expansion

- [x] 5.1 Scan 7 uncovered session-workspace UI files for hardcoded text — all clean
- [x] 5.2 Add files to `checkedFiles` in i18n-visible-text-guardrail.test.ts
- [x] 5.3 Run guardrail and full test suite — all pass

## 6. Verification

- [x] 6.1 `npm run lint` — zero warnings
- [x] 6.2 `npm run test` — 312 tests pass, 95 files
- [x] 6.3 `cargo clippy --manifest-path src-tauri/Cargo.toml` — zero errors
- [x] 6.4 Manual verification: Tauri dev build launched, Files tab error path verified with temp web mock
