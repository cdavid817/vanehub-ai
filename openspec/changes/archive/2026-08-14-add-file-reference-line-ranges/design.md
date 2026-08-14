## Context

See `proposal.md` - Why for the motivation. Constraints that shape the approach:

- `compose_prompt` in the sessions application service is the single place that turns references into prompt text, and it is shared by CLI and API runtimes. It reads each referenced file through the workspace port and wraps the full text in `--- FILE: <path> ---` markers.
- `FileReference` in the sessions domain holds `id`, `path`, `name`, `size_bytes`, `content_hash`. `FileReferenceSet` caps the count at `MAX_FILE_REFERENCES` (5) and rejects a repeated `path`.
- `messages.file_references` is a JSON `TEXT` column. Added optional fields deserialize as absent on existing rows, so there is no migration.
- The composer's mention token is the trailing `@...` run, parsed by `composerMentionQuery` and shared with the layout model that turns it into a search query. `[^\s@]*` already admits `:` and `-`, so `@src/utils.rs:10-50` arrives as one token today - it is simply searched verbatim and finds nothing.
- After the preceding change, `ChatInputBox.tsx` is at 180 lines against the 300-line cap and mention state lives in `use-composer-mention`.

## Goals / Non-Goals

**Goals:**

- Let a reference name a region, and spend context proportional to that region.
- Keep every existing reference, persisted or newly created without a suffix, behaving exactly as before.
- Keep the change inside the existing `sendMessage` payload - no new command, no new service method.

**Non-Goals:**

- The preview modal, drag-and-drop, and clipboard paste. They produce ranges through the data path this change builds.
- Changing the injection strategy. Whole-file inlining stays; see proposal.
- Raising `MAX_FILE_REFERENCES`. Multi-region referencing makes the ceiling easier to reach, which is a separate product decision.

## Decisions

### ADDED requirements only, no MODIFIED block

The obvious delta shape would modify the existing "Chat file references" requirement. Rejected: the preceding change (`expand-file-mention-candidate-coverage`) already carries a MODIFIED block for that exact requirement and is not archived yet. Archiving replaces the requirement wholesale, so two pending MODIFIED blocks built from the same pre-change text would make the outcome depend on archive order — whichever archived second would silently revert the other.

Expressing this change purely as ADDED requirements removes the ordering hazard entirely: the two changes touch disjoint requirement names and can be archived in either order. It also reads better, since line ranges are a distinct behavior rather than a revision of how candidates are found.

### Range lives on the reference, not in the path string

A range could be encoded into the stored `path` (`src/utils.rs:10-50`). Rejected: `path` is what containment checks, file reads, and deduplication resolve against, and overloading it would make every one of those parse a suffix. Two dedicated optional fields keep `path` meaning exactly one thing.

### Identity is (path, start, end)

`FileReferenceSet` moves its dedup key from `path` to the triple. This is what makes "reference two regions of one file" work, and it degrades correctly: every persisted reference has no range, so its identity is `(path, None, None)` — the old behavior exactly. `MAX_FILE_REFERENCES` still bounds the set.

The knock-on effect is that removal can no longer be keyed by path. The composer, the layout model, and `MessageItem` move to the reference `id`, which the layout model must now derive from the path *and* the range rather than from the path alone.

### Clamp rather than reject an out-of-range end

A range whose end exceeds the file is clamped to the last line. Rejecting it would be defensible for a freshly typed range, but a reference can be restored from history long after the file shrank, and failing the send for a stale bound punishes the user for editing their own code. A start line past the end yields an empty region — the file is still identified in the prompt, which tells the Agent the reference existed and pointed nowhere.

Alternative considered: clamp silently vs. surface a notice. Deferred — the injected block already states the range it actually covers, which is visible to the Agent and to anyone reading history.

### Parse the range in the mention hook, not the component

Splitting `@path:10-50` into a path and a range happens where mention parsing already lives. Two consequences fall out of putting it there:

- Candidate search queries the path portion, so completion keeps working while the user types the range. Without this, typing `:` would empty the completion list.
- `ChatInputBox.tsx` gains no parsing code, which matters because the follow-up change adds drop and paste handlers to the same file.

**Revised during implementation.** The original plan was for selecting a candidate to insert a bare path, on the assumption that the user types the range after picking the file. That assumption is wrong in the case the parser makes possible: because search queries the path portion, `@utils:10-50` still finds `src/utils.rs`, so a user can type the range *first* and then pick from completion. Inserting a bare path there would silently discard what they just typed.

So the hook exposes the parsed range alongside the suggestions, and selection carries it into both the attached reference and the re-inserted text. This costs one extra returned value and removes the only path by which a typed range could vanish.

### Slicing in `compose_prompt`

`compose_prompt` keeps reading the whole file through the existing port and slices in memory. Reading only the requested lines from disk would be more efficient, but it needs a new port method, and the existing read already enforces the 1 MB ceiling and binary detection that make slicing safe. The saving this change is after is context, not I/O.

Injected lines are prefixed with their 1-based position. This costs a few tokens per line and buys the Agent the ability to cite positions that match the user's editor — without it, an Agent told "lines 10-50" and handed unlabelled text tends to count from 1.

## Risks / Trade-offs

**Two pending changes touching one capability** → Resolved by construction above: disjoint requirement names, archive order irrelevant. Worth re-checking if a third change lands before either is archived.

**Path-keyed removal is load-bearing in more places than it looks** → Composer chips, layout-model state, and history chips all key on path today. Missing one leaves a UI where removing one region removes both. Covered by a test that attaches two regions of one file and removes one.

**`MAX_FILE_REFERENCES` is easier to hit** → Five references was generous when each meant a whole file; it is tight when a user pins four regions of one file. Not raised here, but if the existing over-limit error surfaces as a raw domain error rather than localized feedback, that becomes visible now — worth confirming during implementation and filing separately if so.

**Range syntax collides with Windows paths** → A session-relative path never carries a drive letter, so `C:` cannot appear. The suffix is parsed only from the last `:` in the token, and only when what follows is digits or digits-dash-digits, so a path containing a colon fails to parse as a range and is treated as part of the path.

**Existing tests construct references positionally** → The domain constructor gains parameters; every call site in tests must be updated. Mechanical, but it is where an accidental argument transposition would hide.

## Migration Plan

No schema migration, no persisted-format break. The JSON column gains two optional keys; rows written before this change deserialize with both absent and render and inject exactly as they do today. Rollback is reverting the code — any rows written with ranges would then deserialize with the extra keys ignored and behave as whole-file references, which is degraded but not broken.
