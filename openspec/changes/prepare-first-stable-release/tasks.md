## 1. Release Metadata

- [x] 1.1 Update npm, Cargo, Tauri, and lockfile version metadata from `0.1.0-preview.1` to `1.0.0`.
- [x] 1.2 Run the version synchronization check and its unit tests for both manifest and `v1.0.0` tag inputs.

## 2. Stable Release Publication

- [x] 2.1 Add reviewed stable release notes covering highlights, supported packages, verification evidence, updater behavior, support routes, and known limitations.
- [x] 2.2 Update the package workflow so stable tags prepend the tracked stable notes while preview tags retain preview-specific guidance.
- [x] 2.3 Add automated release-policy checks that pin stable/preview note selection, fail-closed signing prerequisites, and the declared package matrix.
- [x] 2.4 Update maintainer release documentation with the pre-tag readiness record, GitHub Environment credential inventory, annotated-tag command, and post-publication checks.

## 3. Repository Verification

- [x] 3.1 Validate `prepare-first-stable-release` strictly and validate all main specifications strictly.
- [x] 3.2 Run all mandatory frontend, coverage-policy, version, contract, build, Rust format, clippy, test, and check commands from `AGENTS.md`.
- [x] 3.3 Run the UI and native desktop verification commands required for desktop release behavior and record Windows as `PASSED`, `FAILED`, `BLOCKED`, or `NOT RUN`.
- [x] 3.4 Keep release-version fixtures and the workspace lockfile synchronized so CI documentation builds remain read-only.

## 4. GitHub Release Readiness

- [x] 4.1 Push the reviewed branch and complete a manual non-publishing `Package Desktop Apps` rehearsal for Windows x64, macOS x64/ARM64, and Linux x64/ARM64.
- [ ] 4.2 Confirm all required updater, Windows, and Apple secret names exist in the protected `release` environment without exposing their values.
- [x] 4.3 Record per-platform rehearsal status and verify the source commit is ready to merge before creating any stable tag.
- [ ] 4.4 After merge, create and push the annotated `v1.0.0` tag only with explicit maintainer approval, then verify the GitHub Release, signatures, notarization, checksums, SBOM, attestations, and stable updater metadata.
