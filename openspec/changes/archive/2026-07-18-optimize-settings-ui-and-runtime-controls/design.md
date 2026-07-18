## Context

Basic Configuration currently mixes application preferences, floating-assistant control, network proxy, log management, Node information, and storage notes in a layout that makes desktop data and system behavior hard to find. The settings page registry still exposes SDK Dependencies as a primary navigation item, while the user wants SDK removed from navigation without deleting the underlying SDK capability.

The desktop runtime already owns SQLite through `RegistryStore` and exposes log-directory opening through the settings service boundary. Launch-on-startup is not currently modeled in app settings. Session records already persist stable `agentId` values, and the workspace spec already expects known agent markers, so CLI-specific icons should be derived in the frontend from stable ids instead of stored in SQLite.

## Goals / Non-Goals

**Goals:**
- Reorganize and polish Basic Configuration around user-recognizable sections.
- Add service-backed Data Management and launch-on-startup controls with desktop/Web parity.
- Hide SDK Dependencies from primary settings navigation while retaining SDK code and future capability.
- Move Extension Capabilities lower in navigation as an advanced capability.
- Improve settings icons, rounded icon containers, and floating-assistant settings presentation.
- Use CLI-specific session icons based on stable agent ids after session creation.

**Non-Goals:**
- Delete SDK service, native SDK modules, SDK tests, or SDK specs.
- Add direct Tauri `invoke()` calls to React components.
- Store icon names, colors, or vendor artwork in session rows.
- Change session creation semantics, CLI launch behavior, or provider argument composition.
- Open the SQLite database file for editing; this change opens the containing directory only.
- Redesign the floating assistant native window lifecycle beyond settings-surface polish and lightweight state handling.

## Decisions

1. **Use settings service methods for desktop data and startup actions.**

   Add frontend service methods such as `openDatabaseDirectory`, `getStartupSettings`, and `setLaunchOnStartup`, implemented by both Tauri and Web/mock adapters. Tauri adapters call Rust commands; Web/mock returns deterministic unavailable state or mock persisted values. This keeps React components isolated from native details.

   Alternative considered: call Tauri commands directly from Basic Configuration. Rejected because project architecture requires components to depend on service interfaces.

2. **Adopt the official Tauri autostart plugin.**

   Desktop launch-on-startup should use Tauri's maintained autostart integration instead of hand-written Windows registry entries. This keeps platform semantics centralized and leaves room for future macOS/Linux packaging without replacing the setting contract.

   Alternative considered: implement Windows registry startup entries manually. Rejected because it increases platform-specific maintenance and makes future cross-platform behavior harder to align.

3. **Persist launch-on-startup as an app setting plus native registration state.**

   `launchOnStartup` becomes part of app settings so the UI has a durable desired value. The desktop command synchronizes the desired value with the autostart plugin and returns the committed settings state. Startup reads the persisted value and can reconcile native registration if needed.

   Alternative considered: query only native autostart state without storing app preference. Rejected because the settings provider already owns common preferences and Web/mock parity needs a stable contract.

4. **Open the database directory, not the database file.**

   Data Management exposes the SQLite path and opens its parent directory. This helps users find backup/support data while reducing the chance of opening a live SQLite file in another tool.

   Alternative considered: open `vanehub.sqlite` directly. Rejected because external editors can lock or corrupt the active database.

5. **Hide SDK Dependencies from navigation without removing the page implementation.**

   Remove the `sdk` entry from the primary `settingsPages` registry and associated visible navigation/search route. Keep `SdkPage`, SDK services, native modules, and tests unless they become unreachable-test-only cleanup during implementation. This matches the user's UI request and avoids breaking future extension or package workflows.

   Alternative considered: delete SDK dependencies entirely. Rejected because the request clarified navigation hiding only.

6. **Use a shared agent visual identity helper.**

   Introduce or extend a frontend helper keyed by managed agent ids that returns icon component, color token/class, and accessible label data for `claude-code`, `codex-cli`, `gemini-cli`, and `opencode`. Session cards and session-adjacent workspace UI consume this helper. Unknown ids fall back to a neutral agent icon.

   Alternative considered: duplicate `agentId` branches inside every component. Rejected because it drifts quickly and makes future agent additions harder.

7. **Polish through existing primitives and semantic tokens.**

   Basic Configuration should use existing `PageHeader`, `SectionPanel`, shared button/input styles, lucide icons, and Tailwind classes. Icon containers should use consistent rounded geometry and stable dimensions without nested cards or new UI libraries.

## Risks / Trade-offs

- [Autostart plugin capability setup is incomplete] -> Add Tauri plugin initialization, capability/permission configuration, and focused Rust/frontend checks in the same implementation slice.
- [Web/mock appears to support desktop-only actions] -> Return explicit unavailable state and disable/open actions while keeping the page usable.
- [Opening the database directory could expose implementation details] -> Label it as local app data, show the path read-only, and avoid direct database editing affordances.
- [SDK page removal breaks tests that assume every registry entry renders] -> Update navigation tests to assert hidden SDK behavior and keep service-level SDK tests unchanged.
- [Vendor icon assets are unavailable or legally ambiguous] -> Prefer existing project assets if present; otherwise use lucide-backed semantic icons plus brand colors and accessible labels.
- [Basic Configuration becomes too long] -> Use clear section ordering and internal page scrolling, with floating assistant intentionally last.
