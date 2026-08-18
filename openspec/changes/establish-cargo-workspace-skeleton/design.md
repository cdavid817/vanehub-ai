## Context

See proposal.md — Why for the coupling measurements and for the two ticket steps they invalidate.

## Goals / Non-Goals

**Goals:**

- Get a working workspace with the packaging chain intact, proved before any context depends on it.
- Remove the `default-run` workaround, which exists only because two unrelated binaries share one package.
- Leave a sequenced, measured plan for the extractions this change does not perform.

**Non-Goals:**

- Extracting any bounded context. The prerequisites differ per context and the first one is not ready.
- Extracting `vanehub-platform`. It is not a base layer yet; see below.
- Reducing compile time in this change. The skeleton alone changes nothing there — one member compiles like one package. The gain arrives with the first real split, and this change records a baseline to measure it against.

## Decisions

### Ship the skeleton with one member and prove the packaging chain first

A workspace with a single member is a no-op for the compiler and a real test of everything around it: `npm run tauri -- dev`, `npm run package`, the sidecar preparation script, the Tauri bundler's expectations about target directories, and the three Desktop Smoke platforms.

Those are where a workspace migration actually breaks, and finding out with one member is cheap. Finding out three crates later is not.

### Extract `vanehub-permission-hook` now, because it is the one piece with no prerequisites

It is a separate binary that shares a package purely by history, and that sharing is what forces `default-run = "vanehub-ai"` at `src-tauri/Cargo.toml:11`. Moving it to its own crate removes the workaround rather than carrying it into the workspace.

Its blast radius is the sidecar chain — `scripts/prepare-permission-hook-sidecar.mjs`, the Tauri sidecar config, and `npm run sidecar:prepare` — which is exactly the packaging surface this change wants to exercise anyway.

### The migration inversion is a prerequisite, and it belongs to its own change

`platform/database/migrations/mod.rs` calls `crate::contexts::<name>::infrastructure::apply_schema` for 51 of the 79 migrations. As long as that holds, `platform` cannot be a crate that contexts depend on, because it depends on them.

Inverting it means contexts register their schema with the migration runner. That is a real design change: it touches ordering guarantees, the `EXPECTED_MIGRATIONS` registry that catches version collisions, and a spec requirement about transactional application with startup density verification. Bundling it into a workspace-skeleton change would put the one guard that catches migration collisions into the same diff as a build-graph reorganisation.

### Pilot order, once the skeleton is proved

| Order | Crate | Why |
|---|---|---|
| 1 | `vanehub-permission-hook` | No prerequisites — in this change |
| 2 | `work_board` or `goals` | Zero inbound *and* zero outbound; 765 and 1,363 lines. A pilot should prove the pattern, not survive it |
| 3 | `retrieval` | Zero coupling but 11,500 lines — proves the pattern holds at scale |
| 4 | `operations` | Zero outbound, 51 inbound files. Highest value, largest mechanical edit; do it once the pattern is boring |
| 5 | migration inversion, then `vanehub-platform` | The prerequisite and the base layer |
| — | `agent_runtime`, `tooling`, `cli_delegation` | 10, 6 and 6 outbound contexts. Not extractable without work that is not a move |

The ticket's `web_research` pilot is dropped: it reaches into `agent_runtime`.

### `[workspace.dependencies]` now, `[workspace.lints]` deliberately empty

Hoisting all 87 version declarations is the point of the workspace for dependency management, and it is mechanical.

`[workspace.lints]` is added as a section but left carrying nothing new. The panic-shortcut gate introduced by `freeze-panic-shortcuts-in-production-code` is deliberately *not* a `Cargo.toml` lint — it is a target-scoped clippy invocation, because `[lints]` has no target selectivity and would fail on ~9,560 test sites. That reasoning does not change under a workspace, and `[workspace.lints]` inherits the same limitation.

## What the skeleton surfaced

The single-member skeleton exists to find the things a workspace breaks before anything depends on
it. It found four, and the first two are the ones that would not have announced themselves.

### `[profile.release]` is silently ignored in a non-root member

Cargo only warns:

```
warning: profiles for the non root package will be ignored
```

So `opt-level = 3`, `lto = "thin"`, `codegen-units = 1` and `strip = "debuginfo"` all stop applying.
The architecture test `distributable_release_profile_stays_optimized` would have kept passing
throughout, because it reads the manifest text rather than the profile Cargo resolves. A green gate
over a lost property is worse than no gate. The profile moves to the workspace root.

### The sidecar script's fallback path becomes wrong

`scripts/prepare-permission-hook-sidecar.mjs` fell back to `<root>/src-tauri/target`. Under a
workspace, Cargo writes to `<root>/target`. The failure mode is indirect: the build succeeds, then
the staging copy reports a missing source, which reads like a build problem rather than a path
problem.

### Adopting a workspace changes dependency resolution, by ten build-only packages

The lockfile goes from 785 resolved packages to 795. This was isolated rather than assumed:

- Not caused by hoisting — a bare workspace with **zero** hoisted dependencies also resolves 795.
- Not the resolver — `resolver = "2"` and `"3"` both give 795.
- Not a stale lockfile — re-resolving the pristine single-package manifest still gives 785.

The additions are the `jiff` and `defmt` families plus two others, pulled in through
`tauri-build` → `tauri-utils` → `serde_with` feature unification. `cargo tree --edges normal`
reports "nothing to print" for `jiff`, confirming they are reachable only as build dependencies:
they cost build time, not binary size.

### The lockfile moves, and a CI cache key follows it

`src-tauri/Cargo.lock` ceases to exist; the lockfile lives at the workspace root. The Documentation
job keyed its cache on `hashFiles('src-tauri/Cargo.lock')`, and `hashFiles()` on a missing path
returns a constant — so this would have degraded into a cache that never invalidates rather than
failing.

### `.github/workflows/package.yml` located artifacts by a path that no longer exists

Nine references to `src-tauri/target/${{ matrix.rust_target }}/release/bundle` — in the Windows
signing step, the Windows Authenticode verification step, the macOS notarization verification
step, and the artifact upload step. All nine assumed the pre-workspace target directory.

This was not caught by any of the verification run so far, because none of it exercises the
release workflow: `npm run package` was run locally without a `--target`, landing at
`target/release/bundle/` directly, and CI's `ci.yml` never builds a distributable. It surfaced only
because verifying the local packaging output prompted a direct check for other consumers of the old
path. The upload step has `if-no-files-found: error`, so this would not have failed silently, but
it would have failed only when a release was actually cut — the most expensive point to discover a
path bug at, and on a workflow this repository's CI does not otherwise exercise.

Fixed to `target/${{ matrix.rust_target }}/release/bundle` across all nine sites, with one comment
at the first use rather than nine repeated ones.

## Risks / Trade-offs

- **The Tauri bundler or the sidecar script depends on the current target-directory layout** → This is the specific risk the single-member skeleton exists to surface. Desktop Smoke runs on all three platforms and is the check; a local pass on one OS is not evidence for the others.
- **`cargo test --workspace` changes which tests run** → Compare the test count against the current baseline; it must not fall.
- **Path budgets in `architecture.rs` are repository-root-relative and might double-count a moved crate** → Verified explicitly rather than assumed, since the budgets are the ratchet everything else relies on.
- **A workspace makes it tempting to extract the next context immediately** → The pilot order above is measured, not intuited, and the two contexts the ticket suggested are not both suitable.
- **No compile-time win lands in this change** → Stated in Non-Goals. Recording the baseline now is what makes the later claim checkable, and the last attempt to claim a compile-time win in this work stream was measured and withdrawn.

## Migration Plan

Each step leaves the tree buildable and packageable. Rollback is `git revert` plus `cargo clean`; no data, schema, or runtime surface is touched.
