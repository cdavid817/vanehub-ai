# Release

Packaging targets, signing credentials, version synchronization, and updater artifacts.

Test tiers are in [Testing](testing.md).

> **Applies to** the current stable line (v1.5.0) and `main`. Signing status below is the **current** state as enforced by `.github/workflows/package.yml`; the planned operating-system signing phase is described separately at the end so that a future tense never reads as a present-day guarantee.

## The release process

A release is one synchronized packaging pass across five matrix targets on three operating systems. The version number must agree in `package.json`, `src-tauri/Cargo.toml`, and `src-tauri/tauri.conf.json`, guarded by `version:check`.

```mermaid
sequenceDiagram
    participant Dev as Releaser
    participant Sync as Version sync
    participant Check as version:check + full verification
    participant Tag as git tag
    participant PKG as Five-target package job
    participant Win as Windows runner
    participant Mac as macOS runners (x64, arm64)
    participant Lin as Linux runners (x64, arm64)
    participant Pub as publish job
    Dev->>Sync: Sync the version number<br/>package.json / Cargo.toml / tauri.conf.json
    Sync->>Check: version:check + lint:ci + test + build<br/>+ cargo fmt / check / clippy / test<br/>+ openspec validate --specs --strict
    Check-->>Dev: Continue only when everything is green
    Dev->>Tag: Create the tag
    Tag->>PKG: Trigger the package workflow (release environment)
    PKG->>PKG: Require TAURI_SIGNING_PRIVATE_KEY<br/>(hard gate on a tag)
    par Windows
        PKG->>Win: NSIS .exe<br/>updater-signed; Authenticode only if WINDOWS_CERTIFICATE is set
    and macOS
        PKG->>Mac: .app + .dmg<br/>updater-signed; Developer ID + notarization only if APPLE_CERTIFICATE is set
    and Linux
        PKG->>Lin: .deb + AppImage<br/>updater-signed; no OS code signing
    end
    Win-->>Pub: Upload artifacts
    Mac-->>Pub: Upload artifacts
    Lin-->>Pub: Upload artifacts
    Pub->>Pub: Generate SHA256SUMS<br/>generate an SPDX SBOM<br/>generate attestations<br/>generate latest.json / preview.json<br/>assemble release notes
    Pub-->>Dev: Release complete
```

What matters in a release:

- **Version synchronization** — the version must agree across `package.json`, `src-tauri/Cargo.toml`, and `src-tauri/tauri.conf.json`. `scripts/check-version-sync.mjs` cross-checks them, and `version:unit:test` is its unit test.
- **Full verification comes first** — every verification command at the end of `AGENTS.md` must pass before the tag, plus `version:check`.
- **The published artifact matrix** — Windows x64 (NSIS `.exe`), macOS x64 and arm64 (`.dmg`, with the updater payload as `.app.tar.gz`), Linux x64 and arm64 (`.deb` and AppImage). No `.msi`, no `.rpm`, and **no Windows ARM64 package** is published; see [Release scripts](#release-scripts) for why `package.json` still has a Windows ARM64 helper.
- **The publish artifact list** — `SHA256SUMS` (a per-file sha256, verified to contain no duplicate hashes), an SPDX SBOM, GitHub build-provenance and SBOM attestations, the updater manifest (`latest.json` for stable, `preview.json` for pre-releases), and release notes.
- **Updater signing is the only hard signing gate** — on a tag build the workflow fails closed when `TAURI_SIGNING_PRIVATE_KEY` is empty. The key belongs to the protected `release` environment and never appears in repository configuration or screenshots. Manual (non-tag) rehearsal runs generate an ephemeral key on the runner instead; that path produces no distributable update signature.
- **Signing credential isolation** — signing credentials are injected only in the CI protected environment. A normal local packaging command carries neither the `desktop-e2e` feature nor the signing key.

## Current signing status (updater-only phase 1)

| Evidence | Windows x64 | macOS x64 / arm64 | Linux x64 / arm64 |
| --- | --- | --- | --- |
| Tauri updater signature (`.sig`) | Required | Required (on the `.app.tar.gz`) | Required |
| `SHA256SUMS`, SPDX SBOM, GitHub attestations | Published | Published | Published |
| Operating-system code signing | **Not Authenticode signed** | **Not Developer ID signed, not notarized, not stapled** | Not applicable |
| User-facing consequence | SmartScreen may warn | Gatekeeper may report the app as damaged until quarantine is cleared | None |

The workflow contains provider-independent steps for both operating-system signing paths (`Import-PfxCertificate` + `signtool` for Windows, `codesign --verify` + `stapler validate` + `spctl --assess` for macOS), but each is guarded by `env.WINDOWS_CERTIFICATE != ''` or `env.APPLE_CERTIFICATE != ''` respectively. Those secrets are not provisioned on the `release` environment, so the steps are skipped and a non-prerelease tag logs an explicit unsigned disclosure instead. Checksums, SBOM, and attestations prove integrity and provenance; they do not replace operating-system trust.

The stable release notes (`.github/STABLE_RELEASE_NOTES.md`) and the three README files state the same status; `scripts/release-policy.node-test.mjs` and `scripts/docs-facts.node-test.mjs` fail when any of these surfaces drifts from the workflow.

Packaging and signing details live in `src-tauri/ARCHITECTURE.md` and the [release signing guide](../../release-signing.md); CI orchestration lives in `.github/workflows/ci.yml` and `.github/workflows/package.yml`.

## Release scripts

- **Local packaging helpers** — `package.json` defines six helper scripts: `package:windows:{x64,arm64}`, `package:macos:{x64,arm64}`, and `package:linux:{x64,arm64}`, each preceded by `sidecar:prepare -- --release --target=...`. These are local conveniences, **not** the release matrix: the GitHub Actions matrix in `package.yml` has five entries (Windows x64, macOS x64, macOS arm64, Linux x64, Linux arm64), and the v1.5.0 release assets match those five. `package:windows:arm64` exists for local experiments only; adding Windows ARM64 as a published target needs its own workflow entry, smoke verification, release-notes update, and asset verification first.
- **Version synchronization** — `scripts/check-version-sync.mjs` requires the three version declarations to agree, with `version:unit:test` as its unit test.
- **Signing credentials** — the protected `release` environment is selected by `github.ref_type == 'tag' ? 'release' : 'build-preview'`. The updater uses `TAURI_SIGNING_PRIVATE_KEY` (and its password) to produce `createUpdaterArtifacts`, with the public key embedded in `tauri.conf.json`. Windows (`WINDOWS_CERTIFICATE`, `WINDOWS_CERTIFICATE_PASSWORD`, `WINDOWS_SIGNER_SUBJECT`) and Apple (`APPLE_CERTIFICATE`, `APPLE_CERTIFICATE_PASSWORD`, `APPLE_SIGNING_IDENTITY`, `APPLE_ID`, `APPLE_PASSWORD`, `APPLE_TEAM_ID`) credentials are forwarded only when present and are not required today; the full split is in the release signing guide's secret inventory.
- **Release policy test** — `npm run release:unit:test` (`scripts/release-policy.node-test.mjs`) asserts the five-target matrix, the fail-closed updater key check, and the stable-notes signing disclosures against the workflow file.

## Phase 2 (planned): operating-system signing

This phase is **not** in place. When it is enabled, the plan is:

- Windows: provision a managed certificate or key-vault provider, set the three `WINDOWS_*` secrets, and make the existing `signtool` step and publisher/timestamp verification a hard gate for non-prerelease tags.
- macOS: provision Developer ID credentials and an app-specific password, set the six `APPLE_*` secrets, let the Tauri bundler sign and notarize, and make `stapler validate` plus `spctl --assess` a hard gate.
- Update `.github/STABLE_RELEASE_NOTES.md`, the README signing section, the release signing guide, and `scripts/release-policy.node-test.mjs` in the same change, so that no surface claims signing before the workflow enforces it.
