# Testing, packaging, and release

Run the repository verification commands appropriate to the change:

```powershell
npm run lint
npm run test
npm run build
cargo test --manifest-path src-tauri/Cargo.toml
cargo check --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
openspec validate --specs --strict
```

Documentation changes additionally run:

```powershell
npm run docs:check
npm run docs:test
npm run docs:screenshots:check
npm run docs:build
```

Frontend tests cover pure contracts and visible component behavior. Playwright covers user-visible runtime paths. Native tests cover domain invariants, application port orchestration, persistence/migrations, command mapping, process safety, and lifecycle behavior.

Packaging targets Windows, macOS, and Linux through Tauri. Signing credentials belong in protected release environments, never in repository configuration or screenshots. See the checked-in [release signing guide](../reference/release-signing.md).
