# Code review: review what the Agent produced

## Overview

After an Agent has changed a batch of files, you need somewhere to read the diff line by line, mark problems, and send your comments back. The **Changes** tab in the session workspace is that place, and [`/changes`](slash-commands.md) opens it.

Two things set it apart from an ordinary diff viewer: **comments are anchored to content fingerprints rather than line numbers**, so another round of Agent edits does not shift them out of place; and **revert is a guarded destructive operation** that refuses outright the moment the working tree no longer matches.

## Open a review

Open the Changes tab in a session with changes and the system **creates or recovers** a review session, listing the changed files as `Review files (N)`.

The review session is **persisted in SQLite**: comments, findings, and decisions survive a restart. On recovery it **revalidates the workspace witnesses first**, then presents the current anchor positions — it does not fob you off with line numbers from a stale snapshot.

## Read the diff

- **Unified and split views**, with line numbers
- **Previous changed file** / **Next changed file** for quick navigation
- The file list collapses with **Toggle files**, so it works on a narrow screen
- **Copy diff**

Diff loading is **budgeted**, which produces several states you will encounter:

| Case | What you see |
| --- | --- |
| Binary file | Marked binary; no content returned |
| Single file over budget | Marked oversized; no text diff returned |
| Whole diff over budget | The changed-file summary is kept and files are loaded individually on demand |
| Safety limit reached | "The diff is partial because the safety limit was reached." |

**These are not load failures.** The limits exist so the whole repository is never read into memory at once, and the list of changed files is always complete.

Reliably detected renames, additions, deletions, and bounded untracked text changes are all preserved.

## Comments

Select a line range and choose **Add comment**. A comment stores more than a line number:

**The anchor is determined jointly by a hunk fingerprint and a context fingerprint, never by absolute line number alone.** So when the Agent later touches the same file:

- If it can be **uniquely relocated** within bounded same-file context, the anchor moves to its new position
- If it cannot be located, or is ambiguous, the comment is **marked stale** rather than quietly pointing at unrelated code

**Resolve** transitions a comment to resolved, **preserving it rather than deleting it** — the review history does not disappear because you dealt with a point.

## Decisions and "accept"

A review tracks decisions at two scopes, the whole review and a single hunk, with values pending, accepted, and changes requested.

**Accepting a hunk records a decision only. It does not stage anything and does not modify working-tree files.** This is the easiest thing to misread: it expresses "I am satisfied with this change", not "apply this change" — the change is already in the working tree.

## Revert (destructive)

**Revert file** and **Revert hunk** really do modify your working tree, so there is a full set of guards:

1. **Explicit confirmation is required** — "Revert the selected change? This modifies the working tree."
2. **The path must stay inside this session's canonical workspace**; traversal, absolute paths, and symlink escapes are rejected before anything is read or modified
3. **The file and worktree witnesses must still match the snapshot**
4. **Application is atomic, fail-closed, and does no fuzzy matching**

The effect of rule 3: **if an external edit changed the file after the snapshot, the revert is rejected as stale and no file is modified.** Better not to act than to act wrongly — fuzzy matching in this situation would apply the change in the wrong place.

Reverting a single hunk **leaves the other changes in the same file untouched**.

## Send comments back to the Agent

Tick **Include comment in feedback**, then choose **Send feedback**.

The comments travel as a **structured envelope** across the existing session/Agent boundary, preserving file, side, line, hunk, decision, and stale metadata. The originating session receives a bounded, numbered review feedback message.

**If any selected comment has a stale anchor, you must acknowledge it explicitly** — "Some selected comments are stale. Send them with a stale marker?" The system will not present a stale anchor's old line numbers to the Agent as if they were current.

## Automated review actions

The tab offers three allowlisted actions:

| Action | Purpose |
| --- | --- |
| **Review Agent** | Have an Agent review this batch of changes |
| **Tests** | Run tests |
| **Security checks** | Security scanning |

Their terminal results are normalized into **bounded review findings**, carrying severity, title, source, an optional anchor, an operation reference, and status.

**When an action cannot produce a valid result, the operation fails with page-visible output and never fabricates findings.** This matters: an empty "found no problems" result and a "the run failed" result mean entirely different things.

## What ends up in the logs

Review actions emit **redacted, metadata-only events**: safe ids, counts, operation id, outcome category, and timing.

**Code, full diffs, comment and finding bodies, prompts, credentials, and raw tool output never reach the diagnostic logs.** So a log can tell you that a revert was rejected because its witness was stale, but it will not contain the code that was being reverted.

## Notes and limits

- **Accept does not change files**; it is only a decision record.
- **Revert does change files**, and rejects the whole operation the moment the working tree disagrees with the snapshot — it never applies partially.
- **Stale comments do not vanish**; they are preserved, marked, and require your acknowledgement when sending.
- **Binary and oversized files get no text diff**, only metadata markers.
- Automated actions depend on the existing Agent, tool, and operation runtimes; failures surface explicitly rather than returning quietly empty.

## Related

- The session workspace the Changes tab lives in → [User interface](user-interface.md)
- Opening it quickly with `/changes` → [Slash commands](slash-commands.md)
- The permission interception a revert goes through → [Permission approvals](permissions.md)
- Where the operation records from these actions go → [Observability](observability.md)
