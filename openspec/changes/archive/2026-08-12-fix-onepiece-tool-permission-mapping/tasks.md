## 1. Regression Coverage

- [x] 1.1 Add a table-driven test covering the permission action and resource for every built-in OnePiece tool.
- [x] 1.2 Add explicit regression assertions that MCP stays `mcp.tool` and unknown tools stay fail-closed as `unknown:*`, then confirm the new built-in cases fail before implementation.

## 2. Runtime Mapping

- [x] 2.1 Map read-only search and retrieval tools to established read actions with workspace or memory resources.
- [x] 2.2 Map `edit` to `file.write` with its requested file path while preserving existing shell, file, remember, LSP, MCP, and unknown mappings.
- [x] 2.3 Run the focused Rust tests and confirm all mapping cases pass.

## 3. Verification

- [x] 3.1 Run `openspec validate fix-onepiece-tool-permission-mapping --strict` and `openspec validate --specs --strict`.
- [x] 3.2 Run `npm run lint:ci`, `npm run test`, and `npm run build`.
- [x] 3.3 Run `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`, `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`, `cargo test --manifest-path src-tauri/Cargo.toml`, and `cargo check --manifest-path src-tauri/Cargo.toml`.
