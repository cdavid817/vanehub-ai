## 1. Contract and policy foundation

- [x] 1.1 Add shared TypeScript update DTOs and semantic-version/channel admission tests, including malformed, prerelease, equal-version, and downgrade negative cases
- [x] 1.2 Extend `AgentService` with update snapshot, preference, check, download/install, and restart contracts without component-level Tauri access
- [x] 1.3 Add deterministic update manifest validation and policy benchmark/structural budget evidence

## 2. Native desktop update lifecycle

- [x] 2.1 Add compatible Tauri v2 updater/process dependencies, HTTPS endpoint, embedded public key, and least-privilege updater capabilities
- [x] 2.2 Implement update invariants and application ports in the existing `desktop` bounded context with dependency-free domain tests
- [x] 2.3 Implement Tauri updater infrastructure and backend-managed asynchronous check/download-install operations with progress, safe errors, recovery, and redacted unified logs
- [x] 2.4 Register narrowly mapped desktop update commands and add command DTO/error compatibility plus tampered-signature/TLS/downgrade negative tests
- [x] 2.5 Persist auto-check/channel settings with backward-compatible defaults and test existing-store migration behavior

## 3. Frontend adapters and UI

- [x] 3.1 Implement aligned Tauri and deterministic Web/mock adapters, including queued/checking/available/downloading/failed/ready/restart states and adapter contract tests
- [x] 3.2 Replace the About page's direct GitHub request with service-backed update state while retaining existing content during long-running actions
- [x] 3.3 Add auto-check/channel controls, versions, localized release notes, progress, retry, install, and explicit restart interactions
- [x] 3.4 Add every update key to every registered locale and pass locale parity and hard-coded-visible-text guardrails
- [x] 3.5 Add Vitest and Playwright coverage for update states, channel behavior, failure recovery, and responsive interactions
- [x] 3.6 Capture and inspect stable visual evidence for futuristic/minimal at desktop/narrow widths, covering available, progress, error, and restart-ready states

## 4. Signed release delivery

- [x] 4.1 Extend the existing package workflow with protected target-specific credential forwarding and assertions proving PR/manual jobs cannot access production credentials
- [x] 4.2 Add Windows signing invocation plus Authenticode publisher/timestamp verification and stable fail-closed behavior
- [x] 4.3 Add macOS x64/arm64 Developer ID verification, notarization, stapling, and stapled-ticket verification before artifact collection
- [x] 4.4 Preserve Linux checksum/SBOM/attestation stages and add tests/documentation that distinguish integrity evidence from code signing
- [x] 4.5 Generate Tauri updater artifacts and signed stable/preview metadata from the existing build matrix, with atomic channel publication and invalid/tampered manifest negative tests
- [x] 4.6 Add a manual non-publishing rehearsal using unsigned branches and ephemeral test keys, plus per-platform `PASSED`/`FAILED`/`BLOCKED`/`NOT RUN` evidence
- [x] 4.7 Update release notes and signing/operator verification documentation for artifact -> signature -> signer -> timestamp and macOS build -> verify -> notarize -> staple -> verify -> publish

## 5. Focused verification

- [x] 5.1 Run update policy/manifest unit, signature negative, Web/mock, native domain/application/infrastructure, and migration tests
- [x] 5.2 Run Contract checks and architecture fitness tests for AgentService adapter parity, React/Tauri isolation, Rust dependency direction, and secret/workflow boundaries
- [x] 5.3 Run Playwright behavior and visual suites for both styles and widths
- [x] 5.4 Run desktop unit tests and current-host desktop E2E with a local/mock update endpoint; record unexecuted native platforms honestly
- [x] 5.5 Record deterministic performance benchmark and release binary/package size impact

## 6. Full quality gates

- [x] 6.1 Run `npm run lint:ci`, `npm run test`, and `npm run build`
- [x] 6.2 Run `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`, `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`, `cargo test --manifest-path src-tauri/Cargo.toml`, and `cargo check --manifest-path src-tauri/Cargo.toml`
- [x] 6.3 Run `npm run test:coverage`, `npm run contracts:check`, `npx playwright test`, `npm run desktop:unit:test`, and `npm run test:desktop`
- [x] 6.4 Run `openspec validate --specs --strict` and `openspec validate add-signed-release-auto-update --strict`, then reconcile every acceptance scenario with implementation evidence

## 7. Verification and archive

- [x] 7.1 Complete OpenSpec verification with no critical/warning gaps and record the implementation/test evidence in this change
- [ ] 7.2 Archive `add-signed-release-auto-update`, update the archive index with the required PowerShell script, and rerun post-archive strict validations
