# OnePiece native Agent

OnePiece is VaneHub's built-in first-party Agent. Unlike CLI-backed Agents, it runs entirely through the native API runtime: `launch_kind = api`, `agent_origin = builtin`, reserved stable id `onepiece`. It is seeded into the registry on first launch and stays visible even before any provider configuration or credential exists.

## Identity and lifecycle

The OnePiece identity is owned by the registry, not by provider configuration. It is separated from multiple named catalog-backed upstream-provider **Profiles**, each securing its own credentials independently. At most one Profile is explicitly active for runtime generation at a time. Profile creation must select a reviewed endpoint type owned by the chosen provider — arbitrary provider identity, interface format, or Base URL are not accepted from the user.

## Where the design lives

This chapter orients contributors. The authoritative requirements — stable identity, registry seeding, reserved-id collision handling, the Profile lifecycle, and the provider-directory contract — live in the spec.

- [openspec/specs/onepiece-native-agent](../../../openspec/specs/onepiece-native-agent/spec.md)

The provider directory shared with CLI Agent configuration and the native API runtime are covered in [Runtime and service boundaries](runtime-boundaries.md) and [Native bounded contexts](native-contexts.md).
