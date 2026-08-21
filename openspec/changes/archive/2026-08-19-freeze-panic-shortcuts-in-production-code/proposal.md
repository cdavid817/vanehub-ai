## Why

AGENTS.md says `unwrap()` and `expect()` are permitted only in test code. Nothing enforces it. Clippy's `unwrap_used` and `expect_used` lints are `allow` by default, so `cargo clippy --all-targets -- -D warnings` — which CI runs and which does catch a great deal — is silent on both.

The optimization ticket proposed closing this by adding the lints to `src-tauri/Cargo.toml` and whitelisting existing violations with module-level `#![allow(...)]`, estimating roughly 3,700 non-test sites.

**Measurement says that plan cannot work, and the estimate is off by two orders of magnitude in the direction that matters.**

| Clippy target selection | `unwrap_used` + `expect_used` violations |
|---|---:|
| `--lib` (production only) | **35** |
| `--bins` | **35** |
| `--all-targets` (adds test targets) | **9,595** |

Production code holds 35 violations across 11 files. The other ~9,560 are in test code, where the project deliberately allows them.

That gap is what breaks the ticket's plan. A `[lints.clippy]` entry in `Cargo.toml` applies to every target, and CI already runs `--all-targets -- -D warnings`, so setting these lints to anything above `allow` turns 9,560 test-code sites into hard errors. The whitelist would have to cover several hundred test modules — and every new test module forever after.

## What Changes

- Add a CI gate that runs clippy against **non-test targets only**, denying both lints there:

  ```
  cargo clippy --manifest-path src-tauri/Cargo.toml --lib --bins -- -D clippy::unwrap_used -D clippy::expect_used
  ```

- **Do not touch `src-tauri/Cargo.toml`.** No `[lints]` section is added, so `--all-targets -- -D warnings` keeps its current meaning and no test module needs an exemption — now or when the next one is written.
- Whitelist the 35 existing production violations with a file-level `#![allow(...)]` in each of the 11 files, each carrying the reason and the change that will retire it.
- Expose the gate as an npm script so it is reachable the same way as the other quality commands.

## The whitelist is now a finishable list

The ticket's follow-up item — retire the whitelist — was scoped against ~3,700 sites, which is why it reads as a permanent background chore. The real list is 35 sites in 11 files:

| File | Sites |
|---|---:|
| `contexts/tooling/skills/application/service.rs` | 12 |
| `contexts/retrieval/domain/code_redaction.rs` | 6 |
| `contexts/permissions/application/approval_broker.rs` | 6 |
| `contexts/task_orchestration/domain/graph.rs` | 3 |
| `contexts/permissions/infrastructure/hook_bridge_wait_registry.rs` | 2 |
| six files with one each | 6 |

Two of those files are `domain` layer, where a panic shortcut is least defensible.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `repository-governance`: the existing "Existing source constraints remain enforced" requirement names "production Rust uses a prohibited panic shortcut" among the things the configured checks reject, which today they do not. This change makes that requirement true and records that the enforcement covers non-test targets only.

## Impact

- `.github/workflows/ci.yml` — one new step in the `Rust` job.
- `package.json` — one new script.
- Eleven Rust source files — a file-level `#![allow(...)]` with a reason comment. No logic changes.
- `src-tauri/Cargo.toml` — deliberately unchanged, which also keeps this change free of conflicts with the in-flight dependency cleanup.
- No runtime behavior changes in either the desktop or Web runtime.
