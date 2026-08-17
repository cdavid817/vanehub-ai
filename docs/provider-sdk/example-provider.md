# Test Fixture Provider

The repository's fixture provider exists only under Rust `#[cfg(test)]` composition. It demonstrates the extension seam without creating a production provider or external loading authority.

Implement `AgentProvider` with domain-validated metadata/capabilities, bounded parser/version/cancellation policies, side-effect-free health classification, safe option validation, and generation/interactive invocation specifications. Add it to a test `ProviderRegistry`, then run the conformance harness. Do not edit Session orchestration, usage projection, Tauri commands, or Web/Tauri frontend adapters.

Use repository fixture output containing a token, opaque runtime session id, usage completion and a classified failure. Exercise cancellation with a bounded process-tree policy. The fixture executable must be created by the test runtime and must never appear in the production registry.

This example is intentionally not an installation tutorial: external providers remain unsupported in schema version 1.
