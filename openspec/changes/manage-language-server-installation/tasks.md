## 1. Baseline

- [ ] 1.1 Record the pre-change pass state of `cargo test --workspace code_intelligence`, `cargo test --workspace managed_install`, and the frontend LSP and settings vitest files
- [ ] 1.2 List the `expect(dead_code)` attributes currently marking the archive half as caller-less. Every one of them has to come off in this change; any that survives means the caller was not wired where it was supposed to be

## 2. The archive guard grows one rule

- [ ] 2.1 Refuse an entry that is not a regular file or a directory. A link's containment cannot be decided when it is written — it resolves at use, and one pointing inside the destination today points outside it after something else moves
- [ ] 2.2 Add tests for a symlink entry and a hard-link entry, both refused regardless of where they point
- [ ] 2.3 Confirm the existing zip tests pass unchanged

## 3. The tar.gz adapter

- [ ] 3.1 Add `tar` to `Cargo.toml`. `flate2` is already present for the gzip layer
- [ ] 3.2 Add `extract_tar_gz` beside `extract_zip`, feeding the same `ExtractionGuard`. **Neither adapter may reach the destination path except through `admit`**
- [ ] 3.3 Add the same six tests the zip adapter has — escaping entry, absolute entry, byte ceiling, entry count, clean extraction, unbounded limits — against a tar.gz fixture built in memory
- [ ] 3.4 Add a test that both formats refuse the same escaping entry name, so the shared checks are shown to be shared rather than assumed to be

## 4. The distribution declaration

- [ ] 4.1 Add an optional published distribution to the registry entry: allowlisted host, URL, integrity, archive format, and extraction limits, using `managed_install`'s own types
- [ ] 4.2 Declare Java's: `download.eclipse.org`, the latest tar.gz, `Unverified`, with a byte ceiling and extraction limits sized for a real `jdtls`
- [ ] 4.3 Add a registry test that a declared distribution's retrieval policy `is_bounded()`, the same check the CLI catalog applies to its own
- [ ] 4.4 Confirm the five languages that declare none are unaffected: no install action, discovery unchanged

## 5. Install and uninstall

- [ ] 5.1 Add the install action: retrieve, extract, then rename into `<app data>/lsp/<language>/install`. Extract under the destination's parent, not the system temp, so the rename cannot cross filesystems
- [ ] 5.2 Add the uninstall action: stop the language's processes first — on Windows a directory a process holds open simply will not delete — then remove **only** the managed directory
- [ ] 5.3 Make discovery prefer the managed install when no override is set, with the override always winning
- [ ] 5.4 Add tests: an interrupted install leaves nothing, uninstall leaves an override's directory untouched, and an override wins over a managed install that also exists
- [ ] 5.5 Remove every `expect(dead_code)` from the archive half. **If one still compiles, its caller is missing**

## 6. Commands and the settings surface

- [ ] 6.1 Add the install and uninstall commands, with the install reporting its outcome rather than blocking the UI thread
- [ ] 6.2 Extend the descriptor with the distribution's presence and whether its bytes are verified
- [ ] 6.3 Render install and uninstall on the card, from the descriptor rather than from the language id
- [ ] 6.4 State plainly, before the user clicks, that the download is not checksum-verified
- [ ] 6.5 Add the Web/mock unavailable results. The Web adapter must not simulate a download
- [ ] 6.6 Add locale strings in all five bundles and extend `lsp-settings-localization.test.ts`
- [ ] 6.7 Add a component test driven by a descriptor with a distribution, using a language that is not Java

## 7. Documentation

- [ ] 7.1 User guides: replace the manual Java install steps with the button, and say what "not checksum-verified" means for someone deciding whether to click
- [ ] 7.2 Developer guides: the distribution declaration, the one-guard-two-adapters rule, and the discovery precedence
- [ ] 7.3 Correct the exclusion list's "downloaded servers", which this change makes untrue
- [ ] 7.4 Run `npm run docs:check`

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
- [ ] 8.11 `npm run desktop:unit:test`, then `npm run test:desktop`
- [ ] 8.12 `openspec validate manage-language-server-installation --strict` and `openspec validate --specs --strict`
- [ ] 8.13 Simulate the archive merge with `buildUpdatedSpec`

## 9. Acceptance

- [ ] 9.1 Confirm no `expect(dead_code)` remains in `managed_install`. The attributes were the marker; their absence is the evidence the capability is wired
- [ ] 9.2 Confirm both archive adapters go through one guard: `grep` finds one containment check, not two
- [ ] 9.3 Confirm no frontend file names Java, and the install action follows a descriptor
- [ ] 9.4 Confirm no database migration was added
- [ ] 9.5 Confirm the unverified-bytes statement reaches the user before the click, not only the change record
- [ ] 9.6 Confirm no real download happens in any test. A test that reaches `download.eclipse.org` is a test that fails on an air-gapped runner and passes for the wrong reason everywhere else
