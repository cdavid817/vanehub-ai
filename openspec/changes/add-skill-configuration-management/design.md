## Context

See `proposal.md` for motivation. Skill packages currently expose descriptive frontmatter and body instructions, while VaneHub already has Rust-owned SQLite access, an operating-system credential-store abstraction used by other configuration domains, frontend service adapters, and schema-backed form conventions. Planned effective Skill loading distinguishes Role and Utility contexts and keeps executable Overlay content prohibited.

The design must work with layered Skill resolution, avoid leaking configuration into third-party CLI processes, and keep configuration data out of self-evolution evidence unless it has been reduced to non-sensitive structural metadata.

## Goals / Non-Goals

**Goals:**

- Provide a declarative, bounded configuration contract that can render without Skill-specific UI code.
- Resolve deterministic User and Project values with per-property provenance and revision safety.
- Keep secrets out of general persistence and every model-visible or logged surface.
- Deliver immutable configuration snapshots to eligible native Role, Utility, and Skill tool contexts.
- Make schema drift, recovery, deletion, and adapter limitations explicit.

**Non-Goals:**

- A general settings DSL, executable validation hooks, remote schema references, or arbitrary UI components.
- Session- or message-scoped overrides in the first release.
- Automatically projecting managed values into external CLI files, environment variables, arguments, or processes.
- Letting Overlay or self-evolution candidates modify secret values or configuration records.
- Automatically coercing or migrating incompatible values after a schema change.

## Decisions

### 1. Keep `config_schema` inline and use a restricted JSON Schema subset

`config_schema` is a JSON-compatible YAML mapping inside `SKILL.md` frontmatter. Inline placement makes the schema part of the base Skill revision hash and prevents a detached schema file from drifting independently. Supported property types are string, integer, number, boolean, arrays of bounded scalar values, and enums. Objects are limited to one nesting level for grouping; recursive references, remote/local `$ref`, combinators, pattern-driven property creation, executable formats, and unknown annotations are rejected.

VaneHub annotations provide presentation metadata such as label key or fallback label, help text, order, advanced flag, multiline flag, and `x-vanehub-secret`. Backend normalization produces a presentation-neutral descriptor shared by Tauri and Web adapters. A bespoke form DSL was rejected because it would duplicate validation semantics. Loading schemas from arbitrary supporting files was rejected for the first release because it complicates integrity and path policy.

### 2. Store values as typed scoped records, not per-Skill JSON files

SQLite stores one configuration record per Skill id, scope, canonical workspace identity, and schema hash, plus a monotonically increasing revision and typed non-secret JSON document. This follows the project's native persistence boundary, supports atomic updates and queries, and avoids concurrent filesystem writers. User and Project scopes are independent records; no System or Remote value record exists. Defaults come only from the effective schema.

Effective resolution occurs per property: Project override, then User override, then default. Missing stays missing. The resolver returns value provenance, readiness, and a digest; it does not materialize an inherited value into a higher scope. Compare-and-swap on both schema hash and stored revision prevents stale UI or concurrent processes from overwriting changes.

### 3. Model secrets as separate slots with three-way mutation intent

Secret properties receive stable credential aliases derived from an opaque configuration record id and normalized property id, never a reversible value-bearing reference. SQLite stores only configured/missing/error state and an alias witness needed by the native adapter. DTOs never return the alias. Save requests use `preserve`, `replace(value)`, or `clear`.

Multi-resource changes use a compensation sequence: validate everything; stage/replace credential values while retaining prior values where the credential API permits; commit SQLite CAS; finalize deletion of obsolete credentials; compensate if SQLite fails. If the platform cannot provide a guaranteed atomic credential transaction, the operation reports an explicit recovery state and schedules bounded reconciliation rather than claiming success.

`x-vanehub-secret` cannot be changed to or from non-secret without explicit migration. This prevents accidental movement between SQLite and the credential store.

### 4. Bind stored state and runtime snapshots to the effective revision

The schema hash is computed from canonical normalized schema content and included with the effective base Skill revision. On discovery or winning-scope change, stored records are revalidated without mutation:

- `compatible`: all stored keys still exist and values validate;
- `migration-required`: a property was removed, retyped, reclassified, or tightened incompatibly;
- `invalid`: corruption or a schema that itself cannot be normalized.

Adding an optional property or compatible default does not require migration. Invalid properties are never copied into a runtime snapshot. Missing required configuration prevents only the affected activation. Explicit reconciliation lets the user edit against the new schema and decide how to remove obsolete records and credential slots.

### 5. Runtime snapshots contain public values and opaque secret state

The configuration service creates an immutable snapshot immediately before Role load, Utility delegation, or Skill tool invocation. The snapshot includes Skill/revision/schema ids, canonical workspace, typed non-secret values, provenance, secret-presence flags, readiness, and digest. The parent operation retains it for its lifetime, so later edits affect only later activations.

For model-visible instruction contexts, a stable-key-order configuration block includes bounded non-secret values and only configured/missing secret states. It counts against a separate configuration subsection limit within the applicable Skill budget; oversize data fails activation instead of truncating a semantic value. A native Skill tool may request use of a secret only through an explicitly declared host capability bound to that property; raw secret bytes are not returned to the model or general-purpose sandbox.

External CLI mounts receive the original Skill package without values. A future bridge must define its own security, lifecycle, and compatibility behavior before projection is allowed.

### 6. Overlay and evolution operate on schema, never stored values

Overlay replay can change non-executable `SKILL.md` text under its existing trust and reconciliation rules, but any change affecting `config_schema` creates a new effective schema hash and triggers drift. Overlay learning blocks cannot write configuration records. Signal collection, evidence dossiers, generation prompts, Curator candidates, logs, and notifications receive only schema identity, readiness, changed-key names when safe, and redacted error codes—never values or credential aliases.

Automatic evolution application cannot create, replace, or clear User/Project configuration. This preserves the user's authority over operational values even when Skill instructions evolve.

### 7. Keep configuration behind native and frontend service boundaries

Rust extends the Skill context with schema, configuration, credential, snapshot, drift, and audit domain/application modules. Tauri commands expose descriptor/read, validate/preview, save/reset, secret clear, reconcile, and deletion-retention operations with command-boundary error mapping. The frontend adds matching `AgentService` contracts; only `tauri-agent-client.ts` invokes native commands.

`web-agent-client.ts` keeps the same DTO and validation shape for deterministic mock/preview flows. Without a secure remote backend it does not persist secrets or claim native runtime consumption. React renders normalized descriptors, so new Skills do not introduce component branches.

### 8. Make deletion and restore conservative

Archive retains User/Project values and credentials but prevents new snapshots. Restore revalidates them against the restored effective schema. Deleting a user-created Skill with data requires an explicit retain-or-delete choice. Retain leaves orphaned records hidden but recoverable if the same stable Skill identity returns; delete removes non-secret rows and attempts credential cleanup with a redacted recovery report.

System Skill removal during application upgrade retains user configuration records for a bounded orphan-retention period. Cleanup is a separate explicit maintenance operation, not an install side effect.

### 9. Generate UI from normalized descriptors

The Skill detail page gains a Configuration tab with scope switcher, readiness banner, inherited/effective indicators, generated controls, advanced disclosure, secret status/actions, validation summary, save/reset, and reconciliation mode. Draft state tracks the descriptor/schema hash and stored revision. Server errors preserve the draft; a stale response cannot replace a newer selected Skill or scope.

Controls reuse Tailwind and existing accessible primitives, are split below 300 lines, and never receive credential values after save. Field labels use localization keys when bundled and safe fallback text for user-created Skills.

## Risks / Trade-offs

- [Restricted schemas cannot express every configuration UI] → Cover common scalar and bounded-list cases; require a separate proposal before widening the schema or adding custom components.
- [Credential storage and SQLite cannot form one universal transaction] → Use explicit compensation, recovery states, reconciliation, and tests for every failure point.
- [Configuration blocks consume model context] → Keep values bounded, deterministic, non-secret, and separately budgeted; prefer native tool parameters when configuration need not guide reasoning.
- [Schema changes can temporarily disable a Skill] → Preserve old values, classify drift clearly, isolate the failure, and provide explicit reconciliation rather than coercion.
- [User-created labels may become an injection surface] → Treat labels/help as display text only, sanitize rendering, cap lengths, and never promote them into system authority.
- [Users may expect CLI bindings to receive values] → Display consumption support per binding and prohibit implicit projection.
- [Retained orphan credentials can accumulate] → Expose audited cleanup with retention policy while avoiding destructive automatic deletion.

## Migration Plan

1. Add schema parsing/normalization and read-only configuration metadata; existing Skills without schemas remain unchanged.
2. Add SQLite records, credential adapters, CAS, redacted DTOs, and reconciliation tests with configuration consumption disabled.
3. Release the Configuration tab and User/Project save, reset, preview, secret, and drift flows.
4. Enable immutable snapshots for native Role loading and Utility delegation, then integrate Skill tool secret capabilities after their sandbox boundary is available.
5. Add archive/delete retention handling, redacted evolution filters, operational cleanup, and rollout telemetry.
6. Roll back by disabling runtime configuration consumption and write commands while retaining schema-independent stored records and credentials for recovery; no Skill package rewrite is required.
