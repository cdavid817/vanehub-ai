## Why

Local media currently requires users to discover and type an absolute Python executable path separately for OCR, speech recognition, and speech synthesis, even when a compatible interpreter is already installed. The settings page also exposes a dense collection of engine fields without enough progressive guidance, making first-time setup and readiness troubleshooting harder than necessary.

## What Changes

- Detect compatible Python interpreters available on the desktop host without launching an inference worker, installing Python, or modifying the machine.
- Present detected interpreter candidates with version, compatibility, and safe source information so the user can select one for an engine instead of manually entering a path.
- Keep the selected interpreter explicit in the saved local-media profile; discovery does not silently replace an existing selection or act as a runtime fallback.
- Refine the Local media settings page with clearer setup progress, shared Python environment selection, more deliberate engine grouping, compact status summaries, progressive disclosure for advanced fields, and actionable validation/readiness feedback.
- Preserve truthful Web behavior: Web/mock mode keeps the same service contract and UI structure but reports native Python discovery and local inference as unavailable unless a deterministic test adapter is injected.
- Preserve the existing offline-only boundary: no Python/package/model installation, download, environment mutation, or automatic remediation is introduced.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `local-media-runtime`: Add bounded, side-effect-free host Python interpreter discovery and selection semantics while preserving explicit persisted interpreter paths and worker isolation.
- `app-settings`: Replace manual-only Python path entry with detected-environment selection and refine the Local media page's setup hierarchy, status presentation, progressive disclosure, and error guidance across desktop and Web/mock surfaces.

## Impact

- Desktop runtime: adds native interpreter discovery behind the `local_media` application/API boundary and thin Tauri commands; inference launch continues to use only the explicitly saved profile snapshot.
- Web runtime: adds adapter-parity responses that truthfully report discovery as native-only without claiming host environments.
- Frontend: updates the Local media service contract, Tauri and Web/mock adapters, settings hooks, engine cards, shared fields, localization, and accessibility/E2E coverage.
- Native backend: affects local-media domain/application discovery models, platform process/path probing, command mapping, and focused cross-platform tests.
- Frontend/backend isolation remains intact: React components consume the service interface and do not call Tauri `invoke()` directly.
- No new package manager, Python distribution, model acquisition, cloud provider, or state-management dependency is introduced.
