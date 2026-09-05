# Contributing to VaneHub AI

Thank you for helping improve VaneHub AI. Keep each change focused and open an issue first when the expected behavior or design is not already clear.

## Development setup

Use Node.js 22+, npm, stable Rust, and the native prerequisites for Tauri 2 on your platform.

```bash
npm ci
npm run dev
```

These commands are cross-platform; on Windows run them from PowerShell or any shell with Node on `PATH`. A map of every documentation set in this repository is at [docs/README.md](docs/README.md).

The project uses npm and `package-lock.json`; do not introduce pnpm or Yarn lockfiles.

## Change workflow

1. Create a branch from `main`.
2. For a new feature or architecture change, create an OpenSpec proposal under `openspec/changes/` and validate it with `openspec validate <change-name> --strict` before changing code.
3. Keep React components behind the domain service interfaces in `src/services/` — components never call Tauri `invoke()` directly. Any new native capability must keep the interface contract complete in both the Tauri and Web/mock adapters; the Web/mock side may return `unsupported`/`unavailable` or a deterministic simulation, and must never fake real native side effects.
4. Add or update automated tests for changed behavior.
5. Open a pull request using the repository template and link the issue or OpenSpec change.

Follow `AGENTS.md` and `openspec/project.md`. In particular, do not add TypeScript `any`, `@ts-ignore`, inline styles, feature-local native log files, or production Rust `unwrap()`/`expect()` calls.

## Required validation

Before requesting review, run **every** command in the「校验命令」(validation commands) section of [AGENTS.md](AGENTS.md), copying each command verbatim. This file intentionally does not duplicate the command list; AGENTS.md is the single source of truth.

The flags matter, and two of them are easy to get wrong:

- `npm run lint:ci`, not `npm run lint`.
- `cargo check`, `cargo clippy`, and `cargo test` take `--workspace`, not `--manifest-path src-tauri/Cargo.toml`. This repository is a Cargo workspace, and `--manifest-path` covers only the `vanehub-ai` crate — members such as `vanehub-permission-hook` are silently skipped. `cargo fmt` is the exception and does use `--manifest-path`, matching CI.

Every weaker variant above passes locally and is rejected by CI.

When your change touches the corresponding area, also run the conditional commands listed below that section: `npx playwright test` for UI behavior changes, the coverage and contract checks, and `openspec validate <change-name> --strict` for every active change you modify.

## Visual regression baselines

`npm run visual:test` runs the real Playwright `toHaveScreenshot()` baseline-comparison suite
(`playwright.visual.config.ts`, `tests/e2e-visual-regression/`). This is distinct from the
`page.screenshot()` capture-as-evidence tests under `tests/e2e/` (including the `*.visual.spec.ts`
files there), which write to the gitignored, per-run-wiped `test-results/e2e/` directory and prove a
surface renders without crashing, not that it matches a prior appearance. Baseline PNGs live in
`tests/e2e-visual-regression/*-snapshots/` and are committed to the repository.

These baselines are only valid for the OS that captured them — Playwright embeds the platform in the
filename (for example `-win32.png`) — see `playwright.visual.config.ts`'s own doc comment for why
this suite is not wired into CI's `e2e` job yet.

When a change intentionally alters a surface's appearance:

1. Run `npm run visual:update` to regenerate the affected baseline PNG(s) only — check the diff to
   confirm no unrelated baseline moved.
2. Run `npm run visual:test` at least twice in a row to confirm the new baseline is stable rather
   than flaky. A baseline that fails its own second run is worse than no baseline.
3. In the pull request description, add a short "Visual baseline update" note for every changed PNG:
   which surface/theme/locale/width changed and why the new appearance is correct. GitHub renders a
   diff view for changed PNGs directly in the "Files changed" tab — that rendered diff is the
   before/after reference; the note is the reviewer-facing judgment that the diff is an intentional
   change, not a regression slipping through as an accepted baseline.
4. Do not regenerate a baseline to silence a failing test without first reading why it changed.

## Commits and pull requests

- Write an imperative, scoped commit subject.
- Keep generated files and unrelated formatting out of the change.
- Explain user impact, implementation risk, validation evidence, and any follow-up work.
- Never commit credentials, signing material, local databases, or unredacted diagnostic logs.

All contributions are accepted under the repository's Apache-2.0 license and must follow the [Code of Conduct](CODE_OF_CONDUCT.md).
