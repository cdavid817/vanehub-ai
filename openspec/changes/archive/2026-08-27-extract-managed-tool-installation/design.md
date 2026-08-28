## Context

Measured before deciding scope:

- `contexts/tooling/cli` is 24,372 physical lines across 64 files, 16,942 of them production. 38 of those files name `CliToolId`, `CliToolDefinition`, or `CliSourceKind`.
- `cli-environment-management` has 21 requirements.
- `agent_id` is a persisted primary key in `cli_installations`, `cli_version_catalogs`, and `cli_action_plans`.

The code that actually must not be duplicated is much smaller:

- `domain/trust.rs` (286 lines) — `CliInstallerTrust` (allowlist, ceiling, timeout), `permits_url`, `CliInstallerTemplate`, `CliInstallerIntegrity`, `CliInstallerRuntime`, `template_for` with no fallback arm.
- `infrastructure/vendor_downloader.rs` (241 lines) — the manual redirect loop, per-hop admission, streaming ceiling, deadline and cancellation checks, SHA-256 accumulation, verification, and the owned `TempDir`.
- The `CliInstallerDownloader` port and `DownloadedInstaller` in `infrastructure/vendor_source.rs`.

Roughly 550 lines, of which the tool-specific part is the *template* shape — runtime, version argument, `CliPlatform` — not the policy.

## Goals / Non-Goals

Goals:

- One implementation of allowlisted, bounded, deadlined, digest-verified retrieval, reachable from any context that needs to fetch something it will then run.
- An archive kind designed alongside it, so the next change adds a consumer rather than reopening the capability.
- CLI environment behavior byte-identical afterwards, evidenced by its existing suite rather than by new tests.

Non-Goals:

- Extracting discovery, active resolution, conflicts, version catalogs, action planning, mutation coordination, or the CLI management UI. Those stay in `tooling/cli`.
- Renaming `agent_id`. No schema change, no migration.
- Any new command, DTO, or frontend surface. This change is invisible from outside the native process.
- A managed-install *orchestrator*. Deciding what to install, when, and in what order stays with each consumer.

## Decisions

### 1. A new `tooling` subdomain, not a `platform` module

`platform/` holds mechanism with no policy — `platform::network::blocking_no_redirect_http_client` is already there, and this capability will use it. But an allowlist and a digest are policy, and policy with its own requirements belongs in a context. `contexts/tooling/managed_install/{domain,infrastructure,api}` is a sibling of `cli` under the existing `tooling` context, which already houses several subdomains each with their own `api.rs`.

Consumers reach it through `managed_install::api`. `tooling/cli` is in the same context so this is an ordinary intra-context dependency; a later consumer in another context reaches it through `contexts::tooling::api`, which re-exports on demand.

### 2. The trust policy splits along "who declares it"

`CliInstallerTrust` carries two different things. `allowed_hosts`, `max_download_bytes`, and `download_timeout_seconds` are what the shared capability enforces. `templates` — with `CliPlatform`, `CliInstallerRuntime`, and `CliInstallerVersionArgument` — is how the CLI context describes *its* installers.

The split follows that line:

- **Moves:** `RetrievalPolicy { allowed_hosts, max_bytes, timeout }`, `permits_url`, `ArtifactIntegrity::{Unverified, Sha256}`, and the retrieval itself.
- **Stays:** `CliInstallerTemplate`, `CliInstallerRuntime`, `CliInstallerVersionArgument`, `CliPlatform`, and `template_for`. The no-fallback rule is about *CLI installer templates* specifically, so its test stays with it.

The shared capability gets its own platform-selection requirement anyway, because the archive kind needs one and dragging `CliPlatform` across would make the shared type describe CLI's platform enum rather than its own.

Rejected: moving `CliInstallerTrust` wholesale and having the shared capability know about installer runtimes. That would make "shared" mean "CLI's, plus whatever else shows up", which is how the drift this change prevents gets reintroduced from the other direction.

### 3. `RetrievalPolicy` is validated at construction, not at download

The spec requires a declaration without an allowlist or a ceiling to be refused at declaration. In Rust that means a constructor returning `Result` rather than a public struct literal, and the CLI catalog's `const` declarations becoming checked ones.

The CLI catalog is `static` data today, so a fallible constructor cannot run in a `const`. The resolution: keep the CLI-side declarations as data, and validate them once in a test that walks the whole catalog — the same shape `registry.rs` already uses for language definitions, where `trusted()` accepts a `&'static str` under a `debug_assert` and a test proves every entry is well-formed. That keeps the guarantee without making startup fallible over a constant the build already fixed.

### 4. The error type does not move

`CliEnvironmentError` is the CLI context's error. The shared capability gets its own `ManagedInstallError`, and the CLI adapter converts. This is a few lines of `From`, and it is what stops the shared capability from growing CLI-shaped variants the first time something new needs an error.

### 5. Archive extraction is bounded the way download is

Same posture, different axis: a destination directory the capability owns, every entry's resolved path checked to be inside it before anything is written, a total-bytes ceiling and an entry-count ceiling enforced while extracting, and the whole destination removed if either trips. Path checking is done on the resolved path rather than on the entry name, because `a/../../b` is not caught by scanning for a leading `/`.

The archive is discarded after extraction. Keeping it would double the disk cost of every managed install for a file nothing reads again.

## Risks / Trade-offs

- **The moved code is the security-sensitive part** → move its tests with it rather than rewriting them. `vendor_downloader_tests.rs` is 241 lines of exactly the assertions that matter; a rewrite proves the new code does what the new code does.
- **The archive kind ships without a consumer** → it is specified and tested here so the next change adds a caller rather than a capability, but an untested affordance would be worse than no affordance. Its tests are part of this change's acceptance, not the next one's.
- **`permits_url` has subtle cases** (userinfo, port suffix, case) → its existing tests move verbatim. The function is small enough that the temptation to "clean it up" during the move is real and should be resisted; a behavior-preserving move means the body is unchanged.
- **Two error types where there was one** → a small amount of conversion boilerplate, accepted to keep the shared capability from accumulating CLI vocabulary.

## Migration Plan

No data migration. The move is internal to the native process and no persisted shape or wire DTO changes.

Order: create the subdomain with the moved code and its moved tests, rewire `vendor_source.rs` to the shared port, delete the originals, then add the archive kind. The CLI environment suite is run after the rewire and before the archive work, so a failure there is attributable to the move rather than to the addition.

## Open Questions

- Whether `code_intelligence` should reach this through `contexts::tooling::api` or get its own port in its application layer bound in bootstrap. The architecture rules allow the first and prefer the second for testability; nothing in this change decides it, because this change has no `code_intelligence` consumer. The next one does and should settle it there.
