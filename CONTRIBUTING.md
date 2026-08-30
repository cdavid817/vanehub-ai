# Contributing to VaneHub AI

Thank you for helping improve VaneHub AI. Keep each change focused and open an issue first when the expected behavior or design is not already clear.

## Development setup

Use Node.js 22+, npm, stable Rust, and the native prerequisites for Tauri 2 on your platform.

```powershell
npm ci
npm run dev
```

The project uses npm and `package-lock.json`; do not introduce pnpm or Yarn lockfiles.

## Change workflow

1. Create a branch from `main`.
2. For a new feature or architecture change, create an OpenSpec proposal under `openspec/changes/` and validate it before changing code.
3. Keep React components behind `src/services/agent-service.ts`. Any new native capability must be implemented by both the Tauri and Web/mock adapters.
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

## Commits and pull requests

- Write an imperative, scoped commit subject.
- Keep generated files and unrelated formatting out of the change.
- Explain user impact, implementation risk, validation evidence, and any follow-up work.
- Never commit credentials, signing material, local databases, or unredacted diagnostic logs.

All contributions are accepted under the repository's Apache-2.0 license and must follow the [Code of Conduct](CODE_OF_CONDUCT.md).
