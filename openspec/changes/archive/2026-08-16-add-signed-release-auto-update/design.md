## Context

The existing `Package Desktop Apps` workflow already validates versions, builds the native matrix, collects distributables, generates checksums/SBOM/attestations, and publishes one release. It deliberately forwards only non-empty protected-environment credentials and permits unsigned rehearsal artifacts. The About page independently queries GitHub from React code and only reports a newer tag; Tauri has no updater plugin or signed endpoint configuration.

This change crosses release automation, the React service boundary, Web/Tauri adapters, the native `desktop` context, `operations`, Tauri configuration, and settings UI. It must preserve the active checksum change, keep credentials out of PR jobs and fixtures, support current Windows x64/macOS x64+arm64/Linux x64 targets, and remain testable without production private keys.

## Goals / Non-Goals

**Goals:**

- Make a stable public release fail closed unless Windows publisher verification and macOS signing/notarization/stapling verification succeed.
- Keep manual rehearsal non-publishing and capable of exercising explicit unsigned and ephemeral-test-key branches.
- Publish Tauri updater artifacts plus signed stable/preview metadata from the existing build outputs.
- Give desktop users a non-blocking, signed check/download/install/restart flow with downgrade and channel guards.
- Preserve an equivalent deterministic Web/mock contract and complete localized UI states.
- Provide negative security tests and deterministic policy/manifest performance measurements.

**Non-Goals:**

- Implement roadmap 07 Mission Control or roadmap 08's broader sandbox/supply-chain program.
- Add a detached Linux GPG/minisign scheme; SHA-256, SBOM, and GitHub attestations remain integrity evidence and are not called code signing.
- Add runtime-configurable update URLs, arbitrary feeds, silent unattended installation, or a general background scheduler.
- Provision real vendor credentials or claim native verification on a platform not actually executed.

## Decisions

### 1. Extend the existing workflow and capabilities

`.github/workflows/package.yml` remains the single release pipeline. Platform signing and updater artifact collection are inserted around its existing build/publish stages, while the checksum/SBOM/attestation implementation remains shared with `publish-verifiable-checksums`. `desktop-release-delivery` is modified; `signed-desktop-auto-update` is new because runtime update policy and lifecycle are a distinct product contract.

Alternative: create a second signing workflow. Rejected because artifact identity, version gates, and publication atomicity would drift.

### 2. Stable releases fail closed; rehearsals and prereleases disclose their status

Stable tagged releases require configured signing inputs and successful verification. Preview tags may remain unsigned only when release notes and machine-readable status identify every unsigned platform; manual runs never publish. Windows uses a protected-environment managed signing command/provider and PowerShell Authenticode verification. macOS uses Tauri's Developer ID integration, notarization credentials, `codesign --verify`, `spctl`, `stapler validate`, and notary evidence before upload. Linux retains integrity controls.

Alternative: allow unsigned stable releases for backward compatibility. Rejected because it makes the strongest channel indistinguishable from rehearsal delivery.

### 3. Tauri v2 updater is the only native installer

The compatible `tauri-plugin-updater` and `tauri-plugin-process` releases are composed in bootstrap. Tauri config embeds an HTTPS endpoint template and the public verification key. The private updater key exists only in the protected release environment. Ordinary settings can select `stable` or `preview`, but cannot replace the host or key. TLS failures propagate unchanged and no insecure client option exists.

Alternative: manually download and replace binaries in Rust. Rejected because it duplicates Tauri's signature verification and platform installer handling.

### 4. Existing `desktop` and `operations` contexts own the lifecycle

The `desktop` domain defines channel/version/downgrade invariants and the application service coordinates updater ports. Commands start check/download-install operations and return a stable operation id before network/download work completes. Observable snapshots retain current data across queued, checking, available, downloading, ready-to-restart, up-to-date, and failed states. Unified operation logs receive redacted semantic events. No new bounded context or SQLite table is introduced; auto-check/channel reuse desktop app settings with defaults.

Alternative: call the updater directly from the About component. Rejected because it violates runtime isolation and long-running-operation rules.

### 5. One frontend contract, two adapters

`AgentService` exposes update snapshot, preferences, check, install, and restart actions. The Tauri adapter only maps IPC DTOs. The Web adapter uses a deterministic in-memory operation simulator, including progress and failures, with no filesystem/network/native effects. About consumes the injected runtime service and keeps the previous snapshot visible during refresh/download.

### 6. SemVer/channel policy is explicit and downgrade-safe

Stable clients accept only greater stable semantic versions. Preview clients may accept greater stable or prerelease versions according to SemVer precedence. Stable clients never consume prerelease metadata. Equal/lower versions are rejected before installation except in an explicitly compiled desktop test path; no ordinary setting enables downgrade. Invalid versions/manifests fail closed.

### 7. Rehearsal and tests contain no production secret

Repository fixtures contain only public keys and signatures made with an ephemeral test key generated during the test. Workflow structural tests assert secrets appear only in tag/protected release jobs, manual runs cannot publish, updater JSON is schema-valid, tampering fails verification, and unsigned branches are labeled. Performance evidence uses batch policy evaluation throughput/linear structural budgets rather than wall-clock UI timing.

## Risks / Trade-offs

- [Managed Windows signing providers have different CLIs] → expose one reviewed signing command boundary through protected environment configuration and verify the resulting Authenticode publisher/timestamp independently of provider output.
- [Apple notarization is externally available and can be slow] → bound job time, require verification before publication, and report `BLOCKED` when credentials/service are unavailable rather than claiming pass.
- [Updater endpoints must exist before clients can consume them] → publish versioned channel manifests atomically with artifacts and keep the prior valid manifest usable until publication completes.
- [Install/restart cannot be fully exercised safely in ordinary unit tests] → use port doubles and a local mock server in desktop test mode; reserve real cross-platform replacement evidence for native CI.
- [Adding updater binaries increases bundle size] → record release-binary/package size deltas and deterministic manifest/policy benchmark evidence.
- [Existing settings lack update fields] → use optional persisted keys with stable defaults so no destructive database migration is required.

## Migration Plan

1. Add public updater configuration, dependencies, desktop domain/application ports, adapters, and tests while release publishing remains unchanged.
2. Add updater artifact generation and rehearsal validation using ephemeral keys.
3. Configure protected environment secrets and platform signing providers outside the repository.
4. Enable stable fail-closed publication gates and publish stable/preview channel metadata only after all platform verification succeeds.
5. Existing installations default to the channel derived from their build version (`preview` for prerelease builds, otherwise `stable`) and auto-check disabled; users may opt in.
6. Rollback by publishing no new channel manifest and reverting the client feature. Previously published signed manifests/artifacts remain immutable and existing settings are harmless unknown keys to older clients.

## Open Questions

- Production Windows certificate/provider selection and exact publisher identity remain deployment prerequisites; repository code validates a configured expected identity without embedding a private key.
- Native signing/notarization results for Windows and macOS can only be marked after those protected runners execute.
