# Verification Record

Date: 2026-08-20
Branch: `release/first-stable-version`
Target version: `1.0.0`

## Repository gates

| Gate | Result | Evidence |
| --- | --- | --- |
| Version synchronization | PASSED | All npm, Cargo, Tauri, and lockfile metadata report `1.0.0`; explicit `v1.0.0` validation passed. |
| OpenSpec change validation | PASSED | `openspec validate prepare-first-stable-release --strict` |
| OpenSpec main specifications | PASSED | 138 specifications passed strict validation. |
| ESLint | PASSED | `npm run lint:ci` |
| Frontend unit/component tests | PASSED | 287 files and 1309 tests passed. |
| Frontend coverage | PASSED | 287 files and 1309 tests passed with the configured coverage policy. |
| Frontend build | PASSED | TypeScript, Vite, and the lazy-chunk budget check passed. |
| Contract checks | PASSED | Three contract-conformance tests passed. |
| Release policy tests | PASSED | Stable/preview notes, fail-closed signing gates, and five-target matrix checks passed. |
| Rust formatting | PASSED | `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check` |
| Rust clippy | PASSED | All targets passed with warnings denied. |
| Rust tests | PASSED | 3547 library tests passed, 15 fixture tests were ignored, and architecture/MCP integration tests passed. |
| Rust check | PASSED | `cargo check --manifest-path src-tauri/Cargo.toml` |
| Web Playwright | PASSED | 156 of 156 Chromium tests passed. |

## Native platform status

| Platform | Status | Evidence |
| --- | --- | --- |
| Windows | BLOCKED | The test client built and launched; 12 of 13 desktop spec files passed. Live Claude Code returned HTTP 403 and Gemini CLI requested interactive reauthentication, so the session spec failed 2 external-authentication cases. Evidence: `test-results/desktop/2026-08-20T03-10-09-410Z-303b1332`. |
| macOS x64 | NOT RUN | Native results cannot be inferred from Windows. GitHub rehearsal pending. |
| macOS ARM64 | NOT RUN | Native results cannot be inferred from Windows. GitHub rehearsal pending. |
| Linux x64 | NOT RUN | Native results cannot be inferred from Windows. GitHub rehearsal pending. |
| Linux ARM64 | NOT RUN | Native results cannot be inferred from Windows. GitHub rehearsal pending on `ubuntu-24.04-arm`. |

## GitHub release readiness

| Gate | Status | Evidence |
| --- | --- | --- |
| Protected `release` environment | PASSED | Environment exists and accepts only the `v*` tag policy. |
| Required environment secret names | BLOCKED | GitHub API reports 0 of 11 required names. No secret values were requested or exposed. |
| Non-publishing package rehearsal | FAILED | Run `32328440115` exposed missing `xdg-utils`; run `32329743078` then produced Linux x64/ARM64 `.deb` and AppImage bundles but exposed that current Tauri bundling ignores a path-only signing variable. Corrective rerun pending. |
| Annotated `v1.0.0` tag | NOT RUN | Prohibited until rehearsal, credentials, merge, and explicit maintainer approval are complete. |

The current source state is not eligible for a stable tag while any row above is `BLOCKED`, `FAILED`, or `NOT RUN`.
