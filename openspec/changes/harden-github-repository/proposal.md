## Why

VaneHub AI is a public, cross-platform desktop project, but its GitHub repository currently lacks enforced merge gates, automated dependency and code security checks, a verifiable release channel, and the contributor guidance expected for public collaboration. The existing CI and packaging workflows need to become a coherent repository governance and software-supply-chain system before the project accepts broader contributions or distributes binaries.

## What Changes

- Repair the current formatting and E2E validation baseline, then expand CI with full Vitest, strict OpenSpec validation, clearer failure diagnostics, cancellation, timeouts, and explicit least-privilege permissions.
- Add GitHub repository rules that protect `main`, require pull requests and successful checks, resolve review conversations, prevent destructive pushes, and keep merge behavior consistent.
- Enable and configure dependency graph, Dependabot alerts/security updates, npm/Cargo/GitHub Actions version updates, dependency review, CodeQL, and hardened Action references.
- Replace artifact-only packaging with a release workflow that validates synchronized versions, produces checksums and SBOM/provenance attestations, creates GitHub Releases, and exposes signing/notarization inputs without embedding secrets.
- Add CODEOWNERS, contribution and security policies, code of conduct, support guidance, issue forms, a pull-request template, release-note categories, and repository labels.
- Add safe workflow efficiency controls, badges, automated labeling, and scheduled security/quality checks where they provide concrete value.

## Capabilities

### New Capabilities

- `repository-governance`: Defines protected-branch, contribution, ownership, review, and required-validation behavior for the public GitHub repository.
- `software-supply-chain-security`: Defines dependency maintenance, code scanning, Action hardening, SBOM, and build-provenance expectations.
- `desktop-release-delivery`: Defines versioned GitHub Release creation, cross-platform artifacts, integrity metadata, and credential-safe signing/notarization boundaries.

### Modified Capabilities

None.

## Impact

- Affects `.github/`, repository-level community documents, Playwright diagnostics, Rust formatting, README badges, and GitHub repository settings for `cdavid817/vanehub-ai`.
- Affects validation and delivery for both Web and Tauri desktop code without changing the React service boundary, runtime adapters, SQLite ownership, or application APIs.
- Adds GitHub-hosted automation and security scanning but no production runtime dependencies.
- Release signing and Apple notarization remain conditional on repository secrets and protected environment approval supplied by the maintainer.
