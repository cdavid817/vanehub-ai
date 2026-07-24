# Desktop release signing

The `Package Desktop Apps` workflow uses an unprivileged `build-preview` environment for manual branch runs and a protected `release` environment for tag builds. Manual runs build review artifacts without signing secrets; tags matching `v<package-version>` additionally publish one GitHub Release after every platform build succeeds.

## Release sequence

1. Synchronize the version in `package.json`, `src-tauri/Cargo.toml`, and `src-tauri/tauri.conf.json`.
2. Run `npm run version:check` and the full validation suite.
3. Merge the reviewed version change to `main`.
4. Create and push an annotated `v<version>` tag from the reviewed commit.
5. Approve the `release` environment deployment if environment reviewers are configured.
6. Verify packages, `SHA256SUMS`, the SPDX SBOM, GitHub artifact attestations, and generated release notes.

The publish job cannot run until all Windows, macOS, and Linux jobs finish successfully. It uses short-lived GitHub OIDC identity for attestations and does not require a stored GitHub token.

## Native build profile

Desktop packages use the shared Cargo release profile declared in `src-tauri/Cargo.toml`: optimization level 3, ThinLTO, one codegen unit, and debuginfo stripping. ThinLTO and a single codegen unit can extend release link time while enabling whole-program optimization and changing distributable size; they do not guarantee a smaller package on every target.

Windows x64 builds use the Rust-toolchain-provided LLD linker. Linux x64 builds require Clang and mold; the package workflow verifies both before compilation. Other targets retain their platform-default linker unless a target-specific policy is added and validated.

Debuginfo stripping does not remove VaneHub's operational `debug` log level. Release builds continue to persist redacted `error`, `warn`, `info`, and `debug` events through unified logging. Build prerequisites, verification commands, worktree cache behavior, and measurement evidence are documented in `docs/build-performance.md`.

The current measurement record contains optimized Windows executable, MSI, and NSIS sizes but no comparable pre-change package artifacts because the baseline package was interrupted by an external Rust toolchain update. Those absolute sizes do not establish a measured size reduction.

## GitHub environment secrets

Store credentials only as secrets on the `release` environment. Never place their values in repository variables, workflow files, issues, logs, or artifacts.

| Secret | Purpose |
| --- | --- |
| `APPLE_CERTIFICATE` | Base64-encoded Apple Developer ID certificate bundle |
| `APPLE_CERTIFICATE_PASSWORD` | Certificate bundle password |
| `APPLE_SIGNING_IDENTITY` | Apple signing identity used by Tauri |
| `APPLE_ID` | Apple account used for notarization |
| `APPLE_PASSWORD` | App-specific Apple password |
| `APPLE_TEAM_ID` | Apple developer team identifier |
| `TAURI_SIGNING_PRIVATE_KEY` | Tauri updater artifact signing key, when updater publishing is enabled |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | Password for the Tauri updater signing key |

The repository does not currently define a Windows Authenticode provider. Before claiming signed Windows binaries, choose a managed certificate or key-vault provider, add its authentication at the `release` environment boundary, and verify the signature in the workflow. Do not export a long-lived private key merely to make CI convenient.

## Environment protection

The environment should allow deployment only from protected `v*` tags. Add a human reviewer when a second trusted maintainer is available. A required self-review is intentionally not configured because it would make a single-maintainer repository impossible to release.

Until valid platform credentials are configured, packages may be unsigned. Release notes must say so clearly; the checksums, SBOM, and GitHub attestations establish integrity but do not replace operating-system code signing or Apple notarization.
