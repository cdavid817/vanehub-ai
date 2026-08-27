# Personalization governance

Custom instructions, long-term memory, and the per-session limits on both live in one bounded context, `personalization`. Every runtime — OnePiece and each CLI-wrapped Agent — reads from it through one adapter and one snapshot, rather than each assembling its own prompt from the settings table.

This chapter orients contributors. The authoritative requirements live in [unified-personalization-governance](../../../openspec/specs/unified-personalization-governance/spec.md), with the layers it supersedes in [custom-instructions](../../../openspec/specs/custom-instructions/spec.md) and [agent-cross-session-memory](../../../openspec/specs/agent-cross-session-memory/spec.md).

## The context boundary

```
src-tauri/src/contexts/personalization/
├─ domain/           scopes, merge state machine, memory records, snapshots
├─ application/      use cases and the ports they need
├─ infrastructure/   the memory directory, the SQLite projection, migration state
└─ api/              PersonalizationApi — the only thing outside may hold
```

Two rules make the boundary hold, and both are enforced by `src-tauri/tests/architecture.rs`:

- **`personalization` publishes only a neutral `PersonalizationApi` (and its compatibility views). It never depends on `agent_runtime`'s ports, infrastructure, or storage.** The dependency runs one way. A context that reached back for a runtime type to save a few lines would invert it, and the line budget is not a reason to.
- **The context defines ports; adapters and cross-context assembly live in `bootstrap/`.** `RegistryAgentCapabilities` and `GovernedPersonalizationAdapter` are both in `src-tauri/src/bootstrap/personalization*.rs` — neither inside either context.

The frontend mirrors this: React components depend on `src/services/personalization-service.ts`, which the Tauri and Web/mock adapters implement in lockstep. A component never calls `invoke()`.

## The runtime adapter contract

`agent_runtime` declares what it needs as `AgentPersonalizationSnapshotPort` (`contexts/agent_runtime/application/ports.rs`). `GovernedPersonalizationAdapter` satisfies it from the governed policy.

```rust
fn snapshot(&self, context: GenerationPersonalizationContext) -> AgentPersonalizationSnapshot;
fn pinned_bodies(&self, refs: &[AgentMemoryRef]) -> ...;
```

Two properties are contractual rather than incidental:

- **`snapshot` never fails a generation.** A policy that cannot be read yields a fail-closed snapshot — no custom instructions, no long-term memory — and the turn proceeds. An answer without personalization is still an answer; refusing to generate is not. Only a *validated* last-known-good policy may stand in; there is no permissive default.
- **Bodies are fetched separately from the index.** Only the few memories that survive relevance selection need one, and loading every body to build an index would defeat the budgeting that index feeds. A ref whose record moved since the snapshot is absent rather than silently newer.

### Snapshot sequence

```
turn starts
  └─ resolve_snapshot(agent, session, workspace, runtime kind, session mode)
       ├─ read policy layers: global → agent → workspace → workspace-agent
       ├─ run the merge state machine per field, recording provenance per segment
       ├─ intersect the result with the runtime's declared capabilities
       └─ pin memory refs at their current revisions
  └─ prompt assembly reads only the snapshot
  └─ pinned_bodies(selected refs) for the few that survive selection
turn ends
```

The snapshot is taken **once per generation or seat turn, at the start**. That is what makes a settings change mid-turn reach the *next* turn instead of rewriting one already planned around the old values.

Session mode is part of the resolution context, not a policy row: `standard`, `project-only`, and `temporary` are decided when the session is created and stored with the session, because a mode has to disappear with the session it constrained.

## Scope precedence and the merge state machine

`PersonalizationPolicyScope` (`domain/scope.rs`) has four variants with fixed precedence ranks:

| Scope | `precedence_rank` |
| --- | --- |
| `Global` | 0 |
| `Agent { agent_id }` | 1 |
| `Workspace { workspace_key }` | 2 |
| `WorkspaceAgent { workspace_key, agent_id }` | 3 |

Workspace outranks a generic Agent override so that project guidance wins by default; a workspace-Agent row is the explicit exception. Each layer carries an `InstructionMergeMode` (`inherit`, `append`, `replace`, `disabled`) applied per field, and each surviving segment records which layer and which action produced it. Provenance is per field rather than per layer because a layer that replaced the style rules and left the description alone produces two segments with different origins, and merging them would lose that.

A `scope_key` is built from typed newtypes joined with `/`, which is safe precisely because every identity newtype rejects `/`. Assembling one from display text would let a workspace name forge another scope's key.

### Which spellings of a path are one workspace

A workspace key is derived from a normalized path, and what counts as "the same path" is a fact about the local filesystem. `LocalPathRules` carries both rules, and both are passed in rather than read from `cfg!` at the point of use, so each is exercised in both directions on every platform:

| Platform | `fold_case` | `normalize_unicode` |
| --- | --- | --- |
| Windows | ✓ | — |
| macOS | ✓ | ✓ |
| Linux | — | — |

macOS opens the composed and decomposed spellings of one name as the same file, and a path can arrive in either form depending on whether it came from a file dialog, a shell, or git; two keys for one directory would scope a workspace's memories to whichever spelling was recorded first. On Linux those are genuinely two files, so folding them would merge two real directories into one scope. Normalization runs **before** case folding — lowercasing a decomposed name folds the base letter and leaves the combining mark, which is not the string you get by lowercasing the composed one.

A remote path takes neither rule. The far side's filesystem behaviour is not knowable from here, and applying this machine's would merge directories that are distinct on the server.

Normalization is deliberately string-only: canonicalizing would make the key depend on whether the directory exists and on symlink resolution at that instant, so a workspace would change identity when a link was repointed or a drive went offline.

## Memory: which surface is authoritative

| Surface | Authority | Rebuildable |
| --- | --- | --- |
| Markdown file under the memory directory | **Authoritative** | No |
| SQLite projection row | Derived | Yes |
| `MEMORY.md` derived index | Derived | Yes |
| Retrieval index entry | Derived | Yes |

Everything except the file is regenerated by reconciliation. This is why a failed delete reports **per surface**: a memory whose file is gone but whose retrieval entry survives is still recallable, and reporting one boolean would hide exactly the case the user needs to know about.

Corollaries worth knowing before you touch this area:

- **Generic file tools must not write into the memory directory.** Every write goes through the v2 application service, which is what keeps the projection, the index, and retrieval in step.
- **Automatic extraction produces candidates, never active records.** The active-write path belongs to an explicit human decision.
- **Display names, user titles, and model strings are never a stable memory identity.** They change; the id does not.
- **Policy and memory edits use expected-revision CAS.** Never last-response-wins — a save made against a revision that has moved is refused and both sides are shown.

## Migration and health

`MigrationStatePort` records where conversion from the pre-v2 stores got to. `MemoryHealthPort` answers whether stored memory may be used *right now*, which is not the durable row alone: a process that found maintenance held by another one knows something the row does not say.

Reads are admitted only when the store is `Ready`. An incomplete, in-progress, or repair-required migration yields **no memories rather than a partial set** — a caller would treat a partial set as the whole truth, and half the data is worse than none.

Existing valid memories migrate losslessly; entries that will not parse go to quarantine rather than being dropped. Migration, reset, and repair all enumerate the store through explicitly named maintenance queries — never through the old 200-capped scan, which would silently truncate.

## What this does not take over

VaneHub governs **what it injects**. It does not govern what a CLI does inside its own process:

- **No CLI's internal context compaction** — OnePiece's, Claude Code's, Codex CLI's, OpenCode's, Gemini CLI's, and Antigravity CLI's are theirs.
- **No CLI's native memory or instruction files** — `CLAUDE.md`, `AGENTS.md`, and the equivalents are never written or rewritten.

## Checklist: adding a VaneHub-managed Agent or runtime

Every new Agent must do both of these. Neither is optional, and neither is satisfied by "it works in my testing".

1. **Declare capabilities.** Capabilities come from the launch shape in `RegistryAgentCapabilities::for_launch` (`bootstrap/personalization.rs`):

   | Launch kind | Instructions | Memory index | Selected bodies | Automatic extraction |
   | --- | --- | --- | --- | --- |
   | `api` | ✓ | ✓ | ✓ | ✓ |
   | `cli` | ✓ | ✓ | — | — |
   | anything else | — | — | — | — |

   A launch shape this build has never heard of declares **nothing**. That is deliberate: an adapter that forgets to declare must fail closed rather than inherit OnePiece's full surface. If your runtime is a new shape, add it here explicitly — do not let it fall through to the default and then wonder why no instruction reaches it.

   A capability the runtime lacks beats a policy value that says otherwise. An enabled policy cannot make a CLI accept an injection mechanism it has no place to put.

2. **Call the resolver.** Take one snapshot per turn through `AgentPersonalizationSnapshotPort` and assemble the prompt from it. Do not read the settings table, do not read the memory directory, and do not build a second prompt-assembly path. Two assembly paths drift, and the one that drifts is the one nobody is testing.

3. **Do not hard-code the Agent list anywhere.** Registration is dynamic. `list_capabilities` enumerates the registry for exactly this reason: a screen showing only built-in Agents would be wrong the moment a user adds one, and wrong silently. Tests covering this area register a synthetic Agent to prove the path does not depend on a known id.

## Where the design lives

- [openspec/specs/unified-personalization-governance](../../../openspec/specs/unified-personalization-governance/spec.md) — scopes, merge, memory governance, session modes, migration.
- [openspec/specs/custom-instructions](../../../openspec/specs/custom-instructions/spec.md) — the instruction layer this supersedes.
- [openspec/specs/agent-cross-session-memory](../../../openspec/specs/agent-cross-session-memory/spec.md) — the memory pool this supersedes.
- [openspec/specs/retrieval-vector-search](../../../openspec/specs/retrieval-vector-search/spec.md) — the recall tool and its degradation.
- [Cross-session memory](cross-session-memory.md) — how the shared pool relates to what is governed here.
- [Native bounded contexts](native-contexts.md) — where `personalization` sits among the others.
