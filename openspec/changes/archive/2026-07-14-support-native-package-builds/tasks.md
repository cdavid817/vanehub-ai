## 1. Package Script Setup

- [x] 1.1 Add a local `package` npm script that runs the Tauri desktop bundle workflow.
- [x] 1.2 Add architecture-specific package scripts for x86_64 and ARM64 targets where Tauri supports explicit target selection.
- [x] 1.3 Verify existing `build` and `tauri:build` scripts still work after adding package scripts.

## 2. Tauri Bundle Configuration

- [x] 2.1 Review `src-tauri/tauri.conf.json` bundle settings and confirm required native bundle targets are enabled.
- [x] 2.2 Add or document required application metadata for package artifacts, including product name, identifier, version, and icons.
- [x] 2.3 Document unsigned build behavior and leave signing/notarization placeholders disabled unless credentials are configured.

## 3. GitHub Actions Native Build Workflow

- [x] 3.1 Add a GitHub Actions packaging workflow under `.github/workflows/`.
- [x] 3.2 Define a build matrix for Windows, macOS, and Linux native runners.
- [x] 3.3 Define x86_64 and ARM64 target entries with platform-specific Rust target configuration.
- [x] 3.4 Install Node, Rust, npm dependencies, Rust targets, and Linux native package prerequisites in the workflow.
- [x] 3.5 Run the package command for each matrix entry using the platform-appropriate target settings.
- [x] 3.6 Upload generated Tauri bundle artifacts with names that include VaneHub AI, platform, architecture, and git reference context.

## 4. Packaging Documentation

- [x] 4.1 Document local packaging prerequisites for Windows, macOS, and Linux.
- [x] 4.2 Document local packaging commands and the expected Tauri bundle output directory.
- [x] 4.3 Document GitHub Actions workflow triggers, artifact naming, and download locations.
- [x] 4.4 Document unsupported or credential-dependent target combinations, including signing and notarization limitations.

## 5. Validation

- [x] 5.1 Run the normal frontend build to confirm package script changes did not break the web build.
- [x] 5.2 Run the host-platform package command locally and confirm artifacts are produced under the Tauri bundle output directory.
- [x] 5.3 Validate the OpenSpec change artifacts.
- [x] 5.4 Record any CI-only validation gaps that cannot be verified locally.
