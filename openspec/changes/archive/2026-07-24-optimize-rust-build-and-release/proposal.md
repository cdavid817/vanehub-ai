## Why

Rust development builds in fresh worktrees currently use platform-default linkers and isolated target directories, while release packages use Cargo's default release profile without whole-program optimization or symbol stripping. The project needs an explicit, measurable build policy that evaluates target-scoped linker choices and optimizes distributable builds without weakening production diagnostics, pre-committing to a performance gain before measurement, or making supported builds dependent on undeclared tooling.

## What Changes

- Define supported target-scoped linker behavior for Windows MSVC and Linux native development and validation builds, including required tool provisioning and documented fallback or unsupported-target behavior.
- Define an optimized Cargo release profile using ThinLTO, release optimization, a single codegen unit, and debuginfo stripping.
- Require controlled before/after measurements for representative incremental native builds and packaged artifact sizes where a valid baseline is available, and prohibit improvement claims when evidence is unavailable, neutral, or negative.
- Update native packaging and CI workflows to provision and exercise the declared linker configuration on their supported runners.
- Preserve production `error`, `warn`, `info`, and `debug` unified-log semantics; restrict `#[cfg(debug_assertions)]` to genuinely development-only behavior rather than operational diagnostics.
- Document worktree cache behavior and supported local prerequisites so a fresh worktree's cold build is not mistaken for linker performance.

## Capabilities

### New Capabilities

- `native-build-optimization`: Defines target-scoped linker selection, optimized release profile behavior, measurement expectations, worktree cache guidance, and the boundary between development-only code and production diagnostics.

### Modified Capabilities

- `native-app-packaging`: Require packaged native artifacts to use the declared optimized release profile and document its platform tooling and diagnostic tradeoffs.
- `continuous-integration`: Require native validation runners to provision and verify the linker prerequisites selected for supported Windows and Linux builds.

## Impact

- Desktop/native runtime and its build, CI, and release tooling are affected; the Web runtime and frontend service adapter boundary are unchanged.
- Expected implementation areas include `src-tauri/Cargo.toml`, project Cargo configuration, `.github/workflows/ci.yml`, `.github/workflows/package.yml`, packaging/build documentation, and focused verification scripts or commands.
- Linux builds gain a declared `clang`/mold prerequisite; Windows MSVC builds use the Rust-toolchain-provided LLD linker where validated.
- Release linking can take longer than the current default because of ThinLTO and a single codegen unit. The profile enables whole-program optimization and removes debuginfo, but distributable-size reduction is not claimed without a comparable baseline.
- Unified logging APIs and release diagnostics remain available; this change must not compile away operational debug-level events required by `unified-log-management`.
