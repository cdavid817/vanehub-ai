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

Run these commands before requesting review:

```powershell
npm run lint
npm run test
npm run contracts:check
npm run build
npx playwright test
cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
cargo check --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml
openspec validate --specs --strict
```

Also run `openspec validate <change-name> --strict` for every active change you modify.

## Commits and pull requests

- Write an imperative, scoped commit subject.
- Keep generated files and unrelated formatting out of the change.
- Explain user impact, implementation risk, validation evidence, and any follow-up work.
- Never commit credentials, signing material, local databases, or unredacted diagnostic logs.

All contributions are accepted under the repository's Apache-2.0 license and must follow the [Code of Conduct](CODE_OF_CONDUCT.md).
