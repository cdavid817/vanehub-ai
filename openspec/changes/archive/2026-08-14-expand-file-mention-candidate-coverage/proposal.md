## Why

The chat composer's `@` file-reference completion is fed by `listSessionDocuments`, the same data source that backs the Documents tab. That source is deliberately bounded to Markdown and text documents because `session-workspace-tabs` requires the Documents tab to list "bounded Markdown and text documents". `chat-experience` asks for something different — "bounded **file** candidates" under the session root — but inherits the Documents tab's type restriction through the shared source.

The practical result is that `@` cannot reference source files at all. Typing `@utils.rs` returns nothing, because `.rs`, `.ts`, `.py` and every other code extension are discarded during document discovery. The scan is additionally capped at 300 entries with no exclusion for dependency or build directories, so on a real project the cap is consumed by `README.md` files inside `node_modules` before any first-party document is reached. File referencing — the feature's entire purpose — is unusable for code.

This change decouples the two consumers and gives chat mention its own query-driven candidate source.

## What Changes

- **New `search_files` Tauri command** — session-scoped, query-driven file search over the active session root. Accepts a query and a result cap, traverses breadth-first with a bounded depth, and returns ranked matches instead of an unranked prefix of the workspace.
- **Relevance ranking** — candidates are scored so that exact filename matches outrank prefix matches, which outrank substring matches, which outrank multi-segment path matches. Only the top results cross the service boundary.
- **Candidate coverage widened to source files** — the mention candidate set covers common source and configuration extensions rather than only Markdown and plain text.
- **Dependency and build directories excluded** — traversal for mention candidates skips vendored and generated trees (dependency installs, build outputs, compiler caches, virtual environments) so the result budget is spent on first-party files. Dot-prefixed entries remain excluded as they are today.
- **Chat composer switches candidate sources** — `fileReferenceCandidates` is served by the new search command through the frontend service boundary, replacing client-side substring filtering over a preloaded document list.
- **Documents tab is untouched** — `listSessionDocuments` keeps its current Markdown/text bounds and its existing traversal. Its requirement in `session-workspace-tabs` is unchanged, and this change explicitly stops the chat composer from depending on it.

Not in scope: line-range references, the file preview modal, drag-and-drop, and clipboard file paste. Those build on this change and are proposed separately.

## Capabilities

### New Capabilities

None. The behavior belongs to the existing chat composer capability.

### Modified Capabilities

- `chat-experience`: The "Chat file references" requirement gains explicit bounds for the candidate set — which files are eligible, which directory trees are excluded, how results are ranked and capped, and that the candidate source is independent of the Documents tab. The existing scenarios for chip display, prompt injection, unsafe-reference rejection, and reference metadata persistence are unaffected.

## Impact

**Runtimes:** Both. The desktop runtime gains the native search implementation; the Web runtime needs the matching mock-adapter behavior so browser mode keeps working.

**Adapter boundary:** No change to the isolation rules. The new capability is reached through `agent-service.ts` and implemented twice — `tauri-agent-client.ts` invokes the Rust command, `web-agent-client.ts` provides the mock equivalent. React components gain no direct `invoke()` usage.

**Native layer:**
- New command module under `src-tauri/src/commands/workspaces/`, registered in the command registry.
- New traversal and scoring logic in the workspaces context; `collect_documents` and `list_session_documents` are left alone.
- Path containment, oversize, and binary safeguards continue to apply — search returns candidates only; `compose_prompt` remains the single place that reads file content into a prompt.

**Frontend:**
- `agent-service.ts` gains the search method; both clients implement it.
- `use-main-layout-model.ts` stops deriving `fileReferenceCandidates` from `documentsQuery` and issues a query-scoped request instead.
- `ChatInputBox.tsx` is at 188 of the 300-line limit. Mention completion state moves into a hook so this change and the follow-ups stay under the cap without adding an ESLint exemption.

**Contracts and CI:** The new command must be registered in `src/contracts/` so `npm run contracts:check` passes. Existing tests that stub `listSessionDocuments` for composer completion need updating to the new source.

**No breaking changes.** Persisted messages, `file_references` storage, and prompt injection are unmodified; existing references keep resolving.
