## Why

VaneHub's release workflow can publish unsigned preview packages and integrity metadata, but it cannot yet prove native publisher identity or deliver a cryptographically verified in-app update. The existing frontend-only GitHub release check also bypasses the service/runtime operation boundary and cannot safely download, verify, or apply an update.

## What Changes

- Extend the existing desktop release workflow with protected Windows signing, macOS Developer ID signing/notarization/stapling, platform verification evidence, and a stable-release signing gate while retaining explicit unsigned rehearsal behavior.
- Preserve Linux SHA-256, SPDX SBOM, and attestations as integrity controls without describing them as code signing; reuse the active `publish-verifiable-checksums` manifest contract.
- Produce signed Tauri v2 updater artifacts and channel-specific metadata from the same collected release artifacts, with private keys confined to the protected release environment.
- Add a desktop-owned asynchronous update lifecycle for check, download, verified install, restart readiness, recovery, automatic-check preference, and stable/preview channel policy.
- Replace the About page's direct GitHub fetch with the shared AgentService contract and matching Tauri and Web/mock adapters; expose current/latest versions, release notes, last check, progress, failure, ready, and restart states without blanking existing content.
- Add release rehearsal, manifest/channel/security negative tests, browser/desktop update tests, visual coverage, and deterministic update-policy performance evidence.

## Capabilities

### New Capabilities

- `signed-desktop-auto-update`: Defines trusted update sources, signed metadata/artifact verification, channel/downgrade policy, asynchronous lifecycle, preferences, UI states, recovery, and runtime-adapter compatibility.

### Modified Capabilities

- `desktop-release-delivery`: Adds platform signing/notarization verification, stable publication gates, signed updater metadata, credential isolation, and non-publishing rehearsal requirements to the existing release pipeline.

## Impact

- **Runtimes:** Both desktop and Web/mock. Only Tauri downloads and installs native updates; Web/mock simulates the identical observable contract without native side effects.
- **Frontend boundary:** `AgentService`, `tauri-agent-client`, and `web-agent-client` gain one aligned update API. React remains runtime-agnostic.
- **Native ownership:** Existing `desktop` bounded context owns update policy and lifecycle; existing `operations` contracts own observable long-running status/log association. No new bounded context or database is introduced.
- **Release system:** `.github/workflows/package.yml`, Tauri configuration/capabilities, release documentation, and repository scripts/tests change. Existing packaging, checksum, SBOM, and attestation stages are extended rather than replaced.
- **Dependencies:** Tauri v2 updater and process plugins are added at versions compatible with the repository's current Tauri 2 dependency line.
- **Compatibility/migration:** Existing app settings remain readable; new auto-check/channel values use backward-compatible defaults. The fixed updater endpoint and embedded public key are build-time configuration and cannot be silently overridden by ordinary runtime settings.
- **Coordination:** The active `publish-verifiable-checksums` change remains authoritative for served checksum names and its next-real-tag verification task; this change consumes that contract and does not duplicate it.
