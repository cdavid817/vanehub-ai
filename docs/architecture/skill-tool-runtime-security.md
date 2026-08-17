# Skill Tool Runtime Security

## Feature boundary

`skill-tool-module-runtime` is disabled by default. Builds without it retain declarative Skill tool discovery, validation, integrity, trust, and governance. A WebAssembly entry is reported as `module-runtime-unavailable`; it is never hidden or treated as successfully executable.

Enabling the feature adds `wasmtime 47.0.3` with default features disabled and only `cranelift`, `runtime`, and `std`. The runtime does not link Wasmtime WASI, component-model, cache, profiling, pooling, thread, or network integrations. Host capabilities must be supplied through VaneHub-owned typed gateways.

## Dependency review

The engine was selected because it provides maintained Rust embedding APIs for fuel, epoch interruption, store limits, and memory control, has an explicit security policy, and is continuously fuzzed. Version 47.0.3 requires Rust 1.94 and is compatible with the repository Rust 1.97.1 toolchain. It is pinned exactly because sandbox behavior and security fixes must be reviewed before upgrades. Its declared license is `Apache-2.0 WITH LLVM-exception`.

`wasmi 2.0.0-beta.10` was rejected because the available release is a beta and choosing an interpreter does not remove the need for host-capability and resource enforcement. `jsonschema 0.49.9` was evaluated but not added: the repository already implements the deliberately bounded Skill schema subset, while that crate's default features include HTTP/file resolution and TLS. Adding a general resolver would widen the dependency and network surface without implementing a required schema feature.

The lockfile must pass `cargo audit` with no vulnerable package in the selected feature graph. License review must include the feature-enabled graph, and any Wasmtime upgrade must repeat advisory, license, feature, MSRV, hostile-module, and binary-size review. Wasmtime WASI remains prohibited even if a future dependency makes it available transitively.

The 2026-08-17 review resolved only Wasmtime 47.0.3 crates under
`Apache-2.0 WITH LLVM-exception`; `cargo tree` found neither `wasmtime-wasi` nor
`wasi-common`. `cargo audit 0.22.2` reported zero vulnerabilities and 18 allowed
warnings in the existing desktop dependency graph. Those warnings cover legacy
GTK3 bindings, `proc-macro-error`, several `unic-*` crates, `event-listener`
RUSTSEC-2026-0221, and `glib` RUSTSEC-2024-0429; none is introduced by or on the
Wasmtime graph. They remain repository dependency-governance follow-up work and
are not suppressed by this change.

## Verification evidence

- Linux `x86_64-unknown-linux-gnu`, Rust 1.97.1: declarative-only targeted suite
  passed 138 tests; module-enabled targeted suite passed 143 tests.
- Deterministic structural measurements passed the manifest aggregate budget,
  contained filesystem and byte-exhaustion, cancellation accounting, atomic
  host-budget concurrency, and per-Skill module concurrency tests. The checks
  assert exact admitted counts and hard ceilings rather than elapsed milliseconds.
- Playwright passed 137 tests, including futuristic/minimal and desktop/narrow
  visual variants. Linux native Desktop Smoke crossed the real Tauri IPC boundary
  and passed 1/1 scenarios; desktop harness unit tests passed 11/11.
- Native platform status: Linux `PASSED`; Windows `PASSED`; macOS `PASSED`.
  Linux validation ran locally and again in CI. GitHub Actions run `32000007318`
  passed native Desktop Smoke on all three platforms. Its `windows-latest` Native
  Check also passed the native build plus both the declarative-only and
  module-runtime hostile Skill Tool suites. These statuses are reported only for
  the platforms that actually ran them.

## Rollout

Keep the module feature disabled by default. Roll out declarative tools first,
then enable the module feature only in reviewed native builds. Operators validate,
trust, and enable each immutable revision separately. Global and per-Skill kill
switches atomically withdraw executable entries while retaining trust, validation,
diagnostic, and audit evidence.

## Rollback

Disabling `skill-tool-module-runtime` removes module execution at compile time while preserving manifests, validation state, trust records, diagnostics, and declarative-only operation. No stored Skill data migration is required.

For an operational incident, disable the affected Skill first, then use the global
switch if scope is uncertain. Revoke the exact revision when its integrity or
provenance is suspect. Recovery requires clean rediscovery and validation, an
explicit new trust decision where necessary, and a separate enable action.
