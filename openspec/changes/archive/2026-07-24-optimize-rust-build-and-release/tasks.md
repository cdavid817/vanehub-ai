## 1. Baseline and Tooling Audit

- [x] 1.1 Record the current host, Rust/Cargo versions, target triple, linker availability, worktree cache state, and the exact commands used for comparable measurements.
- [x] 1.2 Capture baseline cold-build context, representative incremental native rebuild/relink time, and locally available pre-change release/package artifact sizes; explicitly record any baseline artifact that cannot be produced.
- [x] 1.3 Define how Windows LLD and Linux mold usage will be verified from build output or produced artifacts without relying on timing alone.

## 2. Cargo Build Configuration

- [x] 2.1 Add root target-scoped Cargo configuration for `rust-lld.exe` on `x86_64-pc-windows-msvc` and `clang` plus mold on `x86_64-unknown-linux-gnu`, leaving undeclared targets on their platform defaults and treating measured performance as evidence rather than an assumed gain.
- [x] 2.2 Add the explicit native release profile with `opt-level = 3`, ThinLTO, one codegen unit, and debuginfo stripping.
- [x] 2.3 Audit existing `cfg(debug_assertions)` and production debug-level logging paths, retaining every unified-log severity, mapping, redaction path, and required operational diagnostic in release builds.
- [x] 2.4 Add or update focused native tests or architecture checks that verify release-compatible unified-log level serialization and prevent development-only gating from removing the production logging contract.

## 3. CI and Packaging Integration

- [x] 3.1 Update Linux native CI provisioning to install or verify Clang and mold before compilation and link at least one native validation artifact.
- [x] 3.2 Update Windows native CI validation to confirm the Rust-toolchain-provided LLD is selected and link at least one MSVC native artifact.
- [x] 3.3 Update Linux desktop packaging provisioning to install or verify Clang and mold before the Tauri release build.
- [x] 3.4 Verify that Windows, Linux, and macOS package commands continue using the shared Cargo release profile while unsupported target triples do not inherit incompatible linker flags.

## 4. Documentation and Evidence

- [x] 4.1 Document Windows and Linux linker prerequisites, verification commands, supported target scope, and failure behavior for missing tools.
- [x] 4.2 Document release profile choices, ThinLTO/link-time tradeoffs, debuginfo stripping, and preservation of operational debug-level unified logs.
- [x] 4.3 Document fresh-worktree cold builds, worktree-local ignored target output, and the distinction between dependency compilation and incremental linker performance.
- [x] 4.4 Capture optimized incremental relink, release executable, and package measurements using the baseline method, then record a reviewable comparison with environment metadata, the measured direction, and any unavailable baseline without claiming unsupported improvement.

## 5. Verification

- [x] 5.1 Run `npm run lint`, `npm run test`, and `npm run build`.
- [x] 5.2 Run `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`, `cargo check --manifest-path src-tauri/Cargo.toml`, `cargo clippy --manifest-path src-tauri/Cargo.toml`, and `cargo test --manifest-path src-tauri/Cargo.toml`.
- [x] 5.3 Run a linked native development build and `cargo build --release --manifest-path src-tauri/Cargo.toml`, confirming the expected linker and release profile on the current host.
- [x] 5.4 Run the supported current-host Tauri package command and verify that expected bundle artifacts are produced and measured.
- [x] 5.5 Run `openspec validate optimize-rust-build-and-release --strict` and `openspec validate --specs --strict`.
