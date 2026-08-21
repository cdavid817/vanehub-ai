## 1. Scope resolution covers nested subdomains

- [x] 1.1 Make `source_scope` in `src-tauri/tests/architecture.rs` resolve both `contexts/<context>/<layer>` and `contexts/<parent>/<subdomain>/<layer>`, carrying the subdomain on `SourceScope`.
- [x] 1.2 Make `is_forbidden_outer_layer` find the layer segment past a subdomain so a nested domain module reaching its own application or infrastructure layer is rejected.
- [x] 1.3 Add unit fixtures for scope resolution: flat context paths, nested subdomain paths, command paths, unrecognised paths, and both `/` and `\` separators producing identical scopes.
- [x] 1.4 Add positive and negative fixtures proving a nested subdomain is rejected for its own outer-layer import and accepted for a sibling subdomain's published API.

## 2. Artifacts publishes a cross-context API

- [x] 2.1 Add `src-tauri/src/contexts/artifacts/api.rs` publishing the contract its consumers need, following the `api.rs` conventions of the other contexts.
- [x] 2.2 Route `tooling/extensions/application/ocr_admission.rs` through `artifacts::api` instead of `artifacts::application`.
- [x] 2.3 Update `openspec/project.md` where it records that `artifacts` has no `api.rs`.
- [x] 2.4 Leave `artifacts` behavior, storage, commands, and ownership unchanged; this is a visibility boundary, not a redesign.

## 3. Network technology rule becomes semantic

- [x] 3.1 Replace the blanket `std::net` entry in `is_forbidden_technology` with an allowlist of address value types (`IpAddr`, `Ipv4Addr`, `Ipv6Addr`, `AddrParseError`) and a denylist of I/O types (`TcpStream`, `TcpListener`, `UdpSocket`, `ToSocketAddrs`, `Incoming`).
- [x] 3.2 Add fixtures proving an address value type is accepted in domain and a socket type is still rejected, so the relaxation cannot widen silently.
- [x] 3.3 Leave `tooling/skill_tools/domain/permission_manifest.rs` unchanged; its origin classification is the case the semantic rule exists for.

## 4. Enable and verify

- [x] 4.1 Run the architecture test with nested resolution active and confirm the only violations are the two this change repairs.
- [x] 4.2 Add no file-level or path-level exemption list, and add no `#[allow]` that suppresses an architecture finding.
- [x] 4.3 Run `npm run architecture:check`, `npm run contracts:check`, `npm run lint:ci`, `npm run test`, `npm run build`, `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`, `cargo check --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `npm run native:panic:check`, and `cargo test --workspace`.
- [x] 4.4 Run `openspec validate fix-nested-context-architecture-enforcement --strict` and `openspec validate --specs --strict`.
