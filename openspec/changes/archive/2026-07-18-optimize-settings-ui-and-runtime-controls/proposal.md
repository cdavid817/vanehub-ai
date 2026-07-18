## Why

The settings center has accumulated important controls in a layout that is harder to scan, while some desktop-only runtime controls such as database access and launch-on-startup are not yet exposed from Basic Configuration. At the same time, the workspace session list should make the selected CLI identity immediately recognizable after session creation.

## What Changes

- Add a Data Management section to Basic Configuration with a desktop action that opens the directory containing the project SQLite database.
- Add a launch-on-startup setting to Basic Configuration using the official Tauri autostart integration in desktop runtime and a clear unavailable state in Web/mock runtime.
- Polish the Basic Configuration layout and visual hierarchy while preserving the existing settings service boundary and Tailwind/semantic-token styling.
- Move the desktop floating assistant setting to the bottom of Basic Configuration and refine its presentation and lightweight state handling.
- Hide the SDK Dependencies page from settings navigation while retaining the underlying SDK service, native implementation, and future capability surface.
- Move Extension Capabilities lower in the settings navigation so higher-frequency management pages remain easier to reach.
- Refine settings navigation and page icons with consistent rounded icon containers and semantic icon choices.
- Render created sessions and session-adjacent workspace surfaces with the CLI-specific icon and visual identity derived from the session's stable agent id.

## Capabilities

### New Capabilities
- `desktop-startup-controls`: Covers service-backed launch-on-startup configuration and desktop/Web runtime behavior.
- `settings-data-management`: Covers Basic Configuration database-directory visibility and desktop/Web behavior for opening local data storage.

### Modified Capabilities
- `settings-center-ui`: Change the settings navigation set and order, hide SDK Dependencies from navigation, lower Extension Capabilities, and refine settings icon presentation.
- `settings-basic-configuration-ui`: Add Data Management and launch-on-startup controls, improve Basic Configuration information architecture, and keep all controls service-backed.
- `settings-extension-management-ui`: Update the Extension Capabilities navigation placement to match its lower advanced-capability position.
- `settings-floating-assistant-ui`: Move the floating assistant setting to the bottom of Basic Configuration and refine its compact settings presentation.
- `desktop-floating-assistant`: Preserve current native behavior while improving settings-surface presentation and avoiding unnecessary UI refresh work.
- `main-layout-ui`: Ensure workspace session cards and session-adjacent UI use CLI-specific icons based on stable agent ids after session creation.
- `session-management`: Clarify that sessions continue storing stable agent ids and UI derives icon identity without persisting redundant icon fields.
- `app-settings`: Add the persisted launch-on-startup setting and expose it through centralized settings side effects.
- `native-runtime-architecture`: Add desktop commands behind the settings adapter for opening the database directory and synchronizing startup registration.

## Impact

- Frontend: settings page definitions, sidebar ordering, Basic Configuration layout, settings provider/service types, Web/mock settings adapter, Tauri settings adapter, workspace session icon presentation, and focused React tests.
- Desktop runtime: Rust settings commands, app settings persistence, database directory opening, Tauri autostart plugin registration, capability/permission configuration as required by Tauri 2, and Rust tests.
- Web runtime: mock settings behavior remains interface-compatible and reports desktop-only actions as unavailable without direct native calls.
- Dependencies: adds the official Tauri autostart plugin rather than custom registry logic.
- Architecture: React components continue calling service interfaces only; all Tauri `invoke()` usage remains in runtime-specific adapters.
