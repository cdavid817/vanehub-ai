## Context

The existing `retrieval` context indexes one `RetrievalDocument` per agent memory. `IndexingService`, `SearchService`, and the worker are wired to a single full-snapshot `AgentMemoryIndexSource`; repository methods accept `SourceKind`, but `SourceKind` only contains `AgentMemory`, status and rebuild are global, and candidate queries intentionally ignore `scope_folder` because memories form a host-wide shared pool.

Code has different invariants. A workspace can contain thousands of mutable files, one file produces multiple chunks and symbols, code results require source locations, and no result may cross the current session's workspace. The workspace context currently identifies known projects by path rather than by a stable id. The desktop already depends on `ignore` and `globset`, and the agent runtime already exposes a bounded workspace-relative `read_file` tool.

This change spans retrieval, workspaces, agent runtime, SQLite, Tauri commands, frontend service adapters, and settings UI. Code parsing and indexing remain native desktop responsibilities. The Web runtime provides contract-compatible deterministic mock behavior and performs no filesystem or embedding network access.

## Goals / Non-Goals

**Goals:**

- Establish a safe, workspace-scoped code-indexing foundation using Tree-sitter chunks and symbols.
- Preserve the existing global agent-memory contract while making indexing and search source-aware.
- Avoid full content snapshots during routine reconciliation by persisting a file manifest and processing only changed paths.
- Prevent sensitive files and detected secret values from reaching retrieval storage, logs, or an embedding provider.
- Return typed code locations and reuse the existing `read_file` tool for context expansion.
- Provide per-workspace configuration, cost confirmation, progress, rebuild, delete, retention, and Web/mock parity.

**Non-Goals:**

- Filesystem event watching, debounce, or `notify` integration.
- Concurrent parsing or embedding of multiple workspaces; the first scheduler is bounded and round-robin.
- A public `search_symbols` tool, call graph, reference graph, or automatic context expansion.
- Local ONNX embedding, model download, or model lifecycle management.
- Remote workspace indexing, dynamically downloaded grammars, or a reduced-parser distribution variant.
- Automatically inferring that a moved directory is the same workspace.

## Decisions

### 1. Retrieval owns indexed-workspace state behind cross-context ports

`retrieval` will own code-index configuration, manifests, chunks, symbols, queues, and search because they exist solely for retrieval. It will define consumer-side `WorkspaceCatalogPort` and `WorkspaceFilePort` contracts. Bootstrap adapters will resolve local session roots through the `workspaces` API and perform bounded filesystem access without importing workspaces infrastructure into retrieval.

React components will use additions to `AgentService`. `tauri-agent-client.ts` will be the only frontend caller of the new Tauri commands, and `web-agent-client.ts` will maintain an in-memory mock with the same contract shape.

Alternative: put scanning and manifests in `workspaces`. Rejected because it would make the workspace context own Tree-sitter, embedding admission, and retrieval-specific lifecycle.

### 2. Stable workspace ids are separate from canonical roots

The first time a local root is configured, retrieval creates an opaque UUID `workspace_id` and stores its canonical root as the current locator. Session paths are canonicalized before resolving them to an indexed workspace. The id scopes all file, chunk, symbol, status, and audit rows.

A missing root marks the workspace unavailable without deleting data. A different root is a different workspace until the user explicitly deletes or rebuilds it; automatic path rebase is deferred because path similarity is not proof of identity.

Alternative: use `scope_folder` or a hash of the path as identity. Rejected because path casing, Windows extended-path normalization, worktrees, and directory moves make it unstable.

### 3. Persist configuration, file manifests, chunk metadata, and symbols separately

The migration adds logical tables equivalent to:

- `code_index_workspaces`: id, canonical root, display name, enabled, selected relative roots, enabled languages, exclusion patterns, maximum bytes, index version, state, and timestamps.
- `code_index_files`: workspace id plus normalized relative path, language, byte size, modified fingerprint, raw file content hash, index version, state, failure category, chunk count, and timestamps.
- `code_index_chunks`: retrieval document id, workspace id, relative path, language, byte/line range, symbol name/kind, chunk ordinal/key, redaction count, and index version.
- `code_index_symbols`: workspace id, normalized name, display name, kind, container, relative path, and definition range.
- `code_index_audit`: bounded local metadata for admission, skip, index, rebuild, and deletion events without file content.

`retrieval_documents` continues to own searchable text, FTS rows, embedding state, model, and vector. `SourceKind::WorkspaceFile` identifies code chunks. Its `source_id` is globally deterministic from `workspace_id`, normalized relative path, and chunk key. Companion chunk rows provide efficient scope filters and typed projection.

Foreign keys or transactional repository operations remove a file's stale chunks and symbols when a file changes or disappears. Existing agent-memory rows require no rewrite.

Alternative: encode metadata in a text prefix. Rejected because it weakens scope filtering, ranking, migrations, and typed tool output.

### 4. The admission pipeline has fixed precedence

Files pass these gates before content is parsed:

1. canonical path remains inside the configured workspace and selected relative roots;
2. the walker respects nested `.gitignore` rules and does not follow escaping symlinks;
3. a mandatory, case-normalized sensitive-file denylist rejects `.env` variants, credentials, private keys, certificates, common credential directories, and equivalent patterns;
4. validated user exclusion globs reject matching relative paths;
5. extension/language selection accepts only an enabled parser;
6. metadata size is at or below the configured byte limit and the file is non-binary.

Mandatory rules cannot be negated by user patterns. Invalid globs reject configuration updates rather than silently weakening filtering. Skip reason codes and counts are stored, but raw private paths are excluded from unified logs.

The runtime compiles parsers for JavaScript, TypeScript/TSX, Python, Rust, Go, Java, C, and C++. Runtime selection reduces traversal and parsing work but does not change binary size.

### 5. Store and return redacted chunks

The parser reads an admitted file into bounded memory and computes its raw SHA-256 content hash. A shared sensitive-information policy redacts detected token, password, private-key, authorization, and credential assignments before chunk text enters `retrieval_documents`, FTS, embedding input, audit detail, or tool output. Only the one-way raw content hash is retained for change detection.

`search_code` returns a bounded redacted snippet. The model can request surrounding current source with the existing workspace-bounded `read_file(path, offset, limit)` tool. This avoids adding a duplicate filesystem tool while ensuring the index itself is not a second raw-code store.

Alternative: retain raw local FTS text and redact only embedding input. Rejected because it duplicates secrets in retrieval storage and can leak them through search results.

### 6. Reconciliation is file-manifest based

Initial enablement performs a metadata inventory through `ignore::WalkBuilder`. A routine audit also enumerates admissible paths, but only reads, hashes, parses, and replaces files whose manifest fingerprint changed. Targeted `reconcile_paths(workspace_id, paths)` processes explicit create/change/delete/rename sets without enumerating the workspace. Deleted manifest paths remove their chunks and symbols transactionally.

The raw content hash is the final no-op check, so a metadata change that preserves content does not requeue embeddings. A low-frequency metadata inventory remains the recovery mechanism for changes made while the application was stopped and for future watcher overflow.

Alternative: extend the existing full `snapshot()`. Rejected because it clones all file contents and recomputes all hashes every cycle.

### 7. Parsing produces deterministic bounded chunks

Tree-sitter query files identify definitions and named symbols per supported language. A symbol definition is the preferred chunk boundary. Oversized symbols split on named child nodes and then bounded line windows; small adjacent top-level nodes can combine up to the embedding budget. Every chunk includes its definition range and a bounded amount of structural context such as signature and container name.

Chunk keys are deterministic within a file using symbol kind/name occurrence and fallback ordinal. Reconciliation replaces all chunks for a changed file in one transaction, so obsolete chunks cannot survive parser output changes. Syntax errors produce best-effort chunks from valid subtrees; an entirely failed parse records a bounded failure category and does not embed raw fallback content.

`CODE_INDEX_VERSION` covers chunking policy, redaction policy, and Tree-sitter query/grammar compatibility. A mismatch marks only that workspace's files stale and requeues them in bounded batches.

### 8. Source-aware retrieval preserves memory semantics

Indexing and repository APIs receive an explicit source kind and scope. Agent memory uses `GlobalMemoryScope` and retains host-wide recall. Workspace code uses `WorkspaceScope(workspace_id)` for reconcile, pending claims, vector candidates, FTS candidates, status, requeue, rebuild, and delete.

`search_code` is a separate agent-runtime port and tool. Its schema exposes `query` and `limit`, never workspace id or folder. The generation adapter supplies the canonical current session workspace. Results contain `file_path`, `start_line`, `end_line`, `language`, optional `symbol_name` and `symbol_kind`, `snippet`, and `matched_via`.

The tool is absent when the session has no local workspace or indexing is disabled. Local keyword search remains available while embedding is awaiting confirmation or temporarily unavailable and reports `degraded: keyword_only`. Existing `recall` registration and payload do not change.

Alternative: return a union from `recall`. Rejected because recall's current specification is a global memory pool with no scope input, while code requires a hard implicit scope.

### 9. External embedding is explicitly confirmed and rate limited

Scanning, parsing, symbols, and local FTS complete before external embedding starts. The UI shows admitted file count, exact chunk/input count, estimated batch requests at the current batch size, provider profile, and model. The user must confirm the first external embedding run for a workspace and confirm again after changing provider/model or re-enabling embedding following deletion.

The worker allows one in-flight embedding request per configured provider profile, applies a configurable minimum inter-batch interval, honors bounded `Retry-After`, and uses existing retry categories and hard HTTP timeouts. It processes workspace queues round-robin so one large workspace does not permanently starve another, without claiming parallel indexing support.

Cancellation is cooperative: disabling/deleting a workspace or shutting down prevents new file and batch claims, invalidates queued generation tokens, and checks cancellation between files and batches. An in-flight blocking HTTP request may finish but its result is discarded when its generation token is stale.

### 10. Status is phased and workspace scoped

Status distinguishes `disabled`, `scanning`, `parsing`, `awaiting_embedding_confirmation`, `embedding`, `ready`, `degraded`, `cancelling`, and `unavailable`. It reports discovered/admitted/skipped/processed files, total/indexed/pending/failed chunks, redaction count, estimated requests, current phase, and timestamps. ETA is optional and only shown when the denominator and observed throughput are stable.

Workspace configuration and management live with workspace UI; the OnePiece retrieval section retains global embedding provider/model selection and adds aggregate code-index visibility. Tauri may emit progress events, while query invalidation/polling remains the correctness fallback. Web/mock returns deterministic phase transitions without filesystem or network work.

### 11. Retention and diagnostics are explicit

Closing the last active view or disabling indexing stops new work but retains manifests and index rows. Rebuild invalidates the selected workspace and preserves configuration. Delete removes that workspace's manifests, symbols, chunks, vectors, audit rows, and confirmation state after user confirmation.

Native diagnostics use the unified logging service and include only workspace id, source kind, phase, counts, durations, model id, and safe reason categories. File-level audit data stays in SQLite for local UI inspection; it is not copied to unified logs or telemetry.

## Risks / Trade-offs

- [Tree-sitter grammars increase desktop size and build time] -> Pin grammar versions, compile only the initial eight parser families, and measure release artifacts; distribution variants remain separate work.
- [Secret detection has false positives and cannot prove all secrets are removed] -> Combine a non-overridable filename denylist, bounded content redaction, explicit external-provider confirmation, and redacted output; document that no heuristic is complete.
- [Without notify, updates are not second-level real time] -> Reconcile on enable, open, manual refresh, targeted application events, and periodic manifest audit; add notify/debounce in the follow-up change.
- [mtime/size fingerprints can miss adversarial same-metadata edits] -> Content hash every targeted or fingerprint-changed file and expose rebuild; future watcher events force hashing even when metadata matches.
- [Round-robin single-worker indexing can be slow] -> Preserve bounded resource use and workspace fairness first; introduce measured concurrency separately.
- [Code vectors greatly increase brute-force candidate cost] -> Filter by workspace in SQL before vector deserialization, cap result candidates, and measure thresholds before considering ANN.
- [Stale in-flight embedding responses can race cancellation] -> Tag claims with a workspace generation token and reject stores from stale generations.

## Migration Plan

1. Add the new tables, indexes, source kind parsing, and companion metadata without changing existing agent-memory rows.
2. Add source-aware repository and application APIs, then keep the existing memory adapters on `GlobalMemoryScope` with regression tests.
3. Add workspace configuration and scanning behind a disabled-by-default setting; no code content is read during migration.
4. Add code search and UI only after scope-isolation, redaction, and Web/mock contract tests pass.
5. Rollback disables code indexing and removes tool registration while leaving additive tables ignored; existing memory retrieval remains operational. A later forward migration can clean retained code tables if required.

## Open Questions

- The exact default maximum file size will start at 100 KiB unless product testing establishes a better limit.
- The minimum inter-batch interval and per-provider request budget require measurement against supported OpenAI-compatible providers; they must be configurable constants rather than UI controls in this change.
- Whether file-level audit paths should be shown in full or workspace-relative form in the UI requires a privacy review; storage and diagnostics will use normalized relative paths only.
