# Design

## What the code does today

Production sets the DACL from an SDDL literal and marks it protected:

```rust
fn apply_current_user_dacl(path: &Path, token: HANDLE) -> io::Result<()> {
    let sid = current_user_sid(token)?;
    apply_sddl(path, &format!("D:P(A;;FA;;;{sid})"))
}
```

`apply_sddl` then calls `SetFileSecurityW` with `DACL_SECURITY_INFORMATION | PROTECTED_DACL_SECURITY_INFORMATION`.

The check renders the descriptor back to a string and compares:

```rust
Ok(actual? == expected?)
```

So the assertion is `"D:PAI(A;;FA;;;S-1-5-21-…)" == "D:P(A;;FA;;;S-1-5-21-…)"` if Windows happens to set `SE_DACL_AUTO_INHERITED` when it stores the descriptor, and `"D:P(A;;0x1f01ff;;;S-1-5-21-…)"` vs `"D:P(A;;FA;;;S-1-5-21-…)"` if the round trip expands the mask abbreviation. Both are string differences that say nothing about access.

They are also only *candidate* explanations. Naming the likeliest cause and fixing that is how you end up fixing the wrong thing convincingly. Phase one exists to replace the candidates with a reading.

## Why the assertion is the defect regardless of the outcome

Even if the ACL turns out to be perfect and only the rendering differs, this test was not verifying the contract. It was verifying that one particular Windows build renders one particular descriptor as one particular string. It passed for the same reason a stopped clock is right — the environment happened to agree — and its failure carries no information, because `assert!(bool)` discards both operands.

The contract in `mcp-client-management` is about *access*: only the current user may reach a directory that is about to hold secrets. That is a statement about owner, principals, masks, inheritance and protection. Checking it as a string is checking a shadow of it.

## What phase one reports, and why each field is on the list

| Field | Why it cannot be dropped |
| --- | --- |
| current user SID | The expected principal. Everything else is relative to it. |
| owner SID | An owner can rewrite the DACL regardless of the DACL. If the owner is not the current user, "only the current user has access" is false no matter what the ACEs say. |
| DACL present vs NULL | A NULL DACL grants everyone full control. An empty DACL denies everyone. Rendered casually they both look like "no entries"; they are opposites. |
| protected | Unprotected means the parent's ACEs flow in later, so a directory that is private now may not be after the next inheritance recalculation. |
| auto-inherited | Distinguishes "we set this" from "Windows recomputed this". |
| ACE list, in order | Windows evaluates ACEs in order and stops at the first match. A deny after an allow does not deny. Order is the guarantee, not an artefact. |
| allow/deny per ACE | Obvious, and cheap to state. |
| explicit/inherited per ACE | An inherited allow is evidence that protection failed, even if the principal looks acceptable. |
| SID per ACE | Names are locale-dependent, renameable, and ambiguous across domains. Two principals can display identically. |
| access mask | The whole question, when the principal is right. |
| inheritance flags | Decide what child objects get, which is the next directory's privacy. |
| expected contract | So the failure states the delta rather than leaving the reader to derive it. |
| raw SDDL | Kept as supplementary evidence. It is the one field that must not be the assertion. |

## Decisions

**Diagnosis is separated from judgement.** Phase one adds a reporting function and a failure message. It does not change `restrict_to_current_user`, and it does not weaken what is asserted — if the DACL is genuinely wrong, this change must still fail, and fail loudly, until phase two repairs it.

**SIDs, never display names.** `LookupAccountSid` is a network call in a domain environment, it is locale-dependent, and it maps distinct SIDs onto identical strings. A diagnostic that says `BUILTIN\Users` when it means `S-1-5-32-545` is easier to read and worse to reason about.

**ACE order is compared, not normalised.** Sorting before comparison would hide the exact defect — a deny placed after an allow — that canonical ordering exists to prevent.

**Masks are reported raw and normalised.** `FILE_ALL_ACCESS`, `GENERIC_ALL`, and `0x1F01FF` can denote the same effective access, and can also denote genuinely different access depending on the mapping. Reporting both is what makes the difference between "equivalent" and "looks similar" checkable rather than assumed.

**Negative tests come with the fix, not after it.** A structural comparison that nothing has ever falsified is the same class of thing as the string comparison being replaced. Each negative case constructs a specific wrong DACL — extra Everyone ACE, missing current-user ACE, unprotected, wrong inheritance, wrong mask, non-canonical order — and requires the check to reject it.

## Sequencing

Phase one runs on Windows CI on its own, targeted:

```
cargo test --workspace private_relay_fs::windows_directory_and_file_dacls_allow_only_the_current_user -- --nocapture
```

The point of the targeted run is a fast, readable reading of the runner's actual state, without waiting for a full workspace suite to decide whether the security contract holds.
