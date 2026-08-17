# Cross-session memory

Stored memories are a single host-level pool shared by every Agent — OnePiece and all CLI-wrapped Agents (`claude-code`, `codex-cli`, `gemini-cli`, `opencode`, `antigravity-cli`) alike. They are not scoped to the Agent or workspace folder that produced them. This is the persistence half of the `retrieval` bounded context; the recall/search path is covered in [Retrieval and vector search](retrieval.md).

## Shared host-level pool

When a memory is saved during a session with a workspace folder, the producing Agent id and that workspace folder are recorded as **provenance metadata** on the stored record, not used as a filter for injection, listing, or management. Consequences:

- A memory saved under one Agent is visible to every other Agent's generations and management views, exactly as if they had produced it.
- A memory saved in a session with no workspace folder is still saved into the shared pool (no folder recorded, not rejected), and is readable/injectable/manageable from any workspace or none.
- Agent id and workspace folder are provenance only; recall does not restrict by them and does not expose them as recall tool input.

## Saving memories

- **OnePiece** exposes a memory-saving tool to its own API tool-calling loop. While the memory enablement toggle is on, the tool is auto-approved — it persists immediately without user confirmation.
- **CLI-wrapped Agents** do not expose this tool, because VaneHub does not control a CLI-wrapped Agent's own internal tool system. They produce memories through a separate automatic-extraction mechanism governed by their own requirement.

## Where the design lives

This chapter orients contributors. The authoritative requirements live in the specs.

- [openspec/specs/agent-cross-session-memory](../../../openspec/specs/agent-cross-session-memory/spec.md) — the shared pool, provenance, and the saving paths.
- [openspec/specs/retrieval-vector-search](../../../openspec/specs/retrieval-vector-search/spec.md) — the recall tool and degradation.

Memory persistence and recall live in the `retrieval` bounded context; see [Native bounded contexts](native-contexts.md).
