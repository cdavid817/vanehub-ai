## Why

`native-runtime-architecture` requires the architecture test to inspect domain, application, infrastructure, and command sources and to reject private cross-context dependencies. It does not.

`src-tauri/tests/architecture.rs` resolves a file's scope by reading `contexts/<context>/<layer>`. `tooling` is the one context with subdomains, so its files are `contexts/tooling/<subdomain>/<layer>`; the third segment is the subdomain, matches no layer name, and `source_scope` returns `None`. `analyze` then returns no violations for that file. Every file in all nine `tooling` subdomains — `cli`, `cli_config`, `extensions`, `extension_platform`, `mcp`, `plugin_integrations`, `prompt_hooks`, `sdk`, `skill_tools`, `skills` — is skipped rather than checked, and the same will apply to the planned `permissions/rules` subdomain.

The gap is silent in both directions: nothing reports that those files were skipped, and the suite passes. It was found while adding `tooling::extension_platform`, whose whole point is to be held to these rules.

Turning the rules on surfaces two pre-existing violations. Both are real, both belong to code this change must repair rather than exempt:

* `tooling/extensions/application/ocr_admission.rs` imports `artifacts::application::{ArtifactDescriptor, ArtifactService}`. `artifacts` has no `api.rs`, which `openspec/project.md` already records as owed.
* `tooling/skill_tools/domain/permission_manifest.rs` imports `std::net::IpAddr` to classify whether a declared network origin is a public address. The detector forbids the whole `std::net` path, drawing no line between an address value type and a socket.

## What Changes

* Make `source_scope` resolve both `contexts/<context>/<layer>` and `contexts/<parent>/<subdomain>/<layer>`, and make the inward-dependency rule find the layer segment past a subdomain. Add positive and negative fixtures for top-level paths, nested paths, and both path separators.
* Give `artifacts` a published `api.rs` and route `tooling::extensions` through it, removing the cross-context reach into another context's application layer.
* Replace the blanket `std::net` ban with a semantic rule: address value types (`IpAddr`, `Ipv4Addr`, `Ipv6Addr`, `AddrParseError`) stay permitted in domain and application; socket and resolution types (`TcpStream`, `TcpListener`, `UdpSocket`, `ToSocketAddrs`, `SocketAddr` construction paths that imply binding) stay forbidden. This matches the main spec's wording, which forbids depending on network *APIs*, not on the concept of an address.
* Enable the nested checks for real, with no file-level or path-level exemption list.

## Capabilities

### Modified Capabilities

* `native-runtime-architecture`: the architecture test's scope resolution covers nested subdomains, the forbidden-technology rule distinguishes network value types from network I/O, and `artifacts` publishes a cross-context API like every other consumed context.

## Impact

* Native architecture test: `source_scope`, `is_forbidden_outer_layer`, and `is_forbidden_technology` change; new fixtures cover nesting and separators.
* `artifacts`: gains `api.rs`. No behavior, storage, or command change.
* `tooling::extensions`: imports move from `artifacts::application` to `artifacts::api`.
* `tooling::skill_tools`: no change; its `IpAddr` use becomes permitted by a rule that is correct rather than by an exemption.
* Nine `tooling` subdomains come under architecture enforcement for the first time. They are otherwise clean today, so no further repair is expected — but this is the change that would surface any that is not.
* Blocks `add-unified-extension-platform` Task Group 1: its new subdomains must be enforced from their first commit, not retrofitted.
