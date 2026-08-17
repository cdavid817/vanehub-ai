# Agent lifecycle and provider runtime

This chapter covers how registered Agents (other than the built-in OnePiece) are edited, and how the runtime resolves a stable Agent id to a concrete provider contract without provider-identity branching in the application layer.

## Editing a registered API Agent

A user-created API Agent's display name, model id, Base URL, and stored API key are editable. The Agent's `id`, `provider`, and `interface format` are immutable through the ordinary edit operation. Edits re-validate like registration: an omitted required Base URL for an `openai-compatible` Agent rejects the whole edit without persisting any part of it. A rotated API key replaces the stored credential and takes effect on the next generation.

OnePiece is the exception: it uses dedicated catalog-backed provider-**Profile** operations that preserve stable id `onepiece` while allowing multiple independently secured provider/endpoint/model configurations and one explicit active Profile. OnePiece's provider, endpoint type, interface format, and Base URL are resolved from the selected built-in directory entry — never edited directly.

## Stable provider resolution

The Agent Runtime resolves supported built-in CLI runtime behavior through a **provider registry** keyed by the Agent registry entry's stable id. Provider-neutral application and Session modules do not branch on provider identity to select behavior. An Agent id with no compatible provider registration returns a classified `unsupported-provider` error with no fallback to another provider.

## Provider metadata and capabilities

Each registered provider declares validated metadata, readiness prerequisites, and supported runtime capabilities (interaction, resume, structured-output, terminal, usage, permission, model-selection, reasoning) independently of display-name matching or caller inference. A capability not declared by a provider is not silently assumed.

## Where the design lives

This chapter orients contributors. The authoritative requirements live in the specs.

- [openspec/specs/agent-lifecycle-management](../../../openspec/specs/agent-lifecycle-management/spec.md)
- [openspec/specs/agent-provider-runtime](../../../openspec/specs/agent-provider-runtime/spec.md)
- [openspec/specs/agent-switching](../../../openspec/specs/agent-switching/spec.md)

The native execution path lives in the `agent_runtime` bounded context; see [Native bounded contexts](native-contexts.md).
