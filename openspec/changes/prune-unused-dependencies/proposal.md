## Why

The optimization ticket's item 12 assumed the repository might carry zombie dependencies but recorded that this was unverified. Running the tools and checking every entry by hand settles it: **the tools report 18 unused dependencies, of which 7 are real.**

Native side, `cargo machete` flags five, and all five check out — zero references across `src-tauri/src`, `build.rs`, and `tests/`:

| Crate | Declared as |
|---|---|
| `tracing` | `=0.1.44`, features `attributes`, `std` |
| `tracing-subscriber` | `=0.3.23`, features `registry`, `std` |
| `tracing-opentelemetry` | `=0.33.0` |
| `opentelemetry-appender-tracing` | `=0.32.0` |
| `opentelemetry-semantic-conventions` | `=0.32.1`, feature `semconv_experimental` |

They are exactly the `tracing`-to-OpenTelemetry bridge. What is *not* flagged is telling: `opentelemetry`, `opentelemetry-otlp`, and `opentelemetry_sdk` are all used, by `otel_support.rs` and `otel_telemetry.rs`, which call the OpenTelemetry API directly. The bridge layer was declared and never wired up. Keeping it also sits badly with the repository's rule that diagnostics go through the unified logging service rather than a second `tracing` facade.

Frontend side, `depcheck` flags thirteen and **two are real**: `react-hook-form`, which has no `useForm`, no `zodResolver`, and no `@hookform` reference anywhere in `src/`, and `katex` — see below. The remaining eleven are configuration-driven and invisible to import analysis: `@commitlint/*` and `lint-staged` run from husky, `@wdio/*` and `expect-webdriverio` back `npm run test:desktop`, `tailwindcss` and `tailwindcss-animate` are reached through `@tailwindcss/vite` and a `@plugin` directive in `src/styles.css`.

### `katex`, and a theory that measurement killed

`katex@0.18.4` is declared at the top level and imported by nothing. `vite.config.ts:29` splits the KaTeX chunk by matching the *nested* path `node_modules/rehype-katex/node_modules/katex/`, and the dependency tree shows all three consumers — `mermaid`, `rehype-katex`, `remark-math` — resolving to a different version, `0.16.47`. It is easy to read that as "the top-level declaration holds the versions apart, which is what keeps the consumers' copies nested where the pattern can find them", and conclude the dependency is load-bearing.

That reading is wrong, and only building proves it. With `katex` removed and a clean `npm ci`:

- `node_modules/rehype-katex/node_modules/katex` still exists,
- `npm run build` emits the same two chunks with the same content hashes, `rich-markdown-katex-CF7tamGr` and `katex-DolUETbr`,
- the lazy-chunk verification still reports 16.

The nesting comes from the lockfile, not from the version skew — `rehype-katex` asks for `^0.16.0` and `mermaid` for `^0.16.45`, both resolving to the same `0.16.47`, and `package-lock.json` simply pins them at nested paths. The top-level declaration contributes nothing.

Two things follow. The dependency is removable, and the chunk rule's real fragility is its dependence on a lockfile-pinned install path — which is consistent with the prior incident where a pnpm layout broke this exact rule.

### A separate finding, not addressed here

The build ships **two copies of KaTeX**: `rich-markdown-katex` at 258.68 kB and `katex` at 258.68 kB, about 155 kB gzip together. That is the nested-copy layout reaching the bundle, and it predates this change. Deduplicating it means re-resolving the lockfile and reworking the chunk rule, which is a bundling change with its own risk, not a dependency cleanup.

## What Changes

- Remove the five unused native crates, `react-hook-form`, and `katex`.
- Declare `playwright` directly. `depcheck` reports it missing: `playwright_sidecar_tests.rs` gates on `node_modules/playwright/package.json` existing, but it is present only transitively through `@playwright/test`.
- Record at `vite.config.ts` that the chunk rule matches a lockfile-pinned nested path under npm, so the next reader knows what would break it.
- **No behavior changes.** No production source is edited beyond dependency declarations and comments.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

None. Dependency declarations and comments only; no capability behaviour changes. The change sets `skip_specs: true`.

## Impact

- `src-tauri/Cargo.toml` and `Cargo.lock` — five dependencies removed.
- `package.json` and `package-lock.json` — `react-hook-form` and `katex` removed, `playwright` added.
- `vite.config.ts` — a comment at the chunk-splitting rule.
- No Rust or TypeScript source file changes, so no runtime behavior is affected in either the desktop or Web runtime.
