## Why

The settings center still exposes Agent registry and runtime operations as a management workflow, even after the standalone navigation entry was removed. Product direction is to remove this management surface completely rather than relocate it into Agent Configuration. Agent Configuration should remain focused on OnePiece and CLI provider configuration.

## What Changes

- **BREAKING** Remove the standalone “Agent Management” settings navigation entry, route/page module, title, and page-specific design/tests.
- **BREAKING** Remove all settings UI for API Agent registration, registered-Agent lists and status, edit/delete operations, runtime selection and mode controls, workflow launch and Session details, tool trust, and Agent memory management.
- Keep Agent Configuration limited to OnePiece and CLI provider configuration; it SHALL NOT become a replacement Agent management surface.
- Keep the underlying Agent registry, runtime/session APIs, stable Agent ids, native commands, database records, and desktop/Web service adapters because session creation and runtime execution still depend on them.
- Update localization, automated tests, and current specifications so no active product design exposes or relocates the removed management workflows.
- Preserve archived OpenSpec artifacts as immutable history; this change supersedes their former page-placement decisions instead of rewriting archive records.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `settings-center-ui`: Removes the standalone Agent Management destination without adding a replacement management destination.
- `agent-switching`: Removes settings-center requirements for registered-Agent switching, status, workflow launch, and Session details while leaving session workflows and runtime services intact.
- `cli-agent-config-management`: Keeps Agent Configuration as a dedicated OnePiece/CLI provider configuration page that is separate from Agent runtime management.

## Impact

- **Frontend:** Deletes Agent-management-only settings components, navigation/page code, localized copy, unit tests, and Playwright flows. Agent Configuration retains OnePiece and CLI configuration panels.
- **Desktop and Web runtimes:** Existing registry, session, and runtime behavior remains available through `AgentService`; no management UI is provided in settings.
- **Backend:** No Rust command, database schema, credential storage, or runtime API change is expected.
- **Architecture:** React continues to call only the shared service boundary. This change removes presentation capabilities without moving or redesigning backend responsibilities.
