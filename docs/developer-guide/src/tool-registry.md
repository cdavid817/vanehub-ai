# Tool registry and execution

Native API Agents (including OnePiece) receive a fixed, provider-agnostic tool catalog, and the runtime drives a multi-turn tool-use loop until the provider returns a terminal response. MCP-sourced tools are layered on top of this fixed catalog — see [MCP tools and clients](mcp-tools.md).

## The fixed native tool catalog

Every native API generation (`launch_kind = api`) declares the same fixed tool set in its outgoing provider request:

- `shell` — command execution
- `file` — read/write
- content search
- filename search
- scoped file edit
- cross-session memory

Each tool is defined once and translated into the request shape required by the session's `interface_format`:

- `anthropic` → `{name, description, input_schema}`
- `openai-compatible` → `{type: "function", function: {name, description, parameters}}`

## Multi-turn tool-use loop

When a provider response requests one or more tool calls, the runtime executes those calls and sends their results back as a new turn, repeating until the provider returns a response with no further tool calls. A response with no tool calls is the terminal response for that user message, identical to a tool-free generation. The loop is bounded by a fixed maximum number of round trips per user message; exceeding it is handled explicitly rather than looping forever.

## Skill-supplied tools

A Skill may contribute its own tools on top of the fixed catalog, executed in a sandbox rather than in the host process. The requirements are in [openspec/specs/skill-tool-runtime](../../../openspec/specs/skill-tool-runtime/spec.md).

## Where the design lives

This chapter orients contributors. The authoritative requirements live in the specs.

- [openspec/specs/agent-tool-execution](../../../openspec/specs/agent-tool-execution/spec.md) — the fixed catalog, per-format translation, and the tool-use loop.
- [openspec/specs/agent-tool-registry](../../../openspec/specs/agent-tool-registry/spec.md) — the registered Agent catalog and capability metadata.
- [openspec/specs/skill-tool-runtime](../../../openspec/specs/skill-tool-runtime/spec.md) — sandboxed execution of Skill-supplied tools.

Tool execution lives in the `agent_runtime` bounded context; see [Native bounded contexts](native-contexts.md).

### Historical decision records

These record decisions taken at a point in time and are not maintained as current narrative. They are linked here so they are reachable rather than orphaned; the specs above remain authoritative.

- [Skill Tool Runtime Security](../../architecture/skill-tool-runtime-security.md) — the dependency review, verification evidence, rollout, and rollback recorded when the sandboxed Skill Tool runtime shipped.
