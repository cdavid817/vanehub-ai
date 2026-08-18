## Context

See proposal.md — Why for the tool output, the per-entry verdicts, and the `katex` measurement.

The shape of the problem: of 18 reported entries, 11 are false positives caused by configuration-driven use, and 7 are real. Any process that trusts the tools deletes eleven working things; any process that ignores them keeps seven dead ones. Neither the tool nor a reading of the dependency tree is sufficient on its own — the `katex` entry looked load-bearing under a plausible theory and turned out not to be, and only a build settled it.

## Goals / Non-Goals

**Goals:**

- Remove the seven dependencies that are genuinely unused.
- Leave the `vite.config.ts` chunk rule's real precondition written down, since it is a lockfile-pinned path rather than anything visible in source.
- Make the frontend's reliance on `playwright`'s install path explicit rather than accidental.

**Non-Goals:**

- Reaching "both tools report zero unused dependencies", the ticket's stated acceptance criterion. Eleven of the remaining reports are correct-to-keep, and chasing the number would delete husky's commitlint, the desktop test harness, and the Tailwind plugin.
- Deduplicating the two KaTeX chunks. Real, worth doing, and a bundling change with its own risk — not this.
- Auditing transitive dependencies, version currency, or security advisories. Dependabot owns that.

## Decisions

### Remove the tracing bridge rather than wire it up

The five native crates form a coherent set: `tracing`, its subscriber, and the bridge to OpenTelemetry. They were declared for an architecture the code did not adopt — `otel_support.rs` and `otel_telemetry.rs` call the OpenTelemetry API directly.

Keeping them costs a real compile and audit surface for an unmade decision, and AGENTS.md already routes diagnostics through the unified logging service, so a second `tracing` facade would need justifying on its own terms.

*Alternative rejected — wire up the bridge instead*: that is a logging-architecture change touching every diagnostic in the crate. It cannot ride along in a dependency cleanup.

### Verify removals by building, not by trusting the analysis

`cargo machete` is a text-based analyser and can be wrong in both directions — a crate reached only through macro expansion, a re-export, or a feature-gated path can look unused. The check that matters is that `cargo check`, `cargo clippy --all-targets`, and the full test suite pass with the declarations gone.

The frontend equivalent is stronger: `npm run build` verifies its own lazy-chunk count, and the chunk content hashes are comparable before and after. That is what turned the `katex` question from an argument into a measurement, and it is the reason `katex` is being removed rather than annotated.

### Record what the chunk rule actually depends on

The rule matches `node_modules/rehype-katex/node_modules/katex/`, a path that exists because `package-lock.json` pins it there, not because of any version constraint in `package.json`. Under a different package manager or a re-resolved lockfile it can move, and a prior incident in this repository showed a pnpm layout breaking this exact rule.

So the comment states the npm-and-lockfile precondition specifically, rather than describing hoisting in the abstract. It goes at the rule, where a reader touching the pattern is already looking.

### Declare `playwright` directly rather than relying on hoisting

`playwright_sidecar_tests.rs` gates on `node_modules/playwright/package.json`. That path exists today because `@playwright/test` depends on `playwright` and npm hoists it — an install-layout accident being relied upon. Declaring it directly makes the gate's precondition explicit.

This interacts with issue #170: the same path check is what makes that test run on developer machines and no-op on CI. Declaring the dependency does not change that — CI still lacks the browser binaries — but it removes one layer of accident.

## Risks / Trade-offs

- **A removed crate turns out to be reached through a macro or feature path** → `cargo clippy --all-targets -- -D warnings` plus the full test suite is the check; a compile failure is immediate and unambiguous.
- **Removing `katex` changes the bundle in some way the chunk count does not catch** → Mitigated by comparing chunk content hashes, not just counts: the measured build produced `rich-markdown-katex-CF7tamGr` and `katex-DolUETbr` both before and after, which is identity, not merely equivalence.
- **Removing `react-hook-form` forecloses a form-validation direction** → It has no call sites to foreclose, and `zod` covers the spec's schema-backed requirement. Adding it back later is one line, in the change that would actually use it.
- **The `vite.config.ts` comment drifts** → The build's own chunk verification fails if the pattern stops matching, so the comment explains a failure the build already detects rather than being the only line of defence.

## Migration Plan

No deployment step and no data migration. Reverting is a plain `git revert` followed by `npm ci` and a Cargo build to restore the lockfiles.
