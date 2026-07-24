## 1. Documentation Build Foundation

- [x] 1.1 Add the developer-guide and English/Simplified-Chinese user-guide mdBook directory trees, `book.toml` files, `SUMMARY.md` files, shared asset directories, and ignored assembled-output directory.
- [x] 1.2 Add repository-owned documentation commands that verify the pinned mdBook prerequisite and build all three books into one deterministic output tree.
- [x] 1.3 Add Rustdoc generation with `--no-deps --document-private-items`, documentation warnings enabled, and assembly under the stable native API reference path.
- [x] 1.4 Add a documentation link/output validator covering Markdown chapter links, assets, language navigation, assembled entry points, and localized image alt text.
- [x] 1.5 Add npm script entry points for documentation checking, testing, building, screenshot generation, and screenshot verification without introducing an application runtime dependency.

## 2. Multilingual README Governance

- [x] 2.1 Add stable shared section identifiers to the English, Simplified Chinese, and Japanese README files.
- [x] 2.2 Implement a README parity checker for section order, command blocks, repository-relative links, version facts, and delivered/planned classifications, with focused regression tests.
- [x] 2.3 Refactor `README.md` into a concise canonical project entry point with accurate runtime/feature-state labels and links to the user and developer guides.
- [x] 2.4 Update `README.zh-CN.md` as a reviewed Simplified Chinese translation with parity to the canonical README.
- [x] 2.5 Update `README.ja.md` as a reviewed Japanese translation with parity to the canonical README.
- [x] 2.6 Verify the parity checker reports actionable file, section, and fact mismatches and never rewrites translated README files.

## 3. Native Developer Guide and API Reference

- [x] 3.1 Author developer-guide chapters for repository orientation, frontend service/adapters, desktop versus Web/mock behavior, and the native bounded-context map.
- [x] 3.2 Author or integrate chapters for SQLite ownership and migrations, unified logging and redaction, testing, packaging, release, contribution, and OpenSpec workflows without duplicating authoritative documents.
- [x] 3.3 Add developer documentation for Multi-Agent coordination plans, stable Agent ids, dependency scheduling, fallback policy, persistence, cancellation, and current UI availability.
- [x] 3.4 Define the selected native documentation-boundary inventory and add focused `//!`/`///` documentation to the crate entry, context APIs, domain contracts, application ports, and command boundary types it names.
- [x] 3.5 Add stable links between the mdBook developer guide and assembled Rustdoc root without widening native item visibility.
- [x] 3.6 Run mdBook navigation and code-sample tests and confirm the Rustdoc reference builds on the supported stable toolchain.

## 4. Localized User Guides and Workflow Media

- [x] 4.1 Author equivalent English and Simplified Chinese guide foundations for prerequisites, CLI installation/authentication, first project/session setup, runtime differences, results, and troubleshooting.
- [x] 4.2 Add consistent delivered, preview, Web/mock-only, desktop-only, and planned labels to every documented workflow.
- [x] 4.3 Author the representative Multi-Agent coding-task workflow with two independently ready implementation nodes, a dependent validation/review path, fallback behavior, cancellation, and final result review.
- [x] 4.4 Gate normal Multi-Agent UI instructions on a user-visible tested path; while the UI remains unavailable, publish only a clearly labeled preview/developer explanation and do not invent control screenshots.
- [x] 4.5 Add named Playwright documentation-capture scenarios using fixed fixtures, viewport, locale, visual style, reduced motion, fonts, and sanitized dynamic values.
- [x] 4.6 Generate and review English and Simplified Chinese screenshots for currently delivered user-guide steps, with localized alt text and no credentials, personal paths, native logs, or other sensitive data.
- [x] 4.7 Add screenshot inventory and freshness checks that distinguish deterministic Web/mock captures from explicitly reviewed desktop-only captures.

## 5. CI Documentation Gate

- [x] 5.1 Add a bounded CI documentation job that installs pinned documentation-only tooling, reuses npm/Rust caches where appropriate, and runs the repository documentation checks and build.
- [x] 5.2 Upload the assembled mdBook/Rustdoc site as a CI artifact without enabling an external deployment or changing repository hosting settings.
- [x] 5.3 Run authoritative deterministic screenshot verification on the pinned CI browser platform and report the scenario and asset for any mismatch.
- [x] 5.4 Confirm the documentation job does not mutate authored Markdown, translated README files, or committed screenshots.

## 6. Verification

- [x] 6.1 Run the focused README parity, link validation, documentation assembly, mdBook test, Rustdoc, and screenshot checks.
- [x] 6.2 Run `npm run lint`, `npm run test`, and `npm run build`.
- [x] 6.3 Run `cargo test --manifest-path src-tauri/Cargo.toml`, `cargo check --manifest-path src-tauri/Cargo.toml`, and `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`.
- [x] 6.4 Run `openspec validate establish-multilingual-documentation --strict` and `openspec validate --specs --strict`.
- [x] 6.5 Review the assembled English/Simplified-Chinese user guides, developer guide, Rustdoc reference, and all three README files for runtime truthfulness, language navigation, accessibility, and sensitive-data safety.

## 7. Verification Remediation

- [x] 7.1 Make documentation screenshot capture allocate an available loopback port, prohibit server reuse, and add focused port-selection coverage.
- [x] 7.2 Replace heuristic README version parsing with named stable facts, validate manifest-owned facts against `package.json`, and align React governance with the approved React 19 upgrade.
- [x] 7.3 Upgrade the native documentation boundary inventory to selected symbols and add negative regression tests for missing symbols and missing `///` documentation.
- [x] 7.4 Add pinned CI caching for mdBook, Cargo registries, and reusable Rustdoc intermediates while clearing assembled Rustdoc pages before each build.
- [x] 7.5 Re-run focused documentation checks, screenshot freshness, build assembly, project quality gates, and strict OpenSpec validation.
