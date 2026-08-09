## Context

Built-in Skill initialization runs on every application start. `ensure_builtins` lists the registry
for the global scope, subtracts the intentional-deletion tombstones, and treats whatever remains as
missing. For each missing Skill it calls `create_source`, then writes all the resulting records in
one repository call, the whole thing wrapped in `transact`.

`create_document` — what `create_source` reaches — begins with:

```rust
let (directory, skill_file) = self.paths.source_paths(location, id)?;
if path_exists(&directory) {
    return Err(SkillApplicationError::Conflict(id.as_str().to_string()));
}
```

So the question "does this Skill already exist?" is answered by the database in one place and by
the filesystem in another. While the two agree, nothing is wrong. Once they disagree, seeding tries
to create a directory that is already there, gets `Conflict`, and `transact` rolls the transaction
back — including the records for Skills that would have succeeded. The registry stays empty, so the
next start computes the same "missing" set and repeats.

Two properties make this permanent rather than transient:

- **The two stores have independent lifecycles.** Sources live under the user's home
  (`~/.vanehub/skills/`); records live in the application database under `%APPDATA%`. Clearing
  application data, changing the application identifier, restoring a partial backup, or pointing
  two installs at one home directory each produce the divergence.
- **No path resolves it.** Drift detection already names the state
  (`SkillDriftIssueType::UnregisteredSource`), and the synchronization path explicitly ignores it:
  `SkillDriftIssueType::MissingSource | SkillDriftIssueType::UnregisteredSource => {}`.

Observed on a real installation: `skills` 0 rows, `skill_agent_mount_paths` 0 rows, six built-in
directories on disk, four `error`-level log lines per start, and an empty Skill management page —
while `agents` (6), `sessions` (1), and `schema_migrations` (51) show the database is otherwise
healthy.

## Goals / Non-Goals

**Goals:**

- An installation already in this state recovers on the next start, with no user action.
- Divergence between the two stores is a recoverable condition, not a terminal one, whatever caused
  it.
- A user's edits to a Skill file survive adoption.
- One unusable Skill cannot cost the user the other five.

**Non-Goals:**

- Determining how this particular installation diverged. `transact` does reverse filesystem work on
  failure, so the obvious "a failed seed left directories behind" explanation does not hold, and
  the true cause is no longer reconstructable. Chasing it would delay a fix that has to be
  divergence-agnostic anyway.
- Unifying the two storage roots. That removes the class of divergence outright and is worth
  considering, but it is a data migration with a different risk profile and does not belong inside
  a recovery fix.
- Workspace-scoped Skills, external imports, and user-created Skills. Their creation paths
  legitimately treat an existing directory as a conflict: the user asked to create something new,
  and silently adopting a stranger's directory would be wrong.

## Decisions

### D1. Seeding reconciles; it does not assume a greenfield

`ensure_builtins` stops treating "absent from the registry" as "absent from the system". For each
built-in that has no record, it distinguishes:

| On disk | Action |
| --- | --- |
| No directory | Create source and record, as today |
| Directory present, readable `SKILL.md` | Adopt: register the existing source |
| Directory present, unreadable or malformed | Report that Skill as failed, continue with the others |

*Alternative considered:* have seeding delete and recreate the directory. Rejected — it discards
whatever the user has there, to fix a problem the user did not cause.

### D2. Adoption registers what is on disk and lets drift speak

The adopted record describes the file as it exists. When that content differs from the shipped
definition, the existing `MetadataChanged` drift reports it.

This is chosen over restoring the shipped content because the two failure modes are not
symmetrical: adopting a modified file and reporting the difference is recoverable and visible,
while overwriting a modified file destroys work silently. The repository already models
"registered content differs from expected" — using it costs no new vocabulary.

*Alternative considered:* overwrite on adoption so built-ins always match the shipped definition.
Rejected for the asymmetry above. A user who wants the shipped content can delete and restore the
built-in, which is an existing, explicit path.

### D3. Per-Skill outcomes replace all-or-nothing

Six Skills in one transaction means an unrelated failure on one strands the other five — which is
precisely how the observed installation ended up with zero rows rather than five. Each built-in is
reconciled and reported independently, and the operation summarizes what succeeded and what did
not.

The mutation coordinator lock and per-Skill filesystem staging still apply; what changes is that
one Skill's failure no longer discards its siblings' work.

### D4. `UnregisteredSource` becomes a repair action rather than a no-op

The same adoption logic backs the synchronization path's `UnregisteredSource` arm, so the issue
that drift already reports can now be cleared. Without this, a user looking at a reported drift
issue has no action available and no explanation.

Intentional deletions keep precedence: a tombstoned built-in stays unregistered, matching the
existing "deleted built-in is not auto-restored" guarantee.

### D5. An expected condition is not an `error`

"This built-in is already present" is a normal state, and logging it at `error` level trains
readers to ignore the error channel. The diagnostic that remains is attributed to the operation
that actually produced it — the observed lines were categorized `skill.seed-builtins` while the
`Conflict` they carried originates in the filesystem layer, which sent the first investigation to
the wrong file.

## Risks / Trade-offs

- **Adoption registers content the shipped definition did not author** → The record reflects disk,
  and `MetadataChanged` drift makes the divergence visible rather than pretending the built-in is
  pristine.
- **A directory that is not a Skill at all sits at a built-in's path** → Adoption requires a
  readable, parseable `SKILL.md`; anything else is reported as a per-Skill failure instead of being
  force-registered.
- **Per-Skill transactions widen the window for a partially-applied start** → Partial success is
  the intended improvement over total failure, and each Skill remains individually atomic.
- **The underlying two-root divergence is untouched** → Stated as a non-goal rather than implied
  away; this change makes the symptom recoverable, and consolidating the roots remains open.

## Migration Plan

1. No schema change and no data migration step.
2. On the next start after the fix, an affected installation adopts its existing sources and the
   registry populates. Recovery is the change's normal behavior, not a one-time script.
3. **Rollback:** reverting restores the previous binary, whose seeding path fails the same way it
   does today. Records written by adoption remain valid — they are ordinary registry rows — so a
   rollback leaves the installation no worse than before.

## Open Questions

- Should the two storage roots be consolidated so this class of divergence cannot recur? Out of
  scope here, worth its own change.
- Should adoption of a built-in whose content diverges be surfaced more prominently than a drift
  entry — for example, offered as "restore the shipped version" on the Skill management page?
