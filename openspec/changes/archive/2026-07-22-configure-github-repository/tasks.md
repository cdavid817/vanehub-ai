## 1. Continuous Integration Baseline

- [x] 1.1 Add pull-request and `main` push triggers with independent frontend, OpenSpec, Rust, native-platform, and Playwright checks
- [x] 1.2 Configure lockfile-based frontend lint, full Vitest, contract, and TypeScript/Vite build validation
- [x] 1.3 Configure Rust formatting, check, Clippy, tests, and required Tauri native dependencies
- [x] 1.4 Configure Chromium Playwright E2E with failure-only report upload, retained diagnostics, concurrency, and timeouts
- [x] 1.5 Repair deterministic formatting, native cross-platform path/window behavior, and E2E test defects exposed by CI

## 2. Supply Chain Security

- [x] 2.1 Add grouped weekly Dependabot configuration for npm, Cargo, and GitHub Actions
- [x] 2.2 Add pinned dependency-review and CodeQL workflows for JavaScript/TypeScript and Rust
- [x] 2.3 Pin every workflow Action to an immutable commit SHA and retain readable version comments
- [x] 2.4 Apply least-privilege permissions and scheduled security checks

## 3. Release Delivery

- [x] 3.1 Add and test synchronized project-version validation
- [x] 3.2 Implement staged Windows, macOS, and Linux builds with single-release publication
- [x] 3.3 Generate checksums, SPDX SBOM output, provenance and SBOM attestations, and generated release notes
- [x] 3.4 Configure the protected release environment boundary and document signing/notarization prerequisites without storing credentials

## 4. Community Governance

- [x] 4.1 Add CODEOWNERS, contribution, security, conduct, and support guidance
- [x] 4.2 Add bug and feature issue forms, issue configuration, and a project-specific pull-request template
- [x] 4.3 Add release-note categories, path-based labeling, repository labels, and README status badges

## 5. GitHub Repository Settings

- [x] 5.1 Enable dependency alerts, security updates, automated fixes, private vulnerability reporting, secret scanning, and push protection where supported
- [x] 5.2 Apply squash-oriented merge settings, automatic branch deletion, and a least-privilege Actions allow policy
- [x] 5.3 Create and verify a default-branch ruleset using registered required checks without blocking the single-maintainer workflow

## 6. Verification and Handoff

- [x] 6.1 Run frontend lint, full Vitest, contracts, TypeScript/Vite build, and Playwright E2E validation
- [x] 6.2 Run Rust formatting, check, Clippy, and tests
- [x] 6.3 Run strict validation for the comprehensive change and all main OpenSpec specifications
- [x] 6.4 Verify the pull request checks and remote GitHub repository settings through read-only inspection
- [x] 6.5 Consolidate and remove the superseded `add-github-ci-workflow` and `harden-github-repository` active changes

## Verification Record

Verified on 2026-07-23 in the `chore/github-config` worktree:

- `npm run lint`: passed with 0 errors and 15 pre-existing React Hooks warnings
- `npm run test`: 206 tests passed
- `npm run contracts:check`: 1 contract test passed
- `npm run build`: TypeScript and Vite production build passed
- `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`: passed
- `cargo check --manifest-path src-tauri/Cargo.toml`: passed
- `cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings`: passed
- `cargo test --manifest-path src-tauri/Cargo.toml`: 545 tests passed, 1 ignored; 8 architecture tests passed
- `npx playwright test`: 43 E2E tests passed
- `openspec validate configure-github-repository --strict`: passed
- `openspec validate --specs --strict`: 58 main specifications passed after synchronization
- GitHub pull request checks: Frontend, OpenSpec, Rust, Windows/macOS native checks, Playwright, Dependency Review, and both CodeQL languages passed
- Workflow dependency inspection: every external Action reference is pinned to a full commit SHA
