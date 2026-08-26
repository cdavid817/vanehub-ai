# Cross-session memory

Stored memories are a host-level pool shared by every Agent — OnePiece and all CLI-wrapped Agents (`claude-code`, `codex-cli`, `gemini-cli`, `opencode`, `antigravity-cli`) alike — but sharing is now a **default, not a rule**: each memory carries a scope and an audience. Governance of both lives in [Personalization governance](personalization-governance.md); this chapter covers the persistence half. The recall/search path is covered in [Retrieval and vector search](retrieval.md).

## Shared by default, narrowable per memory

Every memory records the Agent that produced it and the workspace it was produced in. Two of those facts are now load-bearing rather than decorative:

- **Scope** — `global` or one workspace. A workspace-scoped memory is not resolved for a caller that names no workspace.
- **Audience** — every Agent by default, or only the Agent ids named on the record.

Provenance is still recorded separately from both: **"recorded by" is not "readable by"**. A memory one Agent produced can be readable only by another, and neither field is exposed as recall tool input.

A memory saved in a session with no workspace folder is saved as `global` rather than rejected, and is readable from any workspace or none.

## Saving memories

- **OnePiece** exposes a memory tool to its own API tool-calling loop. What that tool produces is a **candidate**, not an active record — automatic paths propose, and a person decides.
- **CLI-wrapped Agents** do not expose this tool, because VaneHub does not control a CLI-wrapped Agent's own internal tool system. They produce candidates through the separate automatic-extraction mechanism, which rides on context compaction.
- **A memory the user writes themselves** is active immediately: the author is a person, so there is nobody left to review it.

Extraction for CLI-wrapped Agents runs through OnePiece's provider, because those CLIs expose no reusable model credentials. With no provider configured for OnePiece, they produce no extraction at all.

## Storage surfaces

The markdown file is authoritative; the SQLite projection, the `MEMORY.md` index, and the retrieval entry are derived and rebuildable. Writes go through the v2 application service — a generic file tool must not write into the memory directory, because that is what keeps the derived surfaces in step. See [Personalization governance](personalization-governance.md#memory-which-surface-is-authoritative).

## Where the design lives

This chapter orients contributors. The authoritative requirements live in the specs.

- [openspec/specs/agent-cross-session-memory](../../../openspec/specs/agent-cross-session-memory/spec.md) — the pool, provenance, and the saving paths.
- [openspec/specs/retrieval-vector-search](../../../openspec/specs/retrieval-vector-search/spec.md) — the recall tool and degradation.

Memory persistence and governance live in the `personalization` bounded context; recall lives in `retrieval`. See [Native bounded contexts](native-contexts.md).
