## Why

Java works now, and installing it is still a manual chore: download an Eclipse archive, extract it, find the right directory level, paste the path. Every other supported language is a one-line install with a package manager the user already has. `jdtls` is the one that is not, which is why it is the one that needs managing.

The pieces exist. `managed-tool-installation` owns the audited download path and a bounded archive extractor, both shipped tested and both with no caller. `add-lsp-java-jdtls` left one seam: where the install directory comes from. This change fills it.

## What Changes

- Let a registered language declare where its server is published: allowlisted host, URL, integrity, and extraction limits, using the shared capability's own types rather than a second set.
- Add a `tar.gz` adapter beside the existing zip one, since that is what Eclipse publishes. Both feed the same `ExtractionGuard`, so the containment and the limits are not duplicated — only the format walk is.
- Add install and uninstall actions for a declared language, writing into a directory VaneHub owns under the profile, and remove the `expect(dead_code)` attributes that have been marking the shared capability's archive half as caller-less.
- Make discovery prefer a managed install when one exists and no manual override is set. A manual override still wins, so a user who has their own `jdtls` keeps it.
- Report install state on the language card: not installed, installing, installed with the version the directory holds, or failed with a reason the shared retrieval produced.
- Keep the Web/mock runtime honest: it reports the same states and never simulates a download.

## Capabilities

### New Capabilities

None. This is the consumer the previous two changes were built for.

### Modified Capabilities

- `lsp-server-management`: a registered language MAY declare a published distribution; discovery resolves a managed install when there is no manual override; the install directory's lifetime and removal are defined.
- `managed-tool-installation`: the archive kind gains a second format adapter, and both go through the one guard.
- `settings-center-ui`: the language card gains install and uninstall actions and their states for the languages that declare a distribution.

## Impact

**Runtimes affected: desktop and Web.** Installation is inherently desktop; the Web adapter reports the actions as unavailable rather than pretending.

Affected code:

- `src-tauri/src/contexts/tooling/managed_install/infrastructure/extraction.rs` — the `tar.gz` adapter
- `src-tauri/src/contexts/code_intelligence/{domain/registry.rs,infrastructure/server_discovery.rs,api.rs}`
- `src-tauri/src/commands/code_intelligence/` — the install and uninstall commands
- `src/settings/pages/agents/` and the five locale bundles
- `src-tauri/Cargo.toml` — one new dependency

Known hazards this change must handle rather than discover late:

- **The bytes are not verified, and the UI has to say so.** Eclipse publishes `jdt-language-server-latest.tar.gz` with no digest that is stable across releases, so the artifact is declared `Unverified`: allowlisted host, HTTPS, byte ceiling, deadline, cancellation — but no checksum. That is the same posture every shipped CLI vendor installer already has, and it is a posture a user should be told about rather than one that hides behind an install button.
- **A partial install must not look installed.** Extraction already removes its destination on any failure; the install action has to place the finished directory atomically rather than extracting into the final location.
- **Uninstall must not delete a directory the user pointed at.** A manual override names a directory VaneHub did not create; uninstall removes only the managed one.
- **Adding a dependency is a supply-chain decision.** `tar` is the choice, it is widely used and pure Rust, and CI's Dependency Review is what vets it. Recorded here so it is a decision rather than an import.

Dependencies: `extract-managed-tool-installation` for the download and extraction, and `add-lsp-java-jdtls` for the launch shape and the directory-shaped override. Both landed.
