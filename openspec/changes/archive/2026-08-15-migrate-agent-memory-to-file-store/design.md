## Context

See `proposal.md` — Why for motivation. The constraints that shape the approach:

- `contexts/tooling/skills` already implements this exact shape: a directory of Markdown files with `---` frontmatter, scanned and parsed in Rust, assembled into prompt text, with an `Eager` / `OnDemand` delivery split. Memory is a second consumer of a pattern this repo already runs in production.
- The tool catalog's declared prefix is load-bearing. Adding, removing, or reordering a tool breaks seven tests that assert fixed catalog lengths and invalidates the provider-side prompt cache prefix.
- `bootstrap/runtime.rs` already resolves the app data directory with a `VANEHUB_APP_DATA_DIR` override, so a host-level directory gets test isolation without new plumbing.
- Extraction today is one blocking HTTP call (`RuntimeAgentMemoryExtractionAdapter::extract`) returning `Option<String>`. There is no forked-agent facility in the Rust runtime.
- The Web/mock runtime has no filesystem and must keep contract and event parity.
- The tool permission mapping has a defect history (`2026-08-12-fix-onepiece-tool-permission-mapping`), which argues against designs whose safety rests entirely on it.

## Goals / Non-Goals

**Goals:**

- Every memory is an addressable unit the model can read, correct, and retract.
- The extraction path is sandboxed by construction rather than by permission checks.
- The primitives come from the Skills implementation rather than a second set.
- CLI-wrapped agent prompts are byte-identical in structure before and after; only the write sink moves.
- Migration needs no credentials and no network.

**Non-Goals:**

- Index-based injection and relevance selection. Injection keeps its current recency-and-budget shape here; `add-two-tier-memory-recall` replaces it.
- Freshness annotation and staleness caveats — also `add-two-tier-memory-recall`.
- Team or shared-across-users memory scope. The pool stays host-local.
- Changing `recall` semantics, its schema, or its position.
- Dropping the `agent_memories` table in this change.

## Decisions

### D1. One flat host-level directory, not a tree

`<app_data_dir>/memory/` holding `MEMORY.md` plus one `.md` per memory, flat.

Alternatives considered. Per-workspace directories were rejected because `agent-cross-session-memory` mandates a host-level shared pool and records the folder as provenance only — a directory tree would reintroduce the scoping the shared-pool change removed. Grouping into `user/`, `feedback/`, `project/`, `reference/` subdirectories was rejected because a memory's type changes more readily than its topic, and a type change would become a file move that invalidates every reference to it; the type lives in frontmatter where changing it is a one-line edit.

Flat also keeps paths short, which matters on Windows where deep paths under the app data directory have bitten this project before.

### D2. Frontmatter shape, and copying the Skills parser rather than sharing it

```
---
name: shared-rust-toolchain-fragility
description: Concurrent sessions' rustup updates corrupt std mid-build
type: project
agent: onepiece
folder: D:/cdavid/Documents/code/vanehub-ai
source: automatic
created: 2026-08-15T09:12:44Z
---

<body>
```

`name` is the filename stem, so the file path is the identity and the frontmatter merely restates it for readability. `description` is what the next change's selector reads instead of the body. `type` is optional and tolerated when absent or unrecognized.

`skills/infrastructure/filesystem/document.rs::parse` is a hand-rolled line scanner, not a YAML dependency, and it is coupled to `SkillMetadata`'s required fields. The memory parser copies its shape — strip `---\n`, split on `\n---`, scan `key: value` lines, ignore unknown keys — rather than extracting a shared primitive. Two consumers do not justify a third context; revisit when a third appears.

Tolerant parsing is not politeness here. The directory is host-level and therefore shared across git worktrees, so a branch running an older or newer frontmatter contract will see the other's files. Unknown keys are ignored and unparseable files are skipped from enumeration rather than failing it.

### D3. Two write paths onto one substrate

`remember` keeps its name and catalog position and becomes the one-call fast path: given name, description, type, and content, it writes the file and the index line together. The generic `read` / `write` / `edit` / `grep` tools additionally get the memory directory as an auto-approved scope, which is what makes correction and retraction possible at all.

Alternatives considered. Removing `remember` in favor of file tools alone is what Claude Code does, but it breaks the seven fixed-length catalog assertions and shifts the declared tool prefix. Keeping only `remember` cannot express "read this memory, then correct it" — the model would have to guess the prior content, which is exactly the failure the row store had.

Scope enforcement canonicalizes the requested path before comparing it against the memory directory root, so `..` traversal and symlinks cannot escape. Enforcement lives in the permission mapping, but nothing catastrophic rests on it alone: the worst outcome of a scope-check bug is that a write inside the memory directory prompts for approval, or that an ordinary file write is auto-approved — the latter is the real risk and is covered by a dedicated test rather than by inspection.

### D4. Extraction returns actions, and never touches the filesystem

The extraction call is a single request with no tools declared. Its input is the exchange, the manifest of existing memories (`[type] name — description`), and the full bodies of the few most relevant existing memories. Its output is a JSON list:

```
[{ "action": "create" | "update" | "delete",
   "name": "...", "description": "...", "type": "...", "body": "..." }]
```

The Rust layer validates every action — known action, resolvable name, name that canonicalizes inside the memory directory, required fields present for the action kind — then applies the survivors and drops the rest with a log line.

Alternatives considered. Replicating Claude Code's `runForkedAgent` (a real sub-agent with Read/Write/Edit and a `maxTurns` cap) gives the model the ability to investigate before writing, but VaneHub has no forked-agent facility to build on, and the sandbox would rest entirely on the permission mapping. Keeping the current `Option<String>` return and letting Rust choose the filename was rejected because Rust cannot decide whether a fact updates an existing memory or creates a new one — that judgment is the whole point of passing the manifest.

The manifest is the load-bearing part. Without it the model has no way to name an existing memory, so every extraction can only create, and the pool grows the same way it does today. Deduplication is a property of what the prompt contains, not of what the tool allows.

### D5. Retrieval identity is the directory-relative path

Alternatives considered. A stable UUID in frontmatter survives renames, but then identity and location diverge and the index needs a rename detector to avoid orphans. Path identity makes a rename look like delete-then-create, which costs one re-embedding and is correct without extra machinery. The directory scan that reconciliation needs already runs for enumeration, so reconciliation is close to free.

### D6. Migration is deterministic, additive, and leaves the rows alone

At startup, if the memory directory has not been initialized, every `agent_memories` row becomes a file: body is the row's `content` verbatim; `name` is a slug of the leading words with a numeric suffix on collision; `description` is the leading sentence truncated; `type` is omitted rather than guessed; `agent`, `folder`, `source`, and `created` carry the row's provenance. A `migrated_from` frontmatter field holds the row id, which is what makes a second run idempotent — a row whose id already appears in the directory is skipped, so a file the model or user has since edited is never overwritten.

No model call, so migration works with no credentials, offline, and at startup. Guessing the type would need one, and a wrong type is worse than an absent one because absent degrades gracefully by spec while wrong does not.

The rows are not deleted. They are the rollback: reverting the code restores the previous behavior with the data intact. Dropping the table is a later change once the directory has been in the field for a release.

### D7. Runtime boundary

| Layer | Change |
|---|---|
| React components | None. They keep depending on `agent-service.ts`, no `invoke()` |
| `agent-service.ts` | Memory record type gains `name`, `description`, `type` |
| Tauri adapter | Same command names (`list_agent_memories`, `delete_agent_memory`, `reset_agent_memories`), richer payload |
| Web/mock adapter | Same shape backed by an in-process list; no filesystem, no provider call |
| Rust `agent_runtime` | Owns the directory: `domain/memory_document.rs` for the type taxonomy and metadata validation, `infrastructure/memory_directory.rs` replacing `memory_repository.rs` |
| Rust `retrieval` | `agent_memory` document identity and reconciliation source |

Both adapters change together, so the Web runtime never diverges from the desktop contract.

## Risks / Trade-offs

- **Generic file tools can now write anywhere under the memory directory, including malformed files** → enumeration skips unparseable files rather than failing, the index is rebuilt from the directory scan so it self-heals, and the management view can delete anything the model produced.
- **Path traversal through an extraction action or a tool write** → every path is canonicalized and prefix-checked against the memory root before use, with a test for `..` and for symlinked paths.
- **`remember` changing its argument schema shifts the declared tool payload once** → this invalidates the provider prompt cache prefix a single time on upgrade. Unavoidable while changing the tool's contract; keeping the name and position confines it to one invalidation instead of a permanent reordering.
- **Extraction returns malformed or partially invalid JSON** → the schema check rejects per action, not per call, so one bad entry does not discard the good ones; a wholly unparseable response is logged and the generation is unaffected.
- **The directory is shared across git worktrees, like the SQLite database** → unlike the database there is no migration version to collide on, so the failure mode degrades from a startup crash to redundant or unfamiliar files. Tolerant parsing (D2) is what keeps that true.
- **Concurrent writers: the `remember` tool, the model's file tools, and extraction can target one file** → last write wins per file, and the index is derived from the directory rather than maintained as independent state, so a lost update costs content but never leaves the index inconsistent.
- **Migration on a large pool blocks startup** → it runs once, off the UI thread, with per-row failure isolation; a row that cannot be converted is logged and skipped rather than aborting.
- **A memory file and its index line can disagree if the process dies mid-write** → the directory is authoritative by spec, and the next enumeration reconciles the index to it.

## Migration Plan

1. Ship the directory, the parser, and the reader while `agent_memories` remains the write path — reads prefer the directory when initialized.
2. Run the row-to-file migration at startup, idempotent via `migrated_from`.
3. Switch `remember`, OnePiece extraction, and CLI extraction to write the directory.
4. Repoint the retrieval index's `agent_memory` source at file paths and switch reconciliation to the directory scan.
5. Leave `agent_memories` populated and unread for one release.

Rollback at any step before 5 is a code revert; the rows are untouched and the directory is ignored by the reverted code.

## Open Questions

- Whether the `agent_memories` table is dropped or retained as an export target once the directory has been in the field. Deferrable: it changes no spec, no interface, and no task here.
- Whether migrated files should later get types assigned by a one-off model pass. Deferrable: absent types are supported by spec, and the model can now set them in place as it touches each memory.
