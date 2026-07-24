## Context

The native application is a single Tauri 2 Rust crate with a large dependency graph spanning SQLite, async runtimes, networking, telemetry, terminal handling, and platform integrations. No repository Cargo linker configuration or custom release profile currently exists. Native commands run from the repository root, so a root `.cargo/config.toml` can consistently affect direct Cargo commands and Tauri builds.

The supported release matrix currently builds Windows x86_64 MSVC, macOS ARM64, and Linux x86_64 GNU packages. CI also checks Windows and macOS natively. A newly created Git worktree has its own ignored `src-tauri/target` directory, so its first build is a cold dependency compilation; a linker change affects final and incremental linking but does not eliminate that cold-build cost.

The unified logging contract requires production support for `error`, `warn`, `info`, and `debug`. Existing debug-level events describe operational behavior such as ignored IM events and successful opener discovery, so debug log severity must not be treated as synonymous with development-only code.

This change affects repository build configuration and the Rust/native build path only. It does not alter React services, the Tauri frontend adapter, the Web/mock adapter, Tauri commands, or SQLite/runtime behavior.

## Goals / Non-Goals

**Goals:**

- Select and reproducibly exercise target-scoped Windows and Linux linkers, then report controlled relink results without presuming they improve on every host.
- Optimize release binaries with an explicit, reviewable Cargo release profile and measure artifact size without presuming it decreases.
- Keep CI and packaging environments reproducible by provisioning every non-toolchain linker dependency.
- Measure incremental build time before and after the linker change, measure release artifact size where a valid baseline is available, and label unavailable comparisons explicitly.
- Preserve production diagnostic behavior and document when `#[cfg(debug_assertions)]` is appropriate.
- Make cold-worktree and incremental-build behavior explicit in maintainer documentation.

**Non-Goals:**

- Changing application features, frontend/runtime service boundaries, database behavior, or package formats.
- Introducing a shared Cargo target directory, `sccache`, or a remote build cache.
- Requiring mold for unsupported Linux cross-compilation targets.
- Replacing the native Apple linker or changing macOS signing/notarization.
- Compiling production debug-severity operational logs out of the application.
- Guaranteeing any build-time or binary-size improvement across different machines.
- Claiming distributable-size reduction when no comparable pre-change artifact exists.

## Decisions

### 1. Use target-scoped repository linker configuration

Add root Cargo target configuration for the targets that the repository actually validates:

```toml
[target.x86_64-pc-windows-msvc]
linker = "rust-lld.exe"

[target.x86_64-unknown-linux-gnu]
linker = "clang"
rustflags = ["-C", "link-arg=-fuse-ld=mold"]
```

Windows uses the `rust-lld.exe` shipped with the installed Rust MSVC toolchain, avoiding a separate LLVM installation. Linux uses Clang as the linker driver and mold as the ELF linker. The Linux configuration is limited to x86_64 GNU so it cannot silently impose host-linker assumptions on ARM64 or musl cross-builds.

The controlled Windows measurement recorded for this change found Rust LLD 11.2% slower than `link.exe` for the clean prewarmed relink pair on the measured host. The configuration is therefore retained as a reproducible, validated linker policy rather than claimed as a demonstrated Windows speedup. Broader evidence can justify revisiting the Windows selection in a follow-up change.

The project will not silently fall back to the default linker on these declared targets. A silent fallback would make local and CI performance inconsistent and could hide missing build prerequisites. CI workflows and documentation will provision or declare the required tooling.

Alternatives considered:

- User-global Cargo configuration was rejected because CI, packaging, and contributors would not share a reproducible policy.
- A wrapper script that probes and falls back was rejected because `npm run tauri:dev`, direct Cargo commands, and IDE integrations could bypass it.
- Applying mold or LLD through global `RUSTFLAGS` was rejected because it can contaminate unsupported targets and makes the active linker harder to audit.

### 2. Prefer a balanced release profile over maximum size reduction

Define the following explicit release policy in `src-tauri/Cargo.toml`:

```toml
[profile.release]
opt-level = 3
lto = "thin"
codegen-units = 1
strip = "debuginfo"
```

ThinLTO is selected instead of fat LTO because it retains cross-crate optimization with a lower linking-time penalty. One codegen unit gives LLVM the best opportunity to optimize the application crate in the infrequent packaging path. `strip = "debuginfo"` removes debug information from shipped binaries while retaining more useful symbol/backtrace behavior than `strip = true`/`"symbols"`. These choices can change distributable size, but the change does not claim a size reduction without comparable pre-change artifacts.

`opt-level = 3` is already Cargo's release default, but it is stated explicitly so the complete distribution policy is reviewable in one place.

Alternatives considered:

- Fat LTO was rejected for the first iteration because release build latency would increase substantially without measured evidence that its additional size or runtime benefit is material.
- `opt-level = "s"` or `"z"` was deferred because this desktop runtime performs parsing, networking, telemetry, and local orchestration; a benchmark should precede a global speed-for-size trade.
- `strip = "symbols"` was rejected initially because it can make production backtraces and crash investigation substantially less useful.
- `panic = "abort"` was deferred because it changes failure behavior rather than being a purely build-time optimization.

### 3. Preserve operational debug logs in release builds

`#[cfg(debug_assertions)]` may guard only behavior that has no production contract, such as developer-only UI, local probes, or expensive assertions. It must not guard unified logging severity variants, serialization mappings, redaction paths, or operational debug events.

The implementation will audit current conditional compilation and debug-severity call sites. If no genuinely development-only feature exists, it will not add a gratuitous conditional block merely to demonstrate the mechanism. Existing `cfg(test)` isolation and the release-only Windows subsystem attribute remain valid.

Alternatives considered:

- Compiling out all debug-severity records was rejected because `unified-log-management` explicitly requires the level and several production flows use it.
- A new user-facing log-level setting was rejected because the current logging specification explicitly excludes that control.

### 4. Measure cold, incremental, and release outcomes separately

Verification will record:

- environment and toolchain identity;
- cold build context for the fresh worktree;
- a representative incremental Rust rebuild/relink time before and after linker selection;
- release executable size and packaged artifact sizes before and after the release profile when comparable pre-change artifacts are available;
- active linker evidence on Windows and Linux where the host supports it.

Measurements are evidence attached to implementation verification, not hard cross-machine performance requirements. The same source state and comparable target/cache state must be used for each before/after pair. A neutral or negative result must be reported as such. If an environmental failure prevents a valid baseline, verification must identify the missing comparison and must not infer an improvement from the optimized artifact alone.

Alternatives considered:

- A fixed percentage gate was rejected because GitHub-hosted runner load, filesystem cache state, and dependency cache state make such a gate flaky.
- Measuring only a cold build was rejected because dependency compilation would obscure the linker-specific effect.

### 5. Provision linkers in every affected workflow

Linux CI and package jobs will install Clang and mold before invoking Cargo/Tauri. Windows native validation will verify the toolchain-provided LLD by producing at least one linked native artifact rather than relying only on `cargo check`. Packaging continues to use the release profile through the existing Tauri build command.

macOS keeps its current native linker and validation path. Unsupported targets retain their platform defaults because the Cargo configuration is exact-target scoped.

## Risks / Trade-offs

- [ThinLTO and one codegen unit increase release build time] → Limit them to the release profile and record packaging duration alongside size results.
- [A missing mold or Clang installation breaks Linux builds] → Install both in CI/package workflows and document them as local prerequisites with a direct verification command.
- [LLD differs subtly from Microsoft's linker for native libraries] → Run a linked Windows validation artifact and the Windows package build before accepting the change.
- [Changing linker flags invalidates Cargo caches] → Treat the first optimized build as a one-time cold build and compare representative subsequent relinks.
- [A declared candidate linker is slower on a measured host] → Retain the measured direction in documentation, make no speedup claim, and revisit linker selection only through a follow-up spec change with broader controlled evidence.
- [Stripping reduces post-mortem visibility] → Strip debuginfo rather than all symbols and preserve unified production diagnostics.
- [A pre-change release artifact cannot be produced] → Record optimized artifact sizes as absolute values, label the baseline unavailable, and make no size-reduction claim.
- [Fresh worktrees appear unaffected because dependency compilation dominates] → Document separate cold and incremental measurements and avoid claiming linker gains from cold-build totals alone.
- [Repository Cargo configuration affects all contributors] → Scope configuration to the two validated target triples and document unsupported-target behavior.

## Migration Plan

1. Capture baseline toolchain details, incremental build timing, release executable size, and package size where the current host permits; explicitly record unavailable baselines.
2. Add the target-scoped Cargo linker configuration and provision matching CI/package prerequisites.
3. Add the explicit release profile and audit conditional compilation against unified logging requirements.
4. Run native checks/tests, linked Windows/Linux validation as available, local packaging on the current host, and strict OpenSpec validation.
5. Record before/after results and update build/release documentation.
6. Roll back by removing the target linker tables and release profile; no data or runtime migration is required.

## Open Questions

- Whether later evidence justifies `strip = "symbols"`, `opt-level = "s"`, or fat LTO for a dedicated size-minimized release profile.
- Whether controlled measurements on additional Windows hosts justify retaining Rust LLD as the default MSVC linker or returning to `link.exe`.
- Whether cross-worktree compilation should later gain a separate `sccache` or shared-target proposal after linker gains are measured.
- Whether Linux ARM64 packaging should adopt mold after a native or controlled cross-build environment is added to the supported matrix.
