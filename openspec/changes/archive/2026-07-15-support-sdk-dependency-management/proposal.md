## Why

The SDK dependencies page is currently a demo-data surface, but VaneHub needs a real local dependency manager so users can install and maintain Claude Code SDK and Codex SDK from the settings center. This matters now because agent availability and workflow launch depend on local SDK/runtime readiness, and users need a visible, recoverable path when an SDK is missing or outdated.

## What Changes

- Add managed SDK dependency support for Claude Code SDK and Codex SDK.
- Install SDK packages into VaneHub-owned user storage under `~/.vanehub/dependencies/<sdk-id>/` instead of the project `node_modules` or another product's dependency directory.
- Detect installed SDK status, installed version, latest/selectable versions, update availability, install path, and Node/npm environment readiness.
- Support installing a selected version, updating, rolling back to an older selected version, uninstalling, refreshing status, and viewing installation logs.
- Preserve frontend/backend isolation by exposing SDK operations through a frontend SDK service interface with Web/mock and Tauri adapters.
- Keep the SDK settings page visually consistent with the existing VaneHub settings center while replacing demo data with service-backed state.
- Enforce backend safety constraints for package ids, version values, process execution, and deletion boundaries.

## Capabilities

### New Capabilities
- `sdk-dependency-management`: Defines managed local SDK dependency detection, version discovery, install/update/rollback/uninstall operations, logs, installation paths, and safety constraints.

### Modified Capabilities
- `settings-center-ui`: The SDK dependencies page becomes a service-backed management page instead of a static demo-data page.
- `agent-tool-registry`: Agent availability may use managed SDK dependency status as a readiness signal for SDK-backed agents without launching interactive sessions.

## Impact

- Affects both desktop and Web runtimes.
- Desktop runtime adds Tauri/Rust SDK dependency commands for local filesystem, Node/npm detection, npm version queries, package installation, package removal, and operation logs.
- Web runtime adds a mock SDK dependency adapter so browser preview remains usable without native filesystem or npm access.
- Frontend adds SDK dependency service interfaces/adapters and replaces the current `SdkPage` demo table with service-backed controls.
- Local user filesystem adds `~/.vanehub/dependencies/` with SDK subdirectories, package manifests, marker files, and a VaneHub dependency manifest.
- Network-dependent version discovery and installation use npm registry access; fallback version lists are required when registry calls fail.
