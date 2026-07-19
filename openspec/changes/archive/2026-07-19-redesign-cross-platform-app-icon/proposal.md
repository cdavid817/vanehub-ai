## Why

VaneHub AI's original app icon was too small and visually generic to communicate the product's identity as an AI Agent Workspace, and the repository only wired a Windows ICO into the desktop bundle. A coherent, scalable brand icon system is needed so the product remains recognizable and polished across desktop packages, mobile icon exports, browser surfaces, and small favicon sizes.

## What Changes

- Redesign the application icon around the existing `V` brand mark while adding restrained visual language for agent coordination, workspace convergence, and automation.
- Use a minimal surface treatment with a flat three-color palette, no decorative gloss, no ambient aura, no blur shadow, and no nonessential bridge or rim details.
- Add optically distinct master and compact SVG sources so 16–32px surfaces remain legible without sacrificing detail at larger sizes.
- Add Android adaptive foreground and monochrome sources plus a manifest-driven generation configuration.
- Add a single cross-platform Node generation command that emits Windows, macOS, Linux, iOS, Android, and Web icon assets.
- Configure the Tauri desktop bundle to use platform-appropriate PNG, ICNS, and ICO outputs.
- Add favicon, Apple touch icon, and Web App Manifest integration for the browser runtime.
- Remove obsolete candidate assets and legacy PowerShell generators so the repository has one authoritative icon source and workflow.

## Capabilities

### New Capabilities

- `cross-platform-app-icon`: Defines the canonical VaneHub AI app icon, responsive optical variants, deterministic asset generation, platform integration, and verification requirements.

### Modified Capabilities

None.

## Impact

- Desktop runtime packaging: `src-tauri/tauri.conf.json` and generated Windows/macOS/Linux assets.
- Web runtime branding: `index.html`, `public/site.webmanifest`, favicon, and touch-icon assets.
- Brand source and outputs: `src-tauri/icons/`.
- Developer workflow: `scripts/generate-vanehub-icon.mjs` and the `icons:generate` npm script.
- No frontend service, backend command, database, or runtime-adapter boundary changes.
