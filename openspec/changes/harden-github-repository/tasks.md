## 1. Green CI Baseline

- [x] 1.1 Apply rustfmt to the current native baseline and confirm formatting passes
- [x] 1.2 Diagnose the current Playwright failures and repair deterministic E2E configuration or test defects without weakening assertions
- [x] 1.3 Extend CI with full Vitest, pinned strict OpenSpec validation, retained Playwright diagnostics, concurrency, timeouts, and explicit read-only permissions
- [x] 1.4 Add a practical cross-platform native check strategy and verify workflow syntax

## 2. Supply Chain Security

- [x] 2.1 Add grouped weekly Dependabot configuration for npm, Cargo, and GitHub Actions
- [x] 2.2 Add pinned dependency-review and CodeQL workflows for JavaScript/TypeScript and Rust
- [x] 2.3 Pin every workflow Action reference to an immutable commit SHA and retain readable version comments
- [x] 2.4 Add scheduled repository security checks and keep workflow permissions least-privilege

## 3. Release Delivery

- [x] 3.1 Add and test a project-version synchronization command
- [x] 3.2 Replace artifact-only packaging with staged cross-platform build and single-release publication jobs
- [x] 3.3 Generate release checksums, SPDX SBOM output, provenance and SBOM attestations, and generated release notes
- [x] 3.4 Configure the protected release environment boundary and document required signing/notarization secrets without storing credentials

## 4. Community Governance

- [x] 4.1 Add CODEOWNERS, CONTRIBUTING, SECURITY, CODE_OF_CONDUCT, and SUPPORT guidance
- [x] 4.2 Add bug and feature issue forms, issue configuration, and a project-specific pull-request template
- [x] 4.3 Add release-note categories, path-based labeling, repository labels, and relevant README badges

## 5. GitHub Repository Settings

- [x] 5.1 Enable dependency alerts, security updates, automated fixes, and private vulnerability reporting where supported
- [x] 5.2 Apply squash-oriented merge settings, automatic branch deletion, and a least-privilege Actions allow policy
- [x] 5.3 Create and verify a default-branch ruleset after required check names are available without making a single-maintainer repository impossible to merge

## 6. Verification and Handoff

- [x] 6.1 Run frontend lint, tests, contracts, build, and Playwright E2E validation
- [x] 6.2 Run Rust fmt, check, Clippy, and tests
- [x] 6.3 Run strict validation for both active changes and all main OpenSpec specifications
- [x] 6.4 Review local and remote diffs, record signing prerequisites, and verify GitHub settings through read-only API calls
