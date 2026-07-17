## Context

The main workspace currently renders the active session transcript and composer in the center panel, while the right information panel keeps lightweight Agent Info, Files, and Changes panes mounted. The center implementation lives in an already oversized `main-layout.tsx`; local project inspection, session log queries, and interactive shell operations are not exposed through `AgentService`. Messages already contain session ids, tool-use blocks, timestamps, and optional input/output token counts, which can seed the first Terminal and Report implementations without a new persistence model.

The change crosses React layout and state, the frontend service boundary, Tauri and Web adapters, Rust filesystem/process/logging code, dependencies, accessibility, themes, i18n, and E2E coverage. Desktop can access the local project, Git, unified logs, and a PTY. Browser mode cannot, so it must provide deterministic simulations without implying that local actions occurred.

## Goals / Non-Goals

**Goals:**

- Make Chat, Changes, Documents, Files, Terminal, Shell, Logs, and Report available in one session-scoped center workspace.
- Lazy-mount each tab once per selected session, preserve mounted tab state across tab switches, and explicitly clean up session-owned native resources on session changes.
- Retain the right information panel as a compact overview and use the center tabs for detailed inspection.
- Keep every native operation behind `AgentService`, `tauri-agent-client`, and Rust Tauri commands, with interface-compatible Web/mock behavior.
- Constrain filesystem and Git reads to the selected session root, use the existing redacted unified log as the only diagnostic source, and use a real PTY for desktop shell interaction.
- Keep both registered visual styles and synchronized zh-CN/en resources first-class, including accessibility text, dates, numbers, status text, and empty/error states.
- Document deliberate first-version limits and a concrete optimization path.

**Non-Goals:**

- Editing or saving project files from Files or Documents.
- Git staging, reverting, committing, branching, or conflict resolution.
- Persisting interactive shell scrollback or reconnecting to a shell after the user switches sessions.
- Adding a second SQLite log store or exposing unredacted native diagnostics.
- Treating Web/mock Shell as a real browser-side process executor.
- Adding rich document formats, raw HTML execution in Markdown, terminal multiplexing, or multiple shells per session in the first version.

## Decisions

### 1. Split the center workspace into session-tab components

`main-layout.tsx` will retain orchestration and the three-panel shell, while a new session workspace feature folder owns `SessionTabs`, `SessionTabBar`, shared loading/error/empty states, and one component per tab. Chat reuses `MessageList` and `ChatInputBox`; the composer is passed as Chat-only content rather than duplicated.

The container owns:

- `activeTab`, defaulting to `chat`;
- `mountedTabs`, initially containing only `chat`;
- a session-id effect that resets both values when the active session changes;
- panels that are created only after first activation and subsequently switched with Tailwind `hidden`/`block` classes;
- a horizontally scrollable tab bar at narrow center-panel widths.

The reset unmounts old-session panels. Any panel with native resources must perform cleanup during unmount. This makes keep-alive semantics explicit: state survives tab switching inside one session, but not session switching.

Alternative considered: preserve one mounted tab tree per session. It would improve rapid session switching but retain file caches, terminals, editors, and subscriptions for every visited session; that memory and process lifecycle is deferred.

### 2. Preserve the right information panel as compact context

The existing Agent Info / Files / Changes tabs remain exactly three compact overview tabs to preserve the current three-panel specification. Files and Changes in the center become full viewers. Both surfaces consume the same query keys and service results where practical so the duplication is presentational rather than a second data model.

Alternative considered: remove Files and Changes from the right panel. That would reduce duplication but expand this change into a broader information-panel redesign and invalidate an existing explicit requirement.

### 3. Extend one typed runtime boundary

New data contracts live in a focused session-workspace type module and are referenced by `AgentService`. Both adapters implement the same methods:

- list immediate directory children for a session-relative path;
- read bounded text content for a session-relative file;
- list bounded Markdown/text document candidates;
- get structured Git status and a structured diff for a selected file and source;
- list filtered/paged session log entries and export a filtered redacted log;
- create, input, resize, and kill one shell for the active session;
- subscribe to shell output/state events.

React components never import Tauri APIs. The Tauri adapter is the only frontend layer that invokes native commands or listens to Tauri events. The Web adapter returns deterministic data and implements a clearly labelled echo-style shell simulation.

Alternative considered: introduce a separate workspace service. The repository's current architectural rule explicitly directs React session components through `agent-service.ts`, so the first version extends that boundary and groups the added types/methods by name.

### 4. Use capability-specific native command modules

Native domain logic is grouped under session-tab and shell modules. Each Tauri command boundary lives in its own file under `src-tauri/src/commands/session_tabs/` or `src-tauri/src/commands/shell/`, keeping IPC signatures independently reviewable while shared parsers and managers remain in the domain modules. App setup registers a shell manager and every new command in the existing handler.

The native layer resolves the session root from SQLite using the session id. The frontend sends relative paths only; it does not nominate an arbitrary root. Rust canonicalizes the root and candidate, rejects parent traversal and out-of-root symlink targets, skips hidden entries, sorts directories before files, and enforces entry/content limits.

Git commands use explicit argument arrays, a fixed working directory, disabled external diff behavior, and the existing command-safety/audit facilities. No shell-form command string is built from a user path.

### 5. Use lazy filesystem browsing with bounded document discovery

Files loads only immediate children on folder expansion. A directory response has a bounded node count and communicates truncation. File preview refuses content over 1 MiB, detects binary content, decodes supported text safely, and presents a localized unsupported state.

Documents performs a bounded native scan for `.md`, `.markdown`, and `.txt` files under the session root, skipping hidden directories and out-of-root symlinks. Markdown renders without raw HTML; plain text uses a preformatted viewer. Selecting a document uses the same bounded file-content method as Files.

Alternative considered: eagerly return one recursive project tree. It is simpler for the frontend but can block on large repositories and makes symlink, depth, and node-count behavior harder to control.

### 6. Represent Git status and diffs explicitly

Git status models index and worktree state separately and includes untracked, deleted, renamed, and conflicted metadata. Diff requests identify `working` or `staged` source and return binary/oversized metadata plus files, hunks, and typed context/addition/deletion lines. The UI derives unified and split rows from the same structure. First-version untracked text files are represented as additions against an empty file within the same size limit; oversized or binary files show metadata without content.

Non-Git sessions render a localized empty state rather than treating the absence of Git as an error.

Alternative considered: expose raw `git diff` text and parse it in React. Structured parsing belongs in Rust so desktop and future non-mock adapters share semantics and React remains presentational.

### 7. Back desktop Shell with a real PTY

The desktop Shell uses a cross-platform PTY implementation and xterm-compatible frontend package. `shell_create` chooses the platform default shell and uses the canonical session root as its working directory; the frontend cannot provide a replacement executable. The manager maps a stable shell id to the child, PTY writer, and lifecycle metadata. Reader threads/tasks emit output events containing the shell id and session id. Resize updates the PTY dimensions rather than being a no-op.

Only one shell is created per mounted Shell tab. Switching sessions, archiving/deleting the owning session, closing the app, or explicitly disconnecting kills the child. `CD` changes to the session root through a safely quoted shell-specific command; `Clear` clears the terminal presentation without pretending to erase process history. Ctrl+C and terminal input are forwarded through PTY input.

Shell lifecycle and failures are persisted through the unified logging service with session context. Raw interactive commands and stdout are not persisted as diagnostics because they may contain arbitrary secrets; they remain visible in the page only.

Alternative considered: pipe a child process through stdio. It does not provide terminal modes, reliable prompts, interactive applications, signal behavior, or meaningful resizing, so it is inconsistent with an xterm UI.

### 8. Query and export the existing unified log file

The desktop log command reads the active unified JSON-lines log, deserializes only valid entries, filters by exact session id from redacted context, level, and case-insensitive search text, and returns a bounded newest-first page. Malformed lines are skipped without exposing or re-logging raw content; this avoids creating a repeated diagnostic entry every time a persistent malformed line is scanned.

Export writes the filtered, already-redacted entries to a user-selected destination through the native/service boundary. It never returns an unrestricted arbitrary source path or reads a feature-local file. Web mode returns deterministic redacted entries and reports local export as unavailable.

Alternative considered: mirror logs into SQLite. It creates two sources of truth, requires dual-write consistency and migration work, and conflicts with the unified log directory as the established persistence model.

### 9. Derive first-version Terminal and Report data from messages

Terminal displays every `toolUse` block as a command/tool execution card and its badge is the number of tool-use blocks for the selected session. Cards distinguish name, status, sanitized structured input/output, and parent message time. The label remains Terminal for the requested information architecture even though it is an execution history, while Shell is reserved for the interactive PTY.

Report aggregates the current session's messages into reported input/output token totals, separately labelled estimated character counts when token usage is absent, tool frequency, status counts, and a chronological message/tool/completion/failure timeline. The first version requests a bounded message history and indicates when the result is partial.

Alternative considered: add a new report table and backend aggregation immediately. The current message model already contains enough data for a useful first version; server-side aggregation is retained as a scaling optimization.

### 10. Treat theme, localization, and accessibility as contracts

All user-visible text, tooltips, aria labels, errors, empty states, and status labels use synchronized zh-CN/en keys. Dates and numbers use the current application locale. Tab buttons expose tab roles, selection state, keyboard navigation, and a translated label even when narrow layouts emphasize icons.

React styling uses Tailwind and existing semantic CSS variables for both `futuristic` and `minimal`. Xterm colors are derived from computed semantic CSS variables when the terminal is created or the application style changes; application components do not add inline React styles.

## First-Version Implementation Boundary

The concrete implemented bounds, Web/mock fixture inventory, and current optimization backlog are maintained in `implementation-notes.md` so future work can distinguish the short-term implementation from this design's longer-lived decisions.

- One active tab and one mounted-tab set for the selected session; switching sessions resets to Chat and terminates the old Shell.
- One PTY Shell per selected session tab, without reconnect, shell history persistence, multiple terminal instances, or background execution after switching.
- Lazy immediate-child file loading; document discovery capped by depth/file count; file previews capped at 1 MiB.
- Read-only Git status and diffs for working, staged, and bounded untracked changes; no Git mutations.
- Logs read from the current active unified log file with bounded paging; no archive-wide index and no SQLite mirror.
- Terminal and Report use a bounded message history and visibly identify partial results.
- Web/mock supplies deterministic fixtures and simulated I/O, with explicit capability messaging for unavailable native export/process behavior.

## Future Optimization and Extension Path

1. Add per-session tab-state caching with an LRU limit so session switching can restore tab selection without retaining unbounded component trees.
2. Add cancellable directory requests, virtualized trees, file watching, cache invalidation events, `.gitignore` awareness, configurable hidden-file visibility, and incremental search.
3. Move large-file preview to chunked/range reads with syntax highlighting in a worker and explicit encoding selection.
4. Add incremental Git diff streaming, very-large-diff virtualization, commit/base comparison, blame, staging actions, and conflict tooling as separate proposals.
5. Add a native document index, document-type providers, generated-artifact registration, rich preview plugins, and safe link navigation.
6. Add PTY reconnection, multiple named shells, background-shell policy, scrollback persistence with opt-in redaction, terminal search, copy/paste controls, and remote shell adapters.
7. Build a log index or bounded sidecar metadata cache if full-file scanning becomes measurable, while retaining unified log files as the source of truth; add archive selection and streaming export.
8. Move Terminal and Report aggregation into native queries when session histories exceed the first-version bound; add cache token dimensions, duration metrics, cost adapters, and exportable reports without conflating estimates with reported usage.
9. Introduce per-capability authorization and user confirmation if future Web/HTTP adapters gain real filesystem, Git, export, or process access.

## Risks / Trade-offs

- [Eight tabs can crowd narrow layouts] → Use horizontal scrolling, stable tab widths, icon plus translated label, and visible focus/active states.
- [Keep-alive components retain memory] → Mount only visited tabs, reset on session change, cap loaded data, and defer per-session caching.
- [Filesystem traversal or symlink escape] → Resolve roots from the session registry, accept relative paths only, canonicalize both sides, reject escapes, skip hidden paths, and enforce bounds in Rust.
- [Large repositories and diffs can block native commands] → Lazy-load directories, bound results, execute Git off the UI thread, and expose loading/truncated states.
- [PTY processes can leak] → Give the manager explicit ownership, make kill idempotent, clean up on unmount/session lifecycle/app exit, and test repeated create/kill paths.
- [Shell content can expose secrets] → Do not persist raw commands/output; persist only redacted lifecycle diagnostics and keep terminal output page-visible.
- [Log filtering scans a growing file] → Bound reads and results in the first version, document partial behavior, and retain an indexed reader as a future optimization.
- [Markdown can contain unsafe HTML or links] → Disable raw HTML in the first version and handle external links through an explicit safe action.
- [Web simulation can be mistaken for native behavior] → Label mock shell/data and expose capability/unavailable states rather than reporting local success.
- [Main layout refactor can regress chat] → Extract without changing chat queries first, then add tabs, and retain existing main-chat E2E coverage.

## Migration Plan

1. Add types, adapter methods, deterministic Web/mock implementations, and contract tests without changing the visible layout.
2. Add native project, Git, log, and PTY modules with unit tests and register commands/state.
3. Extract the current center chat into the new session workspace structure and preserve existing chat behavior.
4. Add tabs incrementally, then synchronize translations and both style variants.
5. Run frontend, Rust, OpenSpec, and Playwright verification before enabling the completed workspace by default.

Rollback removes the new center tab composition and adapter/native registrations while restoring the extracted Chat panel. No database migration or log-format change is required, so persisted sessions, messages, and logs remain compatible.

## Open Questions

- Exact first-version directory node, document count/depth, log page, and message-history bounds should be selected during implementation from representative repository measurements and encoded as named constants with tests.
- The maintained PTY and xterm package versions must be selected against the repository's supported Rust toolchain, Tauri 2 runtime, React 18, and lockfile at implementation time.
