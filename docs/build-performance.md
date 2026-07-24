# Native build performance

VaneHub uses target-scoped linker policies for the native targets exercised by the repository:

| Target | Linker policy | Required tools |
| --- | --- | --- |
| `x86_64-pc-windows-msvc` | Rust LLD | Stable Rust MSVC toolchain and MSVC libraries |
| `x86_64-unknown-linux-gnu` | Clang driving mold | `clang`, `mold`, and the documented Tauri Linux prerequisites |
| Other targets | Platform default | Platform-specific Tauri prerequisites |

The configuration intentionally does not fall back when a declared fast linker is missing. A missing tool fails the build instead of silently producing incomparable local and CI results.

## Verify the linker

On Windows PowerShell:

```powershell
$rustLld = Join-Path (rustc --print sysroot) "lib/rustlib/x86_64-pc-windows-msvc/bin/rust-lld.exe"
Test-Path -LiteralPath $rustLld
cargo build --manifest-path src-tauri/Cargo.toml --verbose
```

The final verbose `rustc` invocation identifies `rust-lld.exe` as its linker.

On Linux:

```bash
clang --version
mold --version
cargo build --manifest-path src-tauri/Cargo.toml --verbose
readelf -p .comment src-tauri/target/debug/vanehub-ai | grep mold
```

The `.comment` section provides artifact-level confirmation that mold linked the executable.

## Release profile

Distributable builds use:

```toml
[profile.release]
opt-level = 3
lto = "thin"
codegen-units = 1
strip = "debuginfo"
```

ThinLTO and one codegen unit enable whole-program optimization but can increase release link time. Debuginfo is stripped from distributed binaries while more useful symbol behavior is retained than with full symbol stripping. These settings can change executable and package sizes; they do not guarantee a reduction on every target.

The release profile does not compile out operational diagnostics. Unified logging continues to support redacted `error`, `warn`, `info`, and `debug` events. `#[cfg(debug_assertions)]` is reserved for behavior with no production contract; it must not guard unified-log levels, serialization, redaction, persistence, or required operational events.

## Worktrees and comparable measurements

`src-tauri/target/` is ignored and local to each Git worktree by default. The first Cargo command in a new worktree therefore compiles the dependency graph from scratch. That cold-build time measures dependency compilation and must not be presented as linker performance.

For a comparable relink measurement:

1. Use the same source state, host, target triple, Rust toolchain, and power conditions.
2. Complete one cold build so third-party dependencies exist.
3. Run `cargo clean --manifest-path src-tauri/Cargo.toml -p vanehub-ai`.
4. Time `cargo build --manifest-path src-tauri/Cargo.toml`.
5. Record linker identity separately from elapsed time.

Release executable and package comparisons similarly use the same source state and target. Dependency installation time is excluded from package build timing. If a comparable baseline cannot be produced, record the absolute optimized sizes and the missing comparison without claiming a reduction.

## Measurement record

Environment captured on 2026-07-23/24:

- Source base: commit `9ef6bd56c74cbbb42b9c2a7e92748b42ad53f575` plus the uncommitted `optimize-rust-build-and-release` implementation; both controlled linker arms used the identical working-tree state
- Branch/worktree: `codex/package-distribution`
- OS: Windows `10.0.26200.0`
- Logical processors: 8
- Baseline Rust: `rustc 1.97.0 (2d8144b78 2026-07-07)`
- Optimized-run Rust: `rustc 1.97.1 (8bab26f4f 2026-07-14)`
- Cargo: `cargo 1.97.1 (c980f4866 2026-06-30)` during the optimized run
- Host target: `x86_64-pc-windows-msvc`
- Node/npm: `v24.15.0` / `11.18.0`
- Initial `src-tauri/target`: absent

| Measurement | Default linker/profile baseline | Optimized linker/profile |
| --- | ---: | ---: |
| Cold dev build | 622.303 s | Not repeated; linker comparison uses cached dependencies |
| VaneHub crate rebuild with cached dependencies | 216.875 s | 232.024 s with Rust LLD |
| Windows x64 package | Failed after 1126.2 s during an external concurrent Rust toolchain update | Completed after the stable target metadata was repaired |
| Release executable | Not produced by the interrupted baseline package | 30,486,016 bytes (29.074 MiB) |
| MSI package | Not produced by the interrupted baseline package | 13,713,408 bytes (13.078 MiB) |
| NSIS package | Not produced by the interrupted baseline package | 9,018,092 bytes (8.600 MiB) |

Because no comparable pre-change release executable, MSI, or NSIS artifact was produced, the optimized sizes are absolute measurements only. They do not demonstrate that the release profile reduced distributable size.

The optimized crate rebuild was 15.149 seconds (7.0%) slower than the baseline sample. The Rust patch version changed between samples, so this is directional evidence rather than a controlled benchmark. A separate Rust 1.97.1 linker-switch experiment took 300.550 seconds with `link.exe` and 520.270 seconds with Rust LLD, but changing the linker invalidated Cargo fingerprints and relinked the dependency graph; those values are not incremental-link measurements.

### Controlled Windows linker A/B

The controlled comparison used commit `9ef6bd56c74cbbb42b9c2a7e92748b42ad53f575`, Rust 1.97.1, the same host, and separate prewarmed target directories. Each measured run moved only the generated top-level `debug/vanehub-ai.exe` out of its target directory before invoking `cargo build`; cached dependencies and linker-specific Cargo fingerprints remained in place. The default arm explicitly overrode the repository configuration with `link.exe`; verbose output confirmed `rust-lld.exe` for the optimized arm.

| Idle matched run | `link.exe` | `rust-lld.exe` | LLD delta |
| --- | ---: | ---: | ---: |
| Prewarmed VaneHub package relink | 2.858 s | 3.177 s | +0.319 s (+11.2%) |

No other Rust processes appeared during either timed command. Two earlier matched runs were retained only as diagnostics: one was contaminated by unequal external Rust activity, while another measured 6.841 seconds for `link.exe` and 9.700 seconds for Rust LLD with background Rust processes present. The clean idle pair is the reportable result: Rust LLD did not improve steady-state relink time on this host. The target-scoped configuration remains useful for deterministic linker selection, but it is not claimed as a measured Windows speedup.

The optimized release build completed in 1414.416 seconds. Its final `rustc` invocation contained `-C linker=rust-lld.exe`, `-C opt-level=3`, `-C lto=thin`, `-C codegen-units=1`, and `-C strip=debuginfo`.

The baseline package failure coincided with other `rustup` processes replacing the active stable toolchain; subsequent `rustc` and `cargo` proxy calls reported that their binaries were temporarily not applicable. An intermediate attempt reached the bundle stage only by running:

```powershell
npx tauri bundle --target x86_64-pc-windows-msvc --ci --no-sign
```

After the shared toolchain became idle, stable was repaired/reinstalled and `rustup target list --installed` again reported `x86_64-pc-windows-msvc`. The supported command then passed Tauri's target preflight and completed the Cargo release build plus both WiX and NSIS bundling stages:

```powershell
npm run package:windows:x64
```

The command outlived the 20-minute orchestration capture window, so its wall time is intentionally not reported as a controlled package benchmark. The child process chain was monitored through natural completion. The regenerated artifacts are identified by:

| Artifact | SHA-256 |
| --- | --- |
| `vanehub-ai.exe` | `441CFA2F6D3EDB30BCB4436FC51E4E778BDA1909F01F0D6C5F6FB1F199B7FFA7` |
| MSI package | `446944305839D58057B5CBFFFA16F644E4FEBC6F6839A849AB5B227FA4D46C6A` |
| NSIS package | `18E3B13B13A8C16C8FA50074A9CA7331308335435AED73541E88E02238BB5B69` |

The earlier rustup failures are environmental failures, not repository build results. The sizes and hashes above come from the final successful full package run.
