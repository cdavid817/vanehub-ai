# Index workspace code

Workspace code indexing lets OnePiece locate definitions and relevant code without repeatedly scanning every file. Open **Settings > Agent configurations > OnePiece**, configure an OpenAI-compatible embedding source, then use **Workspace code indexes**.

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
