# Testing, packaging, and release

Run the repository verification commands appropriate to the change:

```powershell
npm run lint:ci
npm run test
npm run build
cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
cargo check --workspace
cargo clippy --workspace --all-targets -- -D warnings
npm run native:panic:check
cargo test --workspace
openspec validate --specs --strict
```

Copy those flags verbatim. `check`, `clippy`, and `test` take `--workspace` rather than `--manifest-path src-tauri/Cargo.toml`: the repository is a Cargo workspace, and `--manifest-path` covers only the `vanehub-ai` crate, silently skipping members such as `vanehub-permission-hook`. `fmt` is the exception, and CI uses `--manifest-path` for it. `AGENTS.md` is the source of truth for this list.

Documentation changes additionally run:

```powershell
npm run docs:check
npm run docs:test
npm run docs:screenshots:check
npm run docs:build
```

Frontend tests cover pure contracts and visible component behavior. Playwright covers the browser Web/mock runtime; passing it does not claim that the Tauri desktop runtime passed. Native tests cover domain invariants, application port orchestration, persistence/migrations, command mapping, process safety, and lifecycle behavior.

Runtime-affecting desktop changes additionally use:

```powershell
npm run desktop:unit:test
npm run test:desktop
```

`test:desktop` builds and launches an instrumented native Tauri artifact for the current operating system, waits for the real React WebView, invokes the real Rust-backed `get_settings` command, performs a stable navigation interaction, and requests a clean application shutdown. It sets an isolated temporary `VANEHUB_APP_DATA_DIR`; never point that variable at normal user data.

The instrumented artifact enables test-only WebDriver plugins and permissions through the `desktop-e2e` Cargo feature and `src-tauri/tauri.desktop-e2e.conf.json`. Normal packaging commands do not include that feature. Failure evidence is written beneath `test-results/desktop/<run-id>/` from screenshots, driver output, process state, and the existing redacted unified native logs.

Local results apply only to the current platform. CI runs `Desktop Smoke` independently on native Windows, macOS, and Linux runners with matrix fail-fast disabled. Review and report every platform separately as `PASSED`, `FAILED`, `BLOCKED`, or `NOT RUN`; never infer one platform from another. Failed or blocked jobs upload a platform-labelled evidence artifact, while successful jobs do not retain temporary application data.

Packaging targets Windows, macOS, and Linux through Tauri. Signing credentials belong in protected release environments, never in repository configuration or screenshots. See the checked-in [release signing guide](../reference/release-signing.md).

## Test tiers

The test pyramid has five tiers from the bottom up. Each covers a different risk surface, and none substitutes for another — above all, **a passing Playwright run is not a passing desktop run**.

```mermaid
flowchart TD
    Unit["Vitest unit and component tests<br/>pure contracts and visible component behavior"]
    E2E["Playwright e2e<br/>Web/mock runtime only"]
    Rust["Rust native tests<br/>domain invariants / migrations / process safety / lifecycle"]
    Smoke["Desktop Smoke<br/>instrumented desktop artifact, desktop-e2e feature"]
    Matrix["CI matrix<br/>Windows / macOS / Linux<br/>fail-fast: false"]
    Unit --> E2E
    Unit --> Rust
    E2E --> Smoke
    Rust --> Smoke
    Smoke --> Matrix
    Matrix --> Gates{Judged independently per platform}
    Gates -->|passed| PASSED[PASSED]
    Gates -->|failed| FAILED[FAILED]
    Gates -->|blocked| BLOCKED[BLOCKED]
    Gates -->|not run| NOTRUN[NOT RUN]
```

What each tier covers:

- **Vitest** — unit and component tests over pure contracts and visible component behavior. The coverage gate is enforced by `coverage-policy.json`, with a frontend `minimumLines` of 45.2%.
- **Playwright e2e** — covers the browser Web/mock runtime only. Passing it does not claim the Tauri desktop runtime passed.
- **Rust native tests** — cover domain invariants, application port orchestration, persistence and migrations, command mapping, process safety, and lifecycle behavior. The overall gate is `minimumLines = 67%`, with three `criticalGroups` each requiring `minimumLines = 80%`:
  - `sqlite-transactions`: `sessions/infrastructure/transactions.rs`, `platform/database/mod.rs`, `platform/database/migrations/**.rs`
  - `agent-startup-and-terminal-control`: `agent_runtime/application/terminal_service.rs`
  - `mcp-routing`: `tooling/mcp/infrastructure/relay.rs`
- **Desktop Smoke** — an instrumented desktop artifact enables test-only WebDriver plugins and permissions through the `desktop-e2e` Cargo feature and `src-tauri/tauri.desktop-e2e.conf.json`, which normal packaging commands never include. It builds and launches the instrumented Tauri artifact for the current operating system, waits for the real React WebView, invokes the real Rust-backed `get_settings` command, performs a stable navigation, and requests a clean shutdown. Local results apply to the current platform only and must not be extrapolated.
- **CI matrix** — `Desktop Smoke` runs independently on native Windows, macOS, and Linux runners with matrix `fail-fast` disabled. Review and report each platform separately as `PASSED`, `FAILED`, `BLOCKED`, or `NOT RUN`; never infer one platform from another.

Supplementary scripts must run when a change lands in their area: `npm run test:coverage` (which CI uses in place of `npm run test`), `npm run coverage:policy:test`, `npm run version:unit:test`, and `npm run contracts:check`. UI behavior changes run `npx playwright test`; runtime, startup-chain, or IPC changes run `npm run desktop:unit:test` and `npm run test:desktop`.

## The release process

A release is one synchronized packaging and signing pass across three platforms. The version number must agree in `package.json`, `src-tauri/Cargo.toml`, and `src-tauri/tauri.conf.json`, guarded by `version:check`.

```mermaid
sequenceDiagram
    participant Dev as Releaser
    participant Sync as Version sync
    participant Check as version:check + full verification
    participant Tag as git tag
    participant PKG as Three-platform package job
    participant Win as Windows runner
    participant Mac as macOS runner
    participant Lin as Linux runner
    participant Pub as publish job
    Dev->>Sync: Sync the version number<br/>package.json / Cargo.toml / tauri.conf.json
    Sync->>Check: version:check + lint:ci + test + build<br/>+ cargo fmt / check / clippy / test<br/>+ openspec validate --specs --strict
    Check-->>Dev: Continue only when everything is green
    Dev->>Tag: Create the tag
    Tag->>PKG: Trigger the three-platform package workflow
    par Windows
        PKG->>Win: NSIS .exe<br/>signed
    and macOS
        PKG->>Mac: .dmg<br/>notarize + staple
    and Linux
        PKG->>Lin: .deb + AppImage
    end
    Win-->>Pub: Upload artifacts
    Mac-->>Pub: Upload artifacts
    Lin-->>Pub: Upload artifacts
    Pub->>Pub: Generate SHA256SUMS<br/>generate an SPDX SBOM<br/>generate attestations<br/>assemble release notes
    Pub-->>Dev: Release complete
```

What matters in a release:

- **Version synchronization** — the version must agree across `package.json`, `src-tauri/Cargo.toml`, and `src-tauri/tauri.conf.json`. `scripts/check-version-sync.mjs` cross-checks them, and `version:unit:test` is its unit test.
- **Full verification comes first** — every verification command at the end of `AGENTS.md` must pass before the tag, plus `version:check`.
- **Three platform artifacts** — Windows produces a signed NSIS `.exe`; macOS produces a `.dmg` that is notarized and stapled; Linux produces a `.deb` and an AppImage.
- **The publish artifact list** — `SHA256SUMS` (a per-file sha256, verified to contain no duplicate hashes), an SPDX SBOM, attestations, and release notes.
- **Updater signing** — the auto-updater signs with `TAURI_SIGNING_PRIVATE_KEY` and its password. The signing key belongs to the protected release environment and never appears in repository configuration or screenshots. An empty key takes the rehearsal-only path and produces no distributable update signature.
- **Signing credential isolation** — signing credentials are injected only in the CI protected environment. A normal local packaging command carries neither the `desktop-e2e` feature nor the signing key.

Packaging and signing details live in `src-tauri/ARCHITECTURE.md` and the [release signing guide](../reference/release-signing.md); CI orchestration lives in `.github/workflows/ci.yml` and `.github/workflows/package.yml`.

## Key scripts and commands

The scripts and gates along the test and release path:

- **Vitest** — `package.json` defines `test: "vitest run"` and `test:coverage: "vitest run --coverage --maxWorkers=4 --testTimeout=15000"`, configured in `vite.config.ts`, which excludes `tests/docs/**` and `tests/e2e/**` and writes coverage to `./coverage/frontend`.
- **Playwright** — `playwright.config.ts` sets `testDir: "./tests/e2e"` with a single chromium project, and its `webServer` starts `npm run dev` on port 5174.
- **Desktop native tests** — `npm run test:desktop` runs `node scripts/test-desktop.mjs all`, split into `test:desktop:build` and the per-layer scripts. `test-desktop.mjs` builds the instrumented debug artifact with `--features desktop-e2e --config src-tauri/tauri.desktop-e2e.conf.json`, starts the real React WebView, invokes `get_settings`, navigates, and exits cleanly, under an isolated temporary `VANEHUB_APP_DATA_DIR`, writing failure evidence to `test-results/desktop/<run-id>/`.
- **`desktop:unit:test`** — runs the `scripts/desktop-*.node-test.mjs` automation-boundary tests.
- **The coverage gate** — `coverage-policy.json` sets frontend `minimumLines: 45.2` and native `minimumLines: 67`, with three `criticalGroups` at 80 (`sqlite-transactions`, `agent-startup-and-terminal-control`, `mcp-routing`), checked by `scripts/check-coverage-policy.mjs`.
- **CI Desktop Smoke** — `.github/workflows/ci.yml` runs the `desktop-smoke` job on a windows / macos / ubuntu matrix with `fail-fast: false`, using `xvfb-run -a npm run test:desktop` on Linux and labelling failure evidence per platform.
- **Packaging targets** — `package.json` defines six: `package:windows:{x64,arm64}`, `package:macos:{x64,arm64}`, and `package:linux:{x64,arm64}`, each preceded by `sidecar:prepare -- --release --target=...`.
- **Version synchronization** — `scripts/check-version-sync.mjs` requires the three version declarations to agree, with `version:unit:test` as its unit test.
- **Signing credentials** — the protected `release` environment holds the credentials (`APPLE_CERTIFICATE`, `APPLE_SIGNING_IDENTITY`, `TAURI_SIGNING_PRIVATE_KEY`, `WINDOWS_CERTIFICATE`, and others). The environment is chosen by `github.ref_type == 'tag' ? 'release' : 'build-preview'`, and the updater uses `TAURI_SIGNING_PRIVATE_KEY` to produce `createUpdaterArtifacts`, with the public key embedded in `tauri.conf.json`.
