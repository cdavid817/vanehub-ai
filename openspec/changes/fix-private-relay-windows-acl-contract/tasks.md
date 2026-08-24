# Tasks

## 1. Phase one — diagnosis only

No production ACL change and no relaxed assertion in this phase. If the DACL is genuinely wrong, this change must still fail, and fail loudly, until phase two repairs it.

- [x] 1.1 Replace `assert!(bool)` with a structured failure report, produced separately for the directory and the file.
- [x] 1.2 Report the current user SID and the owner SID. An owner can rewrite the DACL regardless of the DACL, so "only the current user has access" is false if the owner is someone else, whatever the entries say.
- [x] 1.3 Report whether the DACL is present or NULL. A NULL DACL grants everyone full control and an empty one denies everyone; rendered casually both read as "no entries".
- [x] 1.4 Report protected and auto-inherited separately.
- [x] 1.5 Report the ACE list **in order**, without sorting. Windows stops at the first match, so a deny after an allow does not deny — order is the guarantee, not noise.
- [x] 1.6 Per ACE: allow/deny, explicit/inherited, SID, access mask, inheritance flags.
- [x] 1.7 Report the expected contract in the same structural terms, so the failure states a difference rather than a boolean.
- [x] 1.8 Include raw SDDL as supplementary evidence only. It must not be the assertion.
- [x] 1.9 Compare by SID, never by friendly account name: names are locale-dependent, renameable, and can render identically for distinct principals.
- [ ] 1.10 Run it targeted on Windows CI.

  **The filter in the original instruction matches nothing.** The test's real path carries a `tests::` segment, so `private_relay_fs::windows_directory_and_file_dacls_allow_only_the_current_user` selects zero tests -- and libtest exits 0 on zero matches, so it reads as a pass. Verified locally: `0 passed; 0 failed; 3733 filtered out`, exit 0. The command that actually selects it:

  ```
  cargo test --workspace private_relay_fs::tests::windows_directory_and_file_dacls_allow_only_the_current_user -- --nocapture
  ```

- [x] 1.11 Local Windows baseline recorded, so the runner's reading has something to be compared against. Developer machine, Windows 11, user-profile temp directory; directory and file identical:

  ```
  current user SID : S-1-5-21-2866764460-1384598244-3585758151-1001
  owner SID        : S-1-5-21-2866764460-1384598244-3585758151-1001
  DACL             : present
  protected        : true   auto-inherited : false
  entries (1, in stored order):
    #0 allow explicit sid=S-1-5-21-...-1001 mask=0x001f01ff (FILE_ALL_ACCESS) inheritance_flags=0x00
  differences: <none>
  raw SDDL (diagnostic only): D:P(A;;FA;;;S-1-5-21-...-1001)
  ```

  The raw SDDL here is byte-identical to what the old check expected, which is why it passed locally and says nothing about why it fails on the runner.

## 2. Phase two — follow the reading

Exactly one of these applies, and which one is a finding rather than a preference.

- [ ] 2.1 **Rendering only.** Owner, principal, mask, inheritance, protection and ACE order all correct, SDDL text differs → move the test to structured semantic comparison. Do not touch production code.
- [ ] 2.2 **Real defect.** Extra principal, inherited ACE, wrong mask, unprotected DACL, or missing current-user access → repair the production ACL. Widening the permitted set to make the test green is forbidden: that grants exactly the access the contract exists to deny.
- [ ] 2.3 **Mask rendering.** Generic versus expanded representation → report raw and normalised masks, and *prove* equivalence before adjusting the comparison.

## 3. Negative tests

A structural check that nothing has ever falsified is the same class of thing as the string comparison it replaces. Each case builds a specific wrong DACL and requires rejection.

- [x] 3.1 Extra ACE for Everyone (`S-1-1-0`).
- [x] 3.2 Extra ACE for Users (`S-1-5-32-545`).
- [x] 3.3 Extra ACE for Authenticated Users (`S-1-5-11`).
- [x] 3.4 Missing ACE for the current user.
- [x] 3.5 DACL not protected.
- [x] 3.6 Wrong inheritance flags.
- [x] 3.7 Wrong access mask.
- [x] 3.8 Non-canonical ACE order.

## 4. Verification

- [ ] 4.1 Targeted Windows CI run, `--nocapture`, reading recorded verbatim.
- [ ] 4.2 `cargo test --workspace`: Windows.
- [ ] 4.3 `cargo test --workspace`: Linux.
- [ ] 4.4 `cargo test --workspace`: macOS.
- [ ] 4.5 `clippy`, `fmt`, `architecture:check`, `native:panic:check`, `openspec validate --strict`.

## Forbidden

- [x] X.1 No widening of the permitted access set to turn the test green.
- [x] X.2 No assertion on raw SDDL equality.
- [x] X.3 No ACE sorting before comparison.
- [x] X.4 No friendly account names in the comparison.
- [x] X.5 No `#[ignore]`, no skip, no retry-until-green.
- [x] X.6 No change to `fix-portable-pty-bounded-termination` (PR #217) from this branch, and no change to `private_relay_fs` from that one.

## Out of scope

- [x] Y.1 `code_intelligence::initialize_timeout_forces_bounded_process_tree_cleanup` has now failed twice under load (10 pass / 2 fail), which is repeat evidence rather than a one-off. It belongs to `fix-code-intelligence-bounded-cleanup-flake` and is not folded in here.

## Status

- Phase one: **IMPLEMENTED**, awaiting the Windows CI reading
- Local Windows: focused suite 12/12 (1 contract + 2 negative suites + 9 pre-existing); clippy, fmt, architecture 44/44, native-panic and openspec strict all clean
- Windows / Linux / macOS: **NOT RUN**
- Archive: **BLOCKED**
- `fix-portable-pty-bounded-termination` (PR #217): **FROZEN**, blocked on this change landing on `main`
- `fix-sqlite-deferred-write-upgrade-contention`: **BLOCKED**
- `add-unified-extension-platform` Task Group 4: **BLOCKED**
