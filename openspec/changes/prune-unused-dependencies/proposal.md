## Why

The optimization ticket's item 12 assumed the repository might carry zombie dependencies but recorded that this was unverified. Running the tools and checking every entry by hand settles it: **the tools report 18 unused dependencies, of which 6 are real.**

Native side, `cargo machete` flags five, and all five check out — zero references across `src-tauri/src`, `build.rs`, and `tests/`:

| Crate | Declared as |
|---|---|
| `tracing` | `=0.1.44`, features `attributes`, `std` |
| `tracing-subscriber` | `=0.3.23`, features `registry`, `std` |
| `tracing-opentelemetry` | `=0.33.0` |
| `opentelemetry-appender-tracing` | `=0.32.0` |
| `opentelemetry-semantic-conventions` | `=0.32.1`, feature `semconv_experimental` |

They are exactly the `tracing`-to-OpenTelemetry bridge. What is *not* flagged is telling: `opentelemetry`, `opentelemetry-otlp`, and `opentelemetry_sdk` are all used, by `otel_support.rs` and `otel_telemetry.rs`, which call the OpenTelemetry API directly. The bridge layer was declared and never wired up. Keeping it also sits badly with the repository's rule that diagnostics go through the unified logging service rather than a second `tracing` facade.

Frontend side, `depcheck` flags thirteen and **one is real**: `react-hook-form`, which has no `useForm`, no `zodResolver`, and no `@hookform` reference anywhere in `src/`. The other twelve are used through mechanisms import analysis cannot see: `@commitlint/*` and `lint-staged` run from husky, `@wdio/*` and `expect-webdriverio` back `npm run test:desktop`, `tailwindcss` and `tailwindcss-animate` are reached through `@tailwindcss/vite` and a `@plugin` directive in `src/styles.css` — and `katex`, which is the one worth writing down.

### `katex` is imported from CSS, and removing it broke CI

`katex@0.18.4` is declared at the top level and no JavaScript or TypeScript file imports it. Grepping for `from "katex"` and `require("katex")` finds nothing, which is why `depcheck` reports it and why removing it looked safe.

It is imported from **`src/styles.css:3`**:

```css
@import 'katex/dist/katex.min.css';
```

Removing it built cleanly on Windows and failed on the Linux CI runner with `Can't resolve 'katex/dist/katex.min.css'`. The local green build was not evidence of anything: the two platforms resolved the same manifest differently, and only the stricter one told the truth.

The lesson is narrow and worth keeping. **`src/styles.css` is a second import graph**, and this audit already had one hit in it — `tailwindcss-animate` was correctly cleared as a false positive precisely *because* of its `@plugin` directive on line 2 of that file. Line 3 went unchecked. A dependency review that greps only JavaScript import syntax will keep making this mistake, and a passing local build will keep failing to catch it.

`katex` therefore stays, with a comment at the chunk rule in `vite.config.ts` recording both why it looks unused and the rule's other implicit precondition: it matches a `package-lock.json`-pinned nested path rather than anything a `package.json` constraint guarantees, which is consistent with the prior incident where a pnpm layout broke it.

### A separate finding, not addressed here

The build ships **two copies of KaTeX**: `rich-markdown-katex` at 258.68 kB and `katex` at 258.68 kB, about 155 kB gzip together. That is the nested-copy layout reaching the bundle, and it predates this change. Deduplicating it means re-resolving the lockfile and reworking the chunk rule, which is a bundling change with its own risk, not a dependency cleanup.

## What Changes

- Remove the five unused native crates and `react-hook-form`.
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
- `package.json` and `package-lock.json` — `react-hook-form` removed, `playwright` added; `katex` retained and now documented.
- `vite.config.ts` — a comment at the chunk-splitting rule.
- No Rust or TypeScript source file changes, so no runtime behavior is affected in either the desktop or Web runtime.
