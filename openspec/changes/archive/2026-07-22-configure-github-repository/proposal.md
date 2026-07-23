## Why

VaneHub AI needs one coherent GitHub operating model that validates every change, protects the default branch, continuously assesses dependencies and source code, and publishes verifiable desktop releases. Consolidating the earlier CI and repository-hardening proposals keeps the repository contract reviewable as one change and removes overlapping active specifications.

## What Changes

- Add pull-request and `main` branch CI for ESLint, TypeScript/Vite builds, full Vitest and contract checks, strict OpenSpec validation, Rust formatting/check/Clippy/tests, native platform checks, and Playwright E2E with retained failure reports.
- Protect `main` with pull-request, required-check, conversation-resolution, deletion, and non-fast-forward rules while preserving a workable single-maintainer flow.
- Add Dependabot, dependency review, CodeQL, immutable Action references, least-privilege permissions, secret-scanning controls, and repository security settings.
- Publish version-synchronized Windows, macOS, and Linux artifacts through GitHub Releases with checksums, an SPDX SBOM, attestations, generated notes, and protected signing/notarization inputs.
- Add ownership, contribution, conduct, security, support, issue, pull-request, labeling, and release-note conventions for public collaboration.
- Repair deterministic formatting, cross-platform native behavior, and E2E test defects required to establish a green validation baseline.
- Supersede the overlapping `add-github-ci-workflow` and `harden-github-repository` active changes with this comprehensive change.

## Capabilities

### New Capabilities

- `continuous-integration`: Defines required GitHub-hosted frontend, OpenSpec, Rust, native-platform, and browser validation with useful failure diagnostics.
- `repository-governance`: Defines protected-branch, ownership, contribution, review, labeling, and required-validation behavior.
- `software-supply-chain-security`: Defines dependency maintenance, code scanning, workflow hardening, SBOM, and provenance expectations.
- `desktop-release-delivery`: Defines synchronized cross-platform GitHub Release creation, integrity metadata, and credential-safe signing boundaries.

### Modified Capabilities

None.

## Impact

- Affects `.github/`, repository community documents, README badges, release documentation, Playwright configuration/tests, and targeted Rust cross-platform behavior needed by CI.
- Affects GitHub repository settings for `cdavid817/vanehub-ai`, including rulesets, security features, Actions policy, labels, merge settings, and deployment environments.
- Affects validation and delivery for both Web and Tauri desktop runtimes without changing the React service boundary, adapter parity contract, SQLite ownership, or application APIs.
- Adds no production runtime dependencies; signing and notarization credentials remain maintainer-provisioned encrypted secrets.
