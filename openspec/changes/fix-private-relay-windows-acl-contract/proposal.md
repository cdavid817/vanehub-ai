## Why

`private_relay_fs` holds MCP relay configuration — environment values, headers, database location, execution context — and `mcp-client-management` requires it be created "with current-user-only access before writing secret-bearing bytes". On Windows that access control is a DACL, and the test that is supposed to prove it does this:

```rust
assert!(windows_acl::has_private_current_user_dacl(directory.path()).expect("directory ACL"));
```

`has_private_current_user_dacl` renders the descriptor back to SDDL and compares the resulting string, byte for byte, against `D:P(A;;FA;;;{sid})`.

That fails on the GitHub Windows runner. CI run `32719136242`, job `97406616254`, checkout `1415083`: `platform::private_relay_fs::tests::windows_directory_and_file_dacls_allow_only_the_current_user` FAILED, everything else in a 3730-test workspace passed. It has never run in CI before — the Windows leg only gained `cargo test --workspace` in the change that surfaced this — and it passes on a local Windows 11 developer machine.

**We do not know whether the ACL is wrong or the test is.** Those have opposite fixes: one is a security defect in code that guards secrets, the other is an over-strict assertion. And the current assertion cannot tell us, because `assert!(bool)` prints neither side. A security contract asserted by string equality, failing with no diagnostic, is not a contract that is being checked — it is one that happens to have been passing.

## What Changes

**Phase one adds diagnosis only.** No production ACL change, no relaxed assertion. The check is replaced with a structured report of what is actually on disk, for the directory and the file separately:

* current user SID, and the owner SID
* whether a DACL is present at all, or NULL — a NULL DACL grants everyone everything and is the one outcome that must never be mistaken for "no entries"
* whether the DACL is protected, and whether it is auto-inherited
* the ACE list **in order**, because ACE ordering is itself part of the guarantee
* per ACE: allow or deny, explicit or inherited, SID, access mask, inheritance flags
* the expected contract, stated in the same structural terms
* raw SDDL, as supplementary evidence rather than as the assertion

Friendly account names are not used — they are locale- and domain-dependent, and two different principals can display identically. SIDs are the identity. ACEs are not sorted before comparison; canonical order is a property under test, not noise to normalise away.

**Phase two follows the evidence**, and the branch it takes is a finding rather than a preference:

1. Owner, principal, mask, inheritance, protection and ACE order are all correct and only the SDDL rendering differs → the test moves to structured semantic comparison, and production code is not touched.
2. There is an extra principal, an inherited ACE, a wrong mask, an unprotected DACL, or the current user's access is missing → the production ACL is repaired. Widening the permitted set to make the test green is forbidden: that is granting the access the contract exists to deny.
3. The mask difference is generic-versus-expanded rendering → both raw and normalised masks are reported, and equivalence is *proved* before the comparison is adjusted.

**Negative tests, so a passing result means something.** Every one of these must fail the check: an extra ACE for Everyone, Users, or Authenticated Users; a missing ACE for the current user; an unprotected DACL; wrong inheritance flags; a wrong access mask; non-canonical ACE order.

## Impact

* Affected specs: `mcp-client-management`
* Affected code: `src-tauri/src/platform/private_relay_fs_windows.rs`, `src-tauri/src/platform/private_relay_fs_tests.rs`
* Blocks: PR #217 (`fix-portable-pty-bounded-termination`) cannot qualify on Windows until this lands on `main`. That PR is frozen and does not touch `private_relay_fs`; it re-qualifies on all three platforms on a new merge SHA afterwards.
