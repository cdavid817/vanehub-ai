## Why

VaneHub AI sessions are already durable, but users need stronger ways to find, organize, preserve, and recover historical work as session volume grows. This change turns the session list from a simple recent-history sidebar into a managed workspace history with search, categories, export, automatic archival, startup recovery, and richer chat context.

## What Changes

- Add historical session search by title, project metadata, and message content across active and archived sessions.
- Add user-defined session categories with sidebar grouping, collapsible category sections, context-menu moves, category creation from the session menu, and drag-and-drop assignment.
- Add session export from the session context menu with JSON and Markdown formats saved to a user-selected directory.
- Add native automatic archival for inactive non-pinned sessions: run once at application startup and then every hour, using a configurable inactivity threshold with a default of 10 days.
- Add startup session recovery that reconciles sessions and messages left in active states after an application crash while preserving existing partial content and provider resume metadata.
- Render Mermaid flow charts in chat messages when fenced `mermaid` code blocks appear, with safe fallback behavior when rendering fails.
- Add chat composer file `@` references for files under the active session root, using service-backed file discovery and bounded content reads before sending.

## Capabilities

### New Capabilities
- `session-category-management`: User-defined session categories, category assignment, sidebar category grouping, context-menu category actions, and drag-and-drop moves.
- `session-export`: Session transcript and metadata export to JSON or Markdown through desktop-native save behavior and Web/mock parity.

### Modified Capabilities
- `session-management`: Adds historical session search, category linkage on sessions, automatic archival eligibility, and startup recovery expectations for durable session records.
- `session-runtime-management`: Adds crash recovery reconciliation for orphan active generations and unfinished messages while preserving provider resume metadata.
- `chat-experience`: Adds safe Mermaid rendering in chat messages and composer file `@` references that are submitted with messages.
- `main-layout-ui`: Adds search, category view, category context actions, drag-and-drop assignment, and export entry points in the session sidebar.
- `frontend-runtime-architecture`: Extends service boundaries so search, categories, export, file references, and render-supporting metadata remain behind frontend services and adapters.
- `native-runtime-architecture`: Adds Rust-owned scheduled archival, startup recovery work, SQLite queries/migrations, filesystem export, and unified diagnostics.
- `app-settings`: Adds automatic archival configuration for enablement and inactivity days.

## Impact

- Frontend service APIs in `src/services/agent-service.ts` and both `tauri-agent-client.ts` and `web-agent-client.ts` need additive methods and matching contracts.
- React sidebar and chat components need localized UI for search, category grouping, context actions, drag-and-drop assignment, export format selection, Mermaid rendering, and file-reference chips.
- Rust/Tauri needs SQLite migrations for categories, session category linkage, optional file-reference message metadata, and automatic archival settings.
- Native commands need to search sessions/messages, manage categories, export sessions, run startup recovery, and run hourly automatic archival without blocking the UI.
- Unified logging must record automatic archival and crash recovery diagnostics with redaction.
- Web/mock runtime must provide deterministic parity for search, categories, export simulation, Mermaid rendering, and file-reference fixtures.
