# MCP tools and clients

VaneHub integrates Model Context Protocol (MCP) servers in two layers: client configuration/management, and exposure of a server's tools in the native Agent tool catalog.

## Server configuration model

An MCP server configuration has a globally unique kebab-case name; an explicit transport type (`stdio`, legacy `sse`, or `streamable_http`); transport-specific fields; description; active flag; scope; and project-path metadata. Unknown transport values are rejected — they are never silently reinterpreted as `stdio`. Historical `sse` rows are transactionally migrated to `streamable_http` so their previously effective protocol behavior is preserved.

## Tools in the native catalog

Alongside the fixed `shell`/`file`/`remember` tools, the native tool catalog includes bounded entries exposed by MCP servers that are **visible and active** for the current session's workspace folder. The catalog uses each server's most recently cached valid tool list from a "Test Connection" result — not a live connection made at catalog-build time. Consequences:

- An untested or failed server contributes no tools.
- An inactive or out-of-scope server contributes no tools.
- MCP catalog names never collide with or shadow the fixed `shell`/`file`/`remember` tools.
- A catalog lookup failure degrades gracefully: the generation proceeds with only the fixed catalog rather than failing.

## Where the design lives

This chapter orients contributors. The authoritative requirements live in the specs.

- [openspec/specs/mcp-client-management](../../../openspec/specs/mcp-client-management/spec.md) — configuration model, transports, migration.
- [openspec/specs/agent-mcp-tools](../../../openspec/specs/agent-mcp-tools/spec.md) — MCP-sourced tools in the native catalog.

MCP configuration lives in the `tooling` bounded context; see [Native bounded contexts](native-contexts.md).
