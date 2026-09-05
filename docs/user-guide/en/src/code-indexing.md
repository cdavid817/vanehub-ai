# Code indexing

Workspace code indexing lets OnePiece locate definitions and relevant code without repeatedly scanning every file. Open **Settings → Agent configurations → OnePiece**, configure an OpenAI-compatible embedding source, then use **Workspace code indexes**.

## Configure an index

1. Select **Add workspace** and choose a local folder.
2. Open its configuration and enable indexing.
3. Choose the relative roots and languages to index. Supported languages are JavaScript, TypeScript/TSX, Python, Rust, Go, Java, C, and C++.
4. Add one exclusion glob per line and set the per-file size limit. The default limit is 100 KiB.
5. Save, refresh the inventory, and review the file and chunk estimates.
6. Confirm the exact workspace, provider profile, model, chunk count, and estimated batch requests before external embedding begins.

Indexing is disabled per workspace by default. Selected roots, languages, exclusions, and the size limit are independent for every workspace.

## Privacy boundary

VaneHub applies nested `.gitignore` rules and your exclusions before parsing. Mandatory sensitive-file patterns such as `.env`, credential files, private keys, and PEM files cannot be overridden. Admitted chunks are scanned and redacted before embedding, but redacted code is still transmitted to the selected external embedding provider after confirmation.

Confirmation is bound to the workspace generation, provider profile, and model. Rebuilding the index, changing its configuration, or switching the embedding profile/model requires a new confirmation. Keyword search remains local and workspace-scoped while confirmation is pending.

## Retention and removal

Disabling an index retains its configuration and stored index for reuse. **Rebuild** removes that workspace's file manifests, chunks, symbols, and vectors, then scans it again. **Delete** permanently removes its configuration, index data, and local audit records without changing other workspace indexes or agent memories.

Closing a workspace does not delete its index. If its root is moved or unavailable, the status shows **Root unavailable**; register the new path and rebuild rather than assuming the old identity follows the folder.

## Tree-sitter index and live LSP

The persistent Tree-sitter index powers the `search_code` tool for structural, keyword, and optional semantic retrieval. It does not provide compiler-aware types, references, or current diagnostics. Live LSP code intelligence is an independent desktop capability that starts a trusted language server and exposes position-based semantic tools. You may enable either capability without enabling the other.

See [Use live LSP code intelligence](lsp-code-intelligence.md) for supported servers, setup, trust, and troubleshooting.

## Related

- How the live semantic capability and this page's persistent index divide the work → [LSP code intelligence](lsp-code-intelligence.md)
- Cross-session memory and retrieval methodology → [Memory and context](memory-and-context.md)
- The parsing technology itself: GLR incremental parsing, the query system, structured code chunking, and repo maps → [Tree-sitter technical architecture](../../../agent-infrastructure/patterns/tree-sitter.md) (Simplified Chinese)
- Retrieval pipelines and hybrid-retrieval trade-offs → [RAG technical architecture](../../../agent-infrastructure/patterns/rag.md) (Simplified Chinese)
