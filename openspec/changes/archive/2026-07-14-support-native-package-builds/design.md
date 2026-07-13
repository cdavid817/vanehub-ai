## Context

VaneHub AI uses a Vite/React frontend and a Tauri 2 desktop shell. The project already has `npm run build`, `npm run tauri:build`, and Tauri bundling enabled, but there is no release-oriented packaging contract, CI workflow, or platform/architecture artifact naming convention.

Tauri native packaging depends on host operating-system tooling. Windows installers should be produced on Windows runners, macOS bundles on macOS runners, and Linux packages on Linux runners. Architecture support also depends on runner availability, Rust targets, native dependencies, and platform-specific signing or packaging requirements.

## Goals / Non-Goals

**Goals:**

- Provide a local one-command packaging entry point for maintainers.
- Build native desktop artifacts for Windows, macOS, and Linux.
- Cover x86_64 and ARM64 targets where the host platform and Tauri bundler support them.
- Add a GitHub Actions workflow that uses native OS runners instead of relying on unsupported cross-OS packaging.
- Upload artifacts using deterministic names that include app, OS, architecture, and version context.
- Document the prerequisites and limitations for local packaging and CI builds.

**Non-Goals:**

- Full release publishing to GitHub Releases, app stores, package registries, or auto-update feeds.
- Production code signing, Apple notarization, Windows certificate management, or secret provisioning beyond placeholders.
- Guaranteeing that every package format is available for every Linux distribution.
- Replacing the existing development workflow for `npm run dev` or `npm run tauri:dev`.

## Decisions

1. Use Tauri CLI as the canonical packaging backend.

   The packaging workflow should call `tauri build` through npm scripts so frontend build, Rust compilation, and bundling remain aligned with Tauri configuration. This avoids duplicating bundler behavior in custom scripts.

   Alternative considered: write custom OS-specific packaging scripts. This would create more control, but would duplicate Tauri behavior and make signing, bundle targets, and future Tauri upgrades harder to maintain.

2. Use native GitHub Actions runners per operating system.

   The CI workflow should build Windows artifacts on `windows-*`, macOS artifacts on `macos-*`, and Linux artifacts on `ubuntu-*`. This matches Tauri's native bundling model and avoids unsupported cross-OS packaging assumptions.

   Alternative considered: build all platforms from one Linux runner. That is simpler operationally, but native installers and app bundles generally require OS-specific tooling.

3. Model architecture support as an explicit matrix.

   The workflow should include matrix entries for `x86_64` and `aarch64`/ARM64 targets, with runner and Rust target settings declared per platform. Unsupported or impractical combinations should be disabled or documented rather than silently attempted.

   Alternative considered: let Tauri infer the target from the runner. That is easier for default builds, but it does not satisfy the requirement to support both x86 and ARM architectures predictably.

4. Keep artifact naming independent of package extension.

   CI artifact names should identify `vanehub-ai`, platform, architecture, and version or git reference, while preserving the packaged files produced by Tauri. This makes artifacts easy to compare even when Tauri emits multiple files for one target.

   Alternative considered: rely on the raw `src-tauri/target/release/bundle` directory structure. That is sufficient for local inspection, but too ambiguous for CI downloads.

5. Treat signing and notarization as optional release hardening.

   The initial packaging support should include documented placeholders for signing-related secrets, but unsigned artifacts should still build for internal validation. This keeps the build pipeline usable before release credentials exist.

   Alternative considered: require signing from the first implementation. That would block CI validation until certificates and Apple credentials are available.

## Risks / Trade-offs

- Native dependencies differ by runner image -> Pin required Linux packages in the workflow and document local equivalents.
- ARM64 runner availability varies by GitHub plan and image support -> Use explicit matrix entries and document which targets are built natively versus cross-targeted where practical.
- macOS signing/notarization can fail independently of compilation -> Keep unsigned build validation separate from release signing configuration.
- Linux package formats may require additional system packages -> Start with Tauri-supported default Linux bundles and add package dependencies only when required by the selected bundle targets.
- CI runtime can grow with a full OS/architecture matrix -> Allow matrix entries to be maintained independently and make artifact upload conditional on successful bundle generation.

## Migration Plan

1. Add package scripts for local packaging and architecture-specific Tauri builds.
2. Adjust Tauri bundle metadata only where required for reliable artifact generation.
3. Add the GitHub Actions native build matrix and artifact upload steps.
4. Add documentation for local prerequisites, commands, CI artifacts, and known unsupported target combinations.
5. Validate at least the host-platform build locally, then rely on CI to validate the full OS matrix.

Rollback is straightforward: remove the added npm scripts, workflow file, and packaging documentation. Existing development and build commands should continue to work throughout the change.

## Open Questions

- Should release artifacts be unsigned initially, or should implementation include signing secret names and disabled-by-default signing steps?
- Which Linux bundle formats should be required for the first version: AppImage only, or also deb/rpm?
- Should GitHub Actions run on every push, only pull requests, or only manual `workflow_dispatch` plus tags?
