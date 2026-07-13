## Why

VaneHub AI is a desktop app, but the current project only exposes generic Tauri build commands and does not define a repeatable release path for users or maintainers. Native package support is needed so the app can be built consistently for Windows, macOS, and Linux across x86 and ARM architectures, both locally and in GitHub Actions.

## What Changes

- Add a one-command packaging workflow that builds the web frontend and Tauri desktop bundles.
- Define supported native package outputs for Windows, macOS, and Linux.
- Support x86_64 and ARM64 build targets where the platform toolchain supports them.
- Add GitHub Actions workflows that perform native builds on the corresponding operating-system runners.
- Publish or upload build artifacts from CI with clear names that include platform and architecture.
- Document local prerequisites and packaging commands for each supported platform.

## Capabilities

### New Capabilities

- `native-app-packaging`: Defines local and GitHub Actions workflows for producing VaneHub AI desktop packages across Windows, macOS, and Linux for x86_64 and ARM64 architectures.

### Modified Capabilities

None.

## Impact

- Affects Tauri packaging configuration in `src-tauri/tauri.conf.json`.
- Affects npm scripts and release/build commands in `package.json`.
- Adds GitHub Actions workflow configuration under `.github/workflows/`.
- May require Rust target configuration, Tauri bundler prerequisites, signing/notarization placeholders, and platform-specific package dependencies.
- Adds release documentation for local builds and CI artifacts.
