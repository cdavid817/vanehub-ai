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

## Documentation changes

The documentation gate is reproducible from a clean checkout; nothing in it needs credentials.

1. `npm ci`, then install the pinned mdBook version from [docs/toolchain.json](docs/toolchain.json) (`cargo install mdbook --version <mdbook> --locked`). `scripts/docs-tooling.mjs` refuses to run with any other version.
2. `npm run docs:check` — README three-language parity, internal links and anchors, screenshot inventory, native documentation boundaries, the generated agent capability matrix (`docs:agents:check`), and the documentation fact tests (`scripts/docs-facts.node-test.mjs`, which pin statements in the guides to `package.json`, `playwright.config.ts`, the package workflow, and the MCP relay allowlist). Every generated file (`docs/reference/cli/parameter-matrix.md`, `docs/reference/agents/capability-matrix.md`) is regenerated from its source, never edited by hand.
3. `npm run docs:test` and `npm run docs:build` — `mdbook test` for the four books and the full assembled site. Both end with Rustdoc, so a stable Rust toolchain is required; if `cargo` is unavailable, report those two as **NOT RUN**, not as passed.
4. Only when a screenshot changes: `npx playwright install chromium`, then `npm run docs:screenshots:update` (captures from the Web/mock runtime and records the source commit in `docs/user-guide/screenshots.json`) and `npm run docs:screenshots:check`. Screenshots are UI previews, never desktop verification evidence.
5. `README.md` is canonical; `README.zh-CN.md` and `README.ja.md` must carry the same sections, commands, links, and fact markers. A product fact that changes in one README changes in all three, and the stable/`main` boundary note near the top must stay consistent.
6. User and developer guides ship in English and Simplified Chinese with the same facts, limits, and status in both; layout may differ. Use the status vocabulary in [docs/reference/terminology.md](docs/reference/terminology.md) (`stable`, `main` / unreleased, fixture-qualified, live-qualified, legacy, planned); a claim of "supported" or "delivered" must map to a main spec or a release manifest, never only to an active OpenSpec change.
7. Never edit `openspec/changes/archive/` or mark an OpenSpec task complete without the evidence it names; historical documents such as `docs/VaneHub-AI-技术架构深度解析.md` are snapshots and are not updated to the current architecture.

## Commits and pull requests

- Write an imperative, scoped commit subject.
- Keep generated files and unrelated formatting out of the change.
- Explain user impact, implementation risk, validation evidence, and any follow-up work.
- Never commit credentials, signing material, local databases, or unredacted diagnostic logs.

All contributions are accepted under the repository's Apache-2.0 license and must follow the [Code of Conduct](CODE_OF_CONDUCT.md).
