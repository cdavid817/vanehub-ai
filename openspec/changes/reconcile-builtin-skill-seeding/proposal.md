## Why

On a real installation the built-in Skills are gone from the product and cannot come back. The
`skills` table holds zero rows while all six built-in source directories sit on disk, so the Skill
management page lists nothing, no Skill can be bound to an Agent, and every application start logs
four errors:

```
{"level":"error","category":"skill.seed-builtins","message":"Skill already exists: tdd-discipline"}
```

The database is not damaged — `agents` has 6 rows, `sessions` has 1, `schema_migrations` has 51.
Agent seeding succeeds on the same startup that Skill seeding fails.

The failure is a closed loop. `ensure_builtins` decides what is missing by querying the
**database**; `create_document` refuses to proceed when the skill's **directory already exists on
disk**. Two different sources of truth answer "does this skill exist?", so once they disagree,
seeding tries to create what it cannot create, the transaction rolls back, the table stays empty,
and the next start repeats it. Nothing in the product can break the cycle.

The system already has a name for this state. Drift detection produces
`SkillDriftIssueType::UnregisteredSource` for exactly "a source directory exists that is not
registered", and the repair path explicitly does nothing with it:

```rust
SkillDriftIssueType::MissingSource | SkillDriftIssueType::UnregisteredSource => {}
```

So the condition is detected, reported, and then unresolvable by every available path.

**This will recur.** Skill sources live in the user's home at `~/.vanehub/skills/`; Skill records
live in the application database under `%APPDATA%\ai.vanehub.app\`. Two roots with independent
lifecycles. Clearing application data, changing the application identifier, restoring a partial
backup, or sharing one home directory across installs all produce the divergence. Treating "the
file is already there" as an unrecoverable error makes the seeding path fragile by construction,
not by accident.

## What Changes

- **Seeding reconciles instead of assuming a greenfield.** When a built-in Skill's source directory
  already exists but no record does, `ensure_builtins` SHALL adopt the existing source — register
  it and continue — rather than failing the whole transaction.
- **Adoption preserves what is on disk.** The existing `SKILL.md` is registered as-is. Content that
  no longer matches the shipped built-in definition is reported through the existing
  `MetadataChanged` drift, not silently overwritten, so a user's edits survive and a genuine
  divergence stays visible.
- **One failing Skill no longer blocks the others.** Seeding six Skills in a single all-or-nothing
  transaction means an unrelated problem with one leaves the other five unregistered. Per-Skill
  outcomes SHALL be reported so a partial success is possible.
- **`UnregisteredSource` becomes repairable.** The synchronization path SHALL resolve it by
  adopting the source, instead of listing an issue no action can clear.
- **Expected conditions stop being logged as errors.** An already-present built-in is a normal
  state, not an `error`-level event; logging it as one dilutes the signal that real errors carry.

### Non-Goals

- **Establishing how this installation's database and disk originally diverged.** The rollback path
  is sound — `transact` reverses filesystem work when the transaction fails — so the divergence is
  historical and no longer reconstructable. The fix is deliberately written to recover from any
  divergence rather than to prevent one specific cause.
- **Changing where Skill sources or records are stored.** Moving them under a single root would
  remove the whole class of divergence, but that is a data-migration change with its own risk
  profile and does not belong bundled with a recovery fix.
- **Workspace-scoped Skills.** The observed failure and this change are limited to global built-in
  seeding.

## Capabilities

### New Capabilities
(none — this change repairs an existing capability)

### Modified Capabilities
- `skill-management`: built-in seeding gains a defined behavior for an existing-but-unregistered
  source, per-Skill outcome reporting, and a repair path for `UnregisteredSource`.

## Impact

**Runtime:** desktop (Tauri) only; the Web/mock adapter does not seed from a filesystem.

**Rust (`src-tauri/`):**
- `contexts/tooling/skills/application/service.rs` — `ensure_builtins` reconciliation and per-Skill
  outcomes; the `UnregisteredSource` arm of the synchronization path; the log level and category of
  an already-present built-in.
- `contexts/tooling/skills/infrastructure/filesystem/mod.rs` — `create_document` currently signals
  an existing directory as `Conflict`; adoption needs to distinguish "already there" from a real
  conflict, or read the existing document instead of refusing.

**Data:** no schema change. The fix populates `skills` on the next start for installations already
in the broken state, which is the recovery this change exists to provide.

**User-visible:** the Skill management page stops being empty on affected installations, and four
`error`-level log lines per start disappear.
