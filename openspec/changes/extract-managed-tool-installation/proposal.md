## Why

`cli-environment-management` contains the one place VaneHub downloads a program and then runs it. That path carries constraints that took real effort to get right: HTTPS-only URLs matched against an exact host allowlist, redirects followed manually so the allowlist applies to every hop the vendor's CDN chose, a byte ceiling enforced while reading rather than after, a deadline and a cancellation check between hops and while streaming, SHA-256 verification before anything executes, a temporary directory removed on success, failure, timeout and cancellation alike, and installer templates selected by exact platform match with **no fallback arm** — the missing fallback being the fix for a defect that once produced a `bash -lc` plan on Windows and then silently substituted npm while telling the user a vendor install had happened.

Java's `jdtls` is distributed as an archive rather than found on `PATH`, so managing its installation means downloading and executing on the user's machine. There are exactly two ways to reach that: share the audited path, or write a second one.

Writing a second one is the option this change exists to prevent. A duplicated allowlist drifts. A duplicated redirect loop gets the "check each hop" part wrong. A duplicated ceiling gets checked after the write. None of those failures is visible in review of the copy, because the copy looks correct on its own.

## What Changes

This is deliberately **not** the full extraction an earlier draft of this proposal described. Measured, `contexts/tooling/cli` is 24,372 lines across 64 files, 38 of which name CLI-shaped types, behind a 21-requirement spec, with `agent_id` as a persisted primary key in three tables. Moving discovery, active resolution, conflicts, version catalogs, action planning, and mutation coordination into a shared capability — and renaming that key — is a large behavior-preserving refactor whose own proposal admitted it adds no user-visible capability. The risk is real and the payoff is entirely in the next change.

What actually must not be duplicated is the verified-download-and-execute core. That is what moves.

- Extract into a shared capability: URL admission (HTTPS plus exact-host allowlist, applied per redirect hop), bounded and deadlined streaming download with cancellation, SHA-256 verification before execution, owned temporary storage, and exact-platform template selection without fallback.
- Add an **archive** artifact kind: download, verify, then extract into a managed directory, rather than download, verify, then execute. `jdtls` needs it, and its verification and path-traversal handling belong beside the download bounds rather than in a second place.
- Leave everything else in `tooling/cli`: the CLI tool catalog, discovery, active resolution, conflicts, version catalogs, action plans, mutation coordination, persistence, diagnostics, and the CLI management UI. No table changes, no `agent_id` rename, no migration.
- Restate `cli-environment-management`'s download-trust requirements as delegating to the shared capability. Externally observable CLI behavior does not change, and the acceptance evidence for that is the existing CLI environment suite passing unchanged.

## Capabilities

### New Capabilities

- `managed-tool-installation`: the audited path for acquiring an executable artifact VaneHub will run — host-allowlisted HTTPS with per-hop redirect admission, bounded and deadlined download with cancellation, digest verification before use, owned temporary storage released on every exit, exact-platform artifact selection with no fallback, and archive extraction bounded against path traversal and size.

### Modified Capabilities

- `cli-environment-management`: its installer-download and integrity requirements are restated as delegating to `managed-tool-installation`. The CLI catalog, planning, execution ordering, conflicts, and verification requirements are untouched and stay owned here.

## Impact

**Runtimes affected: desktop only.** Nothing in this change reaches a frontend surface. The Web/mock adapter is untouched because no new command is exposed; installation entry points arrive with the change that consumes this one.

Affected code:

- `src-tauri/src/contexts/tooling/cli/domain/trust.rs` and `infrastructure/vendor_downloader.rs` — the code that moves
- `src-tauri/src/contexts/tooling/managed_install/` — new subdomain with its own `api.rs`
- `src-tauri/src/contexts/tooling/cli/infrastructure/vendor_source.rs` — rewired to the shared port
- `src-tauri/src/bootstrap/` — the concrete downloader is constructed here, as it is today

Known hazards this change must handle rather than discover late:

- The moved code is security-sensitive and its tests are the evidence it still works. Move the tests with it rather than rewriting them; a rewritten test proves the new code does what the new code does.
- `CliInstallerTrust` currently carries both policy (hosts, ceiling, timeout) and CLI-specific template shape. Only the policy half is tool-agnostic. Splitting it wrongly would either leave the allowlist behind or drag `CliPlatform` into the shared capability.
- The archive kind has no consumer in this change. Designing it here is deliberate — `add-lsp-java-jdtls` would otherwise have to reopen the capability — but it must ship with its own tests rather than as an untested affordance.

Dependency: none outstanding. `extend-lsp-language-registry` and `expand-lsp-read-only-methods` have both landed.
