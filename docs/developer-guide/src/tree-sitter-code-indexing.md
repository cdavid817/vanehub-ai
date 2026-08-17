# Tree-sitter code indexing

Workspace code is parsed with Tree-sitter into bounded typed chunks and symbols. This is the local half of the `retrieval` bounded context — it runs without any external service and makes FTS available before embedding is confirmed. The shared host-level memory pool is a separate concern covered in [Retrieval and vector search](retrieval.md).

## Admitted code and error tolerance

Only admitted code is parsed, using the selected Tree-sitter grammar for each language. Parsing is tolerant: when a file contains syntax errors, the system indexes only bounded chunks derived from valid named subtrees around the errors. Erroneous subtrees are not indexed.

## Bounded typed chunks

Each chunk is persisted with: workspace id, normalized relative path, language, line range, symbol name, symbol kind, chunk key, and index version. Symbol definition metadata (e.g. a function or class definition's name, kind, and definition range) is persisted during the same file transaction, so a symbol is discoverable alongside its chunks.

## Chunk budget and splitting

A single symbol larger than the configured chunk budget is split into multiple chunks. Every resulting chunk remains attributable to its source symbol and file range.

## Redaction before persistence

The unified sensitive-information policy is applied to admitted code before any chunk text is persisted, embedded, logged, audited, or returned from `search_code`. Raw code content is not duplicated into retrieval storage. A chunk containing a sensitive value carries a redacted marker instead of the value.

## Index version and staleness

A code index version covers grammar compatibility, Tree-sitter queries, chunking, and redaction policy. A version mismatch marks affected workspace files stale and rebuilds them in bounded batches. The native worker performs metadata-first reconciliation and reads or parses only new or changed files.

## Where the design lives

This chapter orients contributors. The authoritative requirements live in the spec.

- [openspec/specs/workspace-code-indexing](../../../openspec/specs/workspace-code-indexing/spec.md)

The `retrieval` bounded context that owns this is described in [Native bounded contexts](native-contexts.md); the shared memory pool half is in [Retrieval and vector search](retrieval.md).
