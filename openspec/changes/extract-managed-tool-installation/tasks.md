## 1. Baseline

- [ ] 1.1 Record the pre-change pass state of `cargo test --workspace cli_environment`, `cargo test --workspace vendor`, and `cargo test --workspace tooling`. These counts are the acceptance evidence for the move, so they have to exist before anything moves
- [ ] 1.2 Record the current physical line counts of `domain/trust.rs`, `infrastructure/vendor_downloader.rs`, `infrastructure/vendor_source.rs`, and the `tooling` subtree budget headroom, so the move can be shown to move code rather than duplicate it

## 2. The subdomain

- [ ] 2.1 Create `contexts/tooling/managed_install/{mod,api}.rs` with `domain` and `infrastructure`, registered in `tooling/mod.rs`
- [ ] 2.2 Add `ManagedInstallError` with the variants the retrieval actually produces, and a `From<ManagedInstallError> for CliEnvironmentError` in the CLI adapter — the conversion belongs on the CLI side, so the shared error never learns CLI vocabulary

## 3. Move the retrieval policy

- [ ] 3.1 Move `allowed_hosts`, `max_download_bytes`, `download_timeout_seconds`, and `permits_url` into `managed_install::domain` as `RetrievalPolicy`. **The body of `permits_url` does not change** — a behavior-preserving move means the diff shows relocation, not edits
- [ ] 3.2 Move `CliInstallerIntegrity` as `ArtifactIntegrity`
- [ ] 3.3 Leave `CliInstallerTemplate`, `CliInstallerRuntime`, `CliInstallerVersionArgument`, `CliPlatform`, and `template_for` in `tooling/cli`. Their no-fallback test stays with them
- [ ] 3.4 Move the `permits_url` tests verbatim. If a moved test needs editing to pass, the move was not behavior-preserving — find out why before editing it
- [ ] 3.5 Add a catalog-walking test that every CLI vendor source declares a non-empty allowlist and a non-zero ceiling, satisfying the "refused at declaration" requirement without making startup fallible over a constant

## 4. Move the retrieval

- [ ] 4.1 Move `vendor_downloader.rs` into `managed_install::infrastructure` as the artifact retriever, generalising `CliInstallerDownloader` to a `ManagedArtifactRetriever` port and `DownloadedInstaller` to `RetrievedArtifact`
- [ ] 4.2 Move `vendor_downloader_tests.rs` with it, unchanged except for names
- [ ] 4.3 Rewire `vendor_source.rs` to the shared port, converting the error at the boundary
- [ ] 4.4 Move the concrete construction in `bootstrap/` to the shared type
- [ ] 4.5 Delete the originals. A move that leaves the old file behind is a copy, and the subtree budget will say so
- [ ] 4.6 **Acceptance for the move, before any new behavior:** the suites from task 1.1 pass with the same counts. Do not start group 5 until this holds

## 5. The archive kind

- [ ] 5.1 Add archive retrieval: download and verify through the shared path, then extract into an owned directory
- [ ] 5.2 Enforce path containment on each entry's **resolved** path, not on its name. `a/../../b` passes a leading-slash check
- [ ] 5.3 Enforce a total-bytes ceiling and an entry-count ceiling while extracting, removing the destination if either trips
- [ ] 5.4 Discard the downloaded archive after successful extraction
- [ ] 5.5 Add tests for: an entry escaping the destination, an absolute-path entry, a parent-component entry, exceeding the byte ceiling, exceeding the entry count, and a clean extraction reporting its directory
- [ ] 5.6 Confirm the archive kind has no production caller in this change and is not wired into any command surface

## 6. Platform selection

- [ ] 6.1 Add the shared capability's own platform-selection type and exact-match rule, independent of `CliPlatform`
- [ ] 6.2 Add a test that no artifact is selected when none is declared for the current platform, and that nothing is substituted

## 7. Documentation

- [ ] 7.1 Update the developer guide's CLI environment section to say where the download path now lives and why only that half moved
- [ ] 7.2 Run `npm run docs:check`

## 8. Verification

- [ ] 8.1 `npm run lint:ci`
- [ ] 8.2 `npm run test`
- [ ] 8.3 `npm run build`
- [ ] 8.4 `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`
- [ ] 8.5 `cargo check --workspace`
- [ ] 8.6 `cargo clippy --workspace --all-targets -- -D warnings`
- [ ] 8.7 `npm run native:panic:check`
- [ ] 8.8 `cargo test --workspace`
- [ ] 8.9 `npm run architecture:check`, `npm run contracts:check`, `npm run coverage:policy:test`, `npm run version:unit:test`
- [ ] 8.10 `npx playwright test`
- [ ] 8.11 `npm run desktop:unit:test`, then `npm run test:desktop`. The `desktop-cli-management` layer is the one that exercises this path end to end, against a temporary PATH with fake package managers — it is the closest thing to evidence that the move did not break a real install
- [ ] 8.12 `openspec validate extract-managed-tool-installation --strict` and `openspec validate --specs --strict`
- [ ] 8.13 Simulate the archive merge with `buildUpdatedSpec`

## 9. Acceptance

- [ ] 9.1 Confirm the task 4.6 checkpoint held: the move alone changed no test outcome
- [ ] 9.2 Confirm the move moved rather than copied — the `tooling` subtree total should fall or hold, not rise by the size of the moved code
- [ ] 9.3 Confirm no schema change and no migration were added
- [ ] 9.4 Confirm no command, DTO, or frontend file changed
- [ ] 9.5 Confirm the shared error type carries no CLI-specific variant, and the shared domain names no CLI type
