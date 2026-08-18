## 1. Record the baseline

- [x] 1.1 Capture `cargo machete src-tauri` and `npx depcheck` output verbatim, so the post-change runs can be compared entry by entry
- [x] 1.2 Capture the `npm run build` chunk summary including both KaTeX chunk **content hashes** and the verified lazy-chunk count — hashes, not just counts, are what make the `katex` removal checkable

## 2. Remove the unused native crates

- [x] 2.1 Remove `tracing`, `tracing-subscriber`, `tracing-opentelemetry`, `opentelemetry-appender-tracing`, and `opentelemetry-semantic-conventions` from `src-tauri/Cargo.toml`
- [x] 2.2 Confirm `opentelemetry`, `opentelemetry-otlp`, and `opentelemetry_sdk` remain — they are used directly by `otel_support.rs` and `otel_telemetry.rs`
- [x] 2.3 `cargo check --manifest-path src-tauri/Cargo.toml` passes; a failure here means a crate was reached through a macro or feature path and the analysis was wrong
- [x] 2.4 `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings` passes — `--all-targets` covers the test and bench targets `check` does not
- [x] 2.5 `cargo test --manifest-path src-tauri/Cargo.toml` passes with an unchanged test count

## 3. Remove the unused frontend dependencies

- [x] 3.1 Remove `react-hook-form`, after confirming no `useForm`, `zodResolver`, or `@hookform` reference exists anywhere in `src/`
- [x] 3.2 Remove `katex`, then run a clean `npm ci` and confirm `node_modules/rehype-katex/node_modules/katex` still exists — the chunk rule's precondition must survive the removal
- [x] 3.3 `npm run build` emits both KaTeX chunks with the **same content hashes** as the 1.2 baseline and the same verified lazy-chunk count
- [x] 3.4 `npx tsc --noEmit`, `npm run lint:ci`, and `npm run test` pass

## 4. Make the implicit preconditions explicit

- [x] 4.1 Comment the `rich-markdown-katex` rule in `vite.config.ts`: it matches a path that `package-lock.json` pins, not one any `package.json` constraint guarantees, and a re-resolved lockfile or a different package manager can move it
- [x] 4.2 Add `playwright` as a direct dependency at the version `@playwright/test` resolves to, noting that `playwright_sidecar_tests.rs` gates on its install path
- [x] 4.3 Add `[package.metadata.cargo-machete]` only if some crate survived review as a deliberate keep — omit the section entirely if none did, rather than adding an empty one

## 5. Prove nothing broke

- [x] 5.1 Re-run both tools and compare against the 1.1 baseline: the seven removed entries are gone, and each of the eleven that remain is a known configuration-driven keep
- [x] 5.2 `npm run architecture:check` and `npm run contracts:check` pass
- [x] 5.3 `npm run docs:check` passes
- [x] 5.4 Record the two-copies-of-KaTeX observation as follow-up scope rather than fixing it here
- [x] 5.5 `openspec validate prune-unused-dependencies --strict` and `openspec validate --specs --strict` pass
