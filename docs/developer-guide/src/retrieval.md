# Retrieval and vector search

The `retrieval` bounded context owns two distinct searches: a host-level cross-session **memory** pool (vector + FTS), and a per-workspace **code index** (Tree-sitter + FTS + vectors). Both degrade gracefully and never fail a generation because of a search error.

## Shared host-level memory pool

Retrieval searches the same host-level memory pool that recency-based memory injection draws from (`agent-memory-shared-pool`). Recall is **not** restricted by agent id or workspace folder. Agent id and workspace folder are recorded on an index row as provenance only and are not exposed as recall tool input:

- A memory saved under a different agent is recallable from any agent's session.
- Recall never returns a strict subset of what memory injection already placed in the system prompt.
- The recall tool input schema exposes exactly `query` and `limit` — no agent id, folder, or scope parameter, because the shared pool has no slice for the model to name.

## Graceful degradation

Retrieval failure never fails a generation. The tool returns a successful result describing unavailability:

- Embedding provider unreachable during search → keyword-only results marked `degraded: keyword_only`.
- FTS5 query fails → vector-only results marked `degraded: vector_only`.
- Both paths execute and neither returns a hit → an empty result list, not an error.

## Workspace code index

The persistent code index is workspace-scoped: workspace identity, file manifests, chunks, symbols, vectors, and bounded local audit records. The native worker performs metadata-first reconciliation and reads or parses only new or changed files. Tree-sitter grammars, chunking queries, and redaction policy share a version marker. Workspace-code embedding is gated by an explicit confirmation tied to workspace id, generation, provider profile, and model. FTS remains workspace-scoped and available before confirmation; vectors from another workspace or model are never candidates.

## Where the design lives

This chapter orients contributors. The authoritative requirements live in the specs.

- [openspec/specs/retrieval-vector-search](../../../openspec/specs/retrieval-vector-search/spec.md) — shared memory pool, recall tool, degradation.
- [openspec/specs/workspace-code-indexing](../../../openspec/specs/workspace-code-indexing/spec.md) — workspace code index, reconciliation, embedding confirmation.

The `retrieval` bounded context is described in [Native bounded contexts](native-contexts.md).
