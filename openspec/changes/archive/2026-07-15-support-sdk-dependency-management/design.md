## Context

VaneHub already has a settings center with an SDK dependencies page, but that page is backed by static demo data. The MCP settings page has the preferred runtime boundary: React calls a TypeScript service interface, runtime adapters select Web/mock or Tauri behavior, and the Tauri adapter invokes Rust commands.

The target behavior is based on the SDK dependency management pattern in `jetbrains-cc-gui`: SDKs are optional packages installed on demand into a product-owned user directory, versions are selectable, install/update/uninstall operations produce logs, and runtime SDK loading checks the managed package paths. VaneHub will use its own installation root, `~/.vanehub/dependencies/`, so dependency lifecycle is isolated from other tools.

## Goals / Non-Goals

**Goals:**
- Manage Claude Code SDK and Codex SDK from the VaneHub SDK settings page.
- Install packages into `~/.vanehub/dependencies/<sdk-id>/`.
- Detect installed status, installed version, latest/selectable versions, update availability, install path, and Node/npm readiness.
- Support install, update, rollback, uninstall, refresh, and operation logs.
- Keep React components behind a frontend SDK service interface with Tauri and Web/mock adapters.
- Enforce backend safety rules for SDK ids, semver versions, npm process execution, and uninstall paths.
- Keep browser preview usable through mock SDK dependency state and simulated logs.

**Non-Goals:**
- Global npm package management or modification of the project `node_modules`.
- Sharing dependency directories with `jetbrains-cc-gui` or other products.
- Managing CLI executables installed by winget, brew, cargo, or system package managers.
- Loading SDK modules into the VaneHub process as part of this change.
- Solving provider authentication or model configuration.

## Decisions

### Use a VaneHub-owned dependency root

Install SDKs under `~/.vanehub/dependencies/<sdk-id>/`.

Rationale: this keeps SDK lifecycle independent from the application source tree and from `jetbrains-cc-gui`'s `~/.codemoss/dependencies/` directory. It also gives uninstall a narrow, predictable deletion boundary.

Alternative considered: reuse `~/.codemoss/dependencies/`. Rejected because it couples two products' install, update, rollback, and uninstall behavior.

### Model SDKs as whitelisted definitions

The backend owns a fixed SDK definition list:
- `claude-sdk`: `@anthropic-ai/claude-agent-sdk`, plus `@anthropic-ai/sdk` and `@anthropic-ai/bedrock-sdk`.
- `codex-sdk`: `@openai/codex-sdk`.

Rationale: installable package ids must not come from user input. A whitelist allows the UI to remain extensible while keeping native operations bounded.

Alternative considered: allow custom npm package names. Rejected for P1 because it expands the security and support surface.

### Follow the MCP service-adapter pattern

Add `SdkService` and runtime adapters:
- `runtime-sdk-client.ts` selects Tauri or Web.
- `tauri-sdk-client.ts` contains all `invoke()` calls.
- `web-sdk-client.ts` returns mock status and simulated logs.
- `sdk-service.ts` defines the TypeScript interface.

Rationale: this matches the existing MCP and agent service boundaries and keeps React components free of runtime-specific calls.

Alternative considered: call Tauri directly from `SdkPage`. Rejected because it violates the frontend/backend isolation already established in the project.

### Perform local package work in Rust Tauri commands

Add a Rust `sdk` module with models, service functions, and commands. Commands cover list/status, Node/npm environment check, version discovery, update check, install, and uninstall.

Rationale: filesystem paths, process execution, deletion, and platform differences belong in the native layer.

Alternative considered: use a Node helper script for package management. Deferred because Tauri/Rust can already run native commands and gives tighter control over paths and command construction.

### Use npm with controlled arguments and scripts disabled

Install with an argument vector equivalent to:

`npm install --include=optional --ignore-scripts --prefix <sdk-dir> <package>@<version> ...`

Rationale: argument vectors avoid shell interpolation, and `--ignore-scripts` reduces npm lifecycle script supply-chain risk.

Alternative considered: normal npm install. Rejected because lifecycle scripts are unnecessary for these SDK packages and increase risk.

### Validate versions in the backend

Requested versions are normalized by stripping a leading `v` and must match semver-like `major.minor.patch` with optional prerelease suffix. Invalid values are rejected before command construction.

Rationale: the selected version eventually becomes part of an npm package specifier. Frontend validation is not sufficient for a native command boundary.

Alternative considered: accept npm tags like `latest`. Rejected for user-selected versions because tags and ranges are less deterministic and harder to validate safely. Default definitions can still use curated fallback behavior internally.

### Provide version fallback data

The backend tries npm registry version queries first and returns fallback versions if registry access fails.

Rationale: the UI should remain usable when offline or behind a restricted network, while still showing whether version data came from a remote source or fallback list.

### Operation logs

The service should expose installation logs. If Tauri event streaming is practical, emit line-by-line log events keyed by operation and SDK id. If not, return accumulated logs from the command and keep the UI contract compatible with later streaming.

Rationale: npm installs can be slow or fail due to network/cache issues; logs are required for user recovery.

## Risks / Trade-offs

- Network-dependent npm queries may fail or be slow -> use timeouts, fallback versions, and visible source/error metadata.
- Node/npm may be missing or configured differently across Windows, macOS, Linux, or WSL -> expose a Node/npm environment check before install and include detected path/version where available.
- Package installation can take time -> disable concurrent SDK operations in the UI and show logs/progress.
- Uninstall can accidentally delete too much if paths are wrong -> normalize paths and require the resolved SDK directory to be inside the resolved `~/.vanehub/dependencies/` directory before deletion.
- `--ignore-scripts` can break packages that rely on lifecycle scripts -> acceptable for the targeted SDKs; document errors in logs if package behavior changes later.
- Rust semver comparison can be simplistic -> prefer a semver crate if already available or add focused parsing tests for update/rollback decisions.

## Migration Plan

1. Add the SDK service interface and Web/mock adapter so the page can be built and tested in browser mode.
2. Add Tauri adapter and Rust SDK commands behind the same service interface.
3. Replace the SDK demo table with service-backed cards and logs while preserving settings-center visual style.
4. Integrate managed SDK status into agent readiness where applicable.
5. Validate with OpenSpec, frontend build, and Rust checks.

Rollback is additive: remove the SDK service-backed page and commands, and the previous demo-data SDK page can be restored without data migration. User-installed SDK directories under `~/.vanehub/dependencies/` are not automatically deleted by rollback.

## Open Questions

- Should VaneHub expose a configurable Node.js path in basic settings before the SDK manager ships, or rely on PATH auto-detection for P1?
- Should install logs be implemented as Tauri events in the first implementation, or as accumulated logs returned from command results with the UI structured for later streaming?
