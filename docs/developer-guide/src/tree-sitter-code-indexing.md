# Tree-sitter code indexing

Workspace code is parsed by Tree-sitter into bounded chunks and symbol metadata, landing in a persistent per-workspace index. This is the local half of the `retrieval` bounded context — **the local pipeline runs with no external service at all**; only the optional semantic-enhancement pipeline involves external vector embeddings. The cross-session memory pool is a separate concern — see [Retrieval and vector search](retrieval.md).

Indexing is **disabled by default**: a `CodeWorkspace` starts with `enabled: false` and phase `Disabled`, and the OnePiece automatic policy also defaults to `Disabled` (no automatic workspace registration). Remote (SSH) workspaces are **not currently supported**: workspace discovery returns nothing for remote sessions, and registration depends on local path canonicalization.

## The two pipelines

### The local pipeline (no external dependencies)

```text
File inventory (ignore walker, honors .gitignore)
    → workspace boundary (canonicalize + relativize; absolute paths and .. rejected)
    → file admission (below)
    → language identification (by extension)
    → Tree-sitter parsing (tolerates syntax errors)
    → symbol extraction (.scm queries) + chunk generation (budgeted splitting)
    → redaction
    → FTS and metadata persistence (one file transaction)
```

The local pipeline ends at the FTS5 full-text index plus chunk/symbol metadata; `search_code` works from here alone.

### The semantic-enhancement pipeline (optional, explicit confirmation required)

The backend keeps a single phase field; the frontend derives two channel states from it. The semantic channel's derived states are `not_applicable` (local mode — semantics not requested), `disabled`, `pending` (scanning/parsing), `unconfigured` (no vector embedding configured), `awaiting_confirmation` (waiting for the external-embedding confirmation), `embedding`, `ready`, and `degraded` (`src/services/code-index-contract.ts`).

**Local mode never passes through embedding confirmation**: reconciliation lands directly on `Ready`/`Degraded`, and the worker skips `prepare_code_embedding` for local workspaces. Only semantic mode enters `AwaitingEmbeddingConfirmation` after parsing completes.

## File admission rules and their priority

Admission is decided in order by `admit_metadata` (`domain/code_admission.rs`); `.gitignore` filtering happens earlier in the walker, and symlinks, non-regular files, and dedup are handled at the scan layer:

1. **Selected roots** — `selected_roots` supports multiple index roots per workspace (an empty string means the workspace root); outside them → `OutsideSelectedRoots`;
2. **Mandatory sensitive paths** (`is_mandatory_sensitive_path`, not overridable by user configuration, path lowercased first): directory components `.ssh`/`.aws`/`.azure`/`.gcp`/`.kube`/`credentials`/`secrets`; filenames `.env` and `.env.*`, `credentials(.json)`, `application_default_credentials.json`, `id_rsa`/`id_dsa`/`id_ecdsa`/`id_ed25519`, `.netrc`; extensions `key`/`pem`/`p12`/`pfx`/`jks`/`keystore`/`crt`/`cer`/`der`;
3. **User exclusion globs** — at most 128 patterns of ≤256 characters; a pattern without `/` matches as `**/<pattern>`;
4. **Language** — unknown extension or a disabled language → `LanguageDisabled`. The enabled languages are **eight** enum variants: JavaScript, TypeScript, Python, Rust, Go, Java, C, C++ — TSX is not a separate language; TypeScript selects the TSX grammar for `.tsx` files;
5. **Size** — default `DEFAULT_MAX_FILE_BYTES = 100 KB` (configurable up to 10 MB, never 0);
6. **Binary sniffing** — a `\0` in the first 8 KB marks the file binary and skips it.

Skips are not failures; they are counted and aggregated into a `Skipped` audit entry.

## Parsing, symbols, and chunks

**Parsing tolerates syntax errors — "any syntax error fails the whole file" is false.** `load_and_parse` never checks the tree for ERROR nodes: with recoverable syntax errors, symbols and chunks are still extracted from the valid named subtrees. Keep four levels apart:

- **Local node skips** — only named nodes matching the `.scm` queries become symbols; ERROR regions naturally produce no symbol chunks;
- **File-level parse failure** — exactly five kinds: unreadable, over the size limit, invalid UTF-8, grammar initialization failure, parser failure (none of which concern syntactic correctness); a failure is counted and the next file proceeds;
- **File admission skips** — not failures (previous section);
- **Whole-index degradation** — `failed > 0` in a round → phase `Degraded`; in semantic mode a failure also skips that round's embedding.

**Symbols are optional — not every chunk has one.** The chunk persistence type carries `symbol_name: Option<String>` and `symbol_kind: Option<String>`; a file with no symbol matches produces whole-file fallback chunks (`chunk_key = "fallback:<part>"`, empty symbol fields). Symbol definition metadata (name, kind, definition range) persists in the same file transaction as the chunks; a symbol's `container_name` is derived afterwards from containment — the `.scm` queries themselves produce no containers.

A symbol over budget (`DEFAULT_MAX_CHUNK_BYTES = 6 KB`, passed as a fixed value by the orchestrator — not configurable) is split on named child-node boundaries, and every resulting chunk still traces back to its source symbol and file range.

> **Spec gap**: the `workspace-code-indexing` spec requires that an unparsed whole-file fallback never be embedded, but the current implementation generates whole-file fallback chunks for symbol-less files and writes them into the index and the embedding queue. This is a pending spec-versus-implementation decision (converge the implementation or revise the spec).

## The security boundary: redaction

The accurate statement is: **the parser reads raw code, but an unredacted chunk must never be written to the retrieval index, sent to external embedding, recorded in the unified log, or returned as a search result.** In the implementation:

- The single construction point of chunk content redacts first (`code_chunker.rs`); the persistence entry redacts again and computes `content_hash` over the redacted text written to `retrieval_documents.content`; FTS is fed from that column by triggers, so FTS also indexes redacted text;
- Embedding reads exactly those redacted rows; search-result `snippet`s come straight from that column, never re-reading the original file;
- Reconciliation and batch logs record only workspaceId, phase, generation, counts, durations, and model; audits store only normalized relative paths and reason categories.

Redaction is **known-pattern detection over six regex classes, not full DLP**: PEM private-key blocks, quoted sensitive assignments (keywords like `api_key`/`token`/`password`), unquoted assignments of the same keywords, `bearer` tokens, provider token prefixes (`sk-`, `ghp_`, `github_pat_`, `AKIA…`), and internal URLs (localhost/private ranges). Matches become `[REDACTED]` and increment `redaction_count`. If regex compilation fails, redaction **fails closed**: the whole content is replaced with `[REDACTED]` rather than leaking the original. The mandatory sensitive-path denylist additionally keeps the highest-risk files out of parsing entirely.

## Index version, manifest, and reconciliation

- `CODE_INDEX_VERSION` (currently `"1"`) covers grammar compatibility, Tree-sitter queries, chunking, and redaction policy; files with a mismatched version are marked stale and rebuilt (loading a workspace that detects a stale version synchronously clears its file rows and bumps the generation — the read path has this implicit side effect).
- The manifest (`CodeFileManifest`) records path, `content_hash`, mtime, size, and index version; reconciliation compares size+mtime+version first, then the content hash, skipping unchanged files outright (metadata-first, reading only new or changed files).
- **Targeted reconciliation** — after a successful Agent file write, `notify_targeted_change` feeds a bounded coalescing queue (512 paths per pass). `CodePathChange` defines `Upsert`/`Delete`/`Rename` (Rename expands to delete-old + add-new), but **the production chain currently submits only `Upsert`** — there is no filesystem watcher, and the Delete/Rename variants are constructed only by tests (annotated as dead code reserved for the follow-up watcher). A path that no longer passes admission or no longer exists is treated as a delete.
- **Cancellation** — the `reconcile_*_cancellable` variants exist but are not wired in production (the non-cancellable wrappers are called); actual interruption relies on generation-drift checks.

### Disable, rebuild, and delete are three different operations

| Operation | Behavior |
| --- | --- |
| **Disable** | Only `enabled=false`, phase → `Disabled`, generation+1, embedding confirmation cleared; **data kept** |
| **Rebuild** | Deletes all file rows (cascading chunks/symbols/documents), generation+1, phase back to `Scanning`, confirmation cleared, `Rebuilt` audit recorded |
| **Delete index** | Deletes the workspace row itself; everything goes |

There is also **refresh**: run one reconciliation synchronously and return the status.

## External-embedding confirmation

In semantic mode, after parsing the index stops at `AwaitingEmbeddingConfirmation` until the user explicitly confirms. Confirmation binds the triple **profile_id + model + generation** (the workspace is implied by the row); all three must match at three checkpoints: the embedding-entry decision, the batch guard, and the vector-search precondition. The confirmation dialog shows provider/profile, model, total chunks, and the estimated embedding requests (`total_chunks.div_ceil(32)` — this is the network and cost impact).

**There is no standalone "revoke confirmation" command.** Confirmation is implicitly invalidated (three columns nulled, generation+1) by three paths: saving configuration, rebuilding, and index-version invalidation. After invalidation the phase returns to awaiting confirmation, in-flight batches are dropped by the guard, and semantic search degrades to keyword-only.

Embedding failure retry: at most 5 attempts per item with backoff `[1, 4, 15, 60, 300]` seconds; auth/invalid-request errors give up immediately, network errors give up at the attempt cap. Content is truncated to 8,000 characters before embedding.

## The `search_code` tool contract

- Input is **exactly** `query` (required) + `limit` (optional, default 5, clamped 1–20), `additionalProperties: false`; a dedicated test pins this shape.
- **The workspace is determined implicitly by the trusted runtime** (the session's workspace folder); the model cannot name a workspace or path root. The tool enters the catalog only when that workspace's index is enabled and its phase is not `Unavailable`.
- Result entries carry `file_path`, `start_line`, `end_line`, `language`, `symbol_name` (nullable), `symbol_kind` (nullable), `snippet` (redacted text), and `matched_via`, with an optional top-level `degraded`.
- Retrieval is RRF fusion of the vector and FTS keyword paths (over-fetch `limit×4`); **local mode has no vector channel and is not marked degraded** (no local embedding is the expected state); semantic mode marks `keyword_only` when vectors are missing, `vector_only` when keywords fail, and returns a soft "temporarily unavailable" result when both fail.
- **Search snippets do not replace precise reads**: a snippet is bounded, redacted index text; use the file-read tool when exact content is needed.

## Where the design lives

The authoritative requirements live in the spec; this chapter describes the current implementation and records the gaps.

- [openspec/specs/workspace-code-indexing](../../../openspec/specs/workspace-code-indexing/spec.md)

Known spec-versus-implementation gaps (besides the fallback conflict above): the spec's four targeted-reconciliation kinds (created/modified/renamed/deleted) versus a production chain that submits only Upsert (no watcher); the spec's cooperative cancellation channel, which is unwired (generation checks only); and the spec's Retry-After rate-limit honoring, for which no header parsing exists in the embedding adapter. The spec's Purpose section is still archive placeholder text awaiting a real statement.

The owning `retrieval` bounded context is described in [Native bounded contexts](native-contexts.md); the cross-session memory half in [Retrieval and vector search](retrieval.md); the responsibility comparison with LSP in [LSP code intelligence](lsp-code-intelligence.md).
