# MCP servers: connect external tools to an Agent

## What MCP is

**MCP (Model Context Protocol) is the standard protocol through which an Agent calls external tools.**

An Agent by itself has only its built-in capabilities — running commands, reading and writing files, searching, memory. To have it query a database, call an internal API, or drive a design tool, you connect an MCP server, and that server exposes its own tools to the Agent.

**Register it once in VaneHub AI and it can be handed to each Agent**, instead of writing it again in every CLI's configuration file. That is the main reason it exists: the five Agents each have their own configuration format and location, and keeping them in sync by hand drifts sooner or later.

## What it can do

- **Register external tool servers centrally**, isolated by user or project scope
- **Test a connection** and list the tools it discovers, so you verify it before handing it to an Agent
- **Forward centrally registered servers to external CLIs** (the relay), removing duplicate configuration
- **Import and export** Claude Desktop format configuration to migrate existing setups
- Put **every MCP tool call through your approval**, rather than opening it up once configured

What it cannot do: **it does not write any CLI's own configuration files** (binding is achieved through launch flags and the relay), and **it does not guarantee every Agent can use the same set** — see the relay's scope below.

## What a server configuration is made of

| Field | Notes |
| --- | --- |
| **Name** | **Globally unique kebab-case** — see the naming rules below |
| **Transport** | `stdio`, legacy `sse`, or `streamable_http` |
| Transport-specific fields | stdio needs a launch command; both URL transports need a URL |
| Description | Optional |
| Active flag | Whether it takes part in delivery |
| Scope | User configuration / project configuration |
| Project path | The owning directory when project-scoped |

### Three transports

| Transport | Required | Suits |
| --- | --- | --- |
| **stdio (local process)** | The launch command | A server you can execute locally |
| **Legacy SSE** | A URL | Early MCP's SSE endpoint |
| **Streamable HTTP** | A URL | The current HTTP streaming protocol |

**The transport must be declared explicitly.** An unrecognized transport value in a configuration, an import entry, or a database row is rejected and is **never quietly treated as `stdio`** — doing so would connect over the wrong protocol, which presents as connecting successfully while nothing works.

### The naming rules are stricter than you would guess

A name must be kebab-case, and all of these are rejected: **an empty name, uppercase letters, spaces, underscores, and leading or trailing hyphens**.

**Duplicate detection spans every scope**: once a name is taken in any scope it cannot be used elsewhere — on creation, import, or rename alike — and it **does not overwrite the existing configuration**, it is rejected or skipped.

## Register and test

Select **Add MCP** under **Settings → MCP Servers** to create one. Configuration is displayed in two groups by scope, **User configuration** and **Project configuration**.

![The MCP Servers settings page showing user and project configuration groups](assets/screenshots/mcp-en.png)

Each server card can **test** its connection. A passing test lists the tools it discovered along with the elapsed time. There are four states:

| State | Meaning |
| --- | --- |
| **Test passed** | The most recent test connected |
| **Test failed** | The most recent test failed, with the error |
| **Not tested** | Never tested — **not the same as unusable** |
| **Disabled** | You turned it off; not the same as a failed test |

**All four read from cache, not from a live connection.** Test results — status, discovered tools, error message, connection timestamp, and duration — are persisted locally, and asking for status **starts no process and opens no network connection**. So what you see is "the conclusion of the last test", not "whether it works right now".

When you disable a server, **the last test's details are preserved** for you to read.

## Import and export

**Import/Export** in Claude Desktop format is supported. The type-inference rules on import are stated in the interface:

- An explicit `type=sse` → imported as legacy SSE
- `type=http`, `streamable_http`, and **a URL with no declared type** → imported as Streamable HTTP

## Relay: let external CLIs use the same MCP servers

VaneHub AI can forward centrally registered MCP servers to external CLIs, so you do not have to configure them again inside the CLI.

> **Relay is currently enabled only for Claude Code and Codex CLI.** Gemini CLI, OpenCode, and Antigravity CLI need their own configuration, and their MCP calls do not appear in the execution trace.

The relay is **explicitly enabled and scoped to one invocation**: it forwards the MCP protocol **without mutating your global provider configuration**, and records correlated proxied request lifecycle telemetry.

With the relay disabled, or against a provider that cannot accept invocation-scoped configuration, Agent execution continues along its existing path — MCP visibility is simply reported as **inferred** or **opaque** rather than **relayed**. In other words: **what the relay buys you is observability, not capability.** See the fidelity table in [Observability](observability.md).

## Every MCP tool call needs your approval

**This is MCP's most important safety design: every MCP-sourced tool call requires explicit approval, with no automatic path.**

When the model requests any MCP tool — whatever the server, whatever the tool — the tool-use loop pauses for you to approve or deny.

If you deny, the system **does not connect to that MCP server and does not invoke the tool**, and reports the denial to the model as the tool's result, matching how denied shell and file calls are reported.

There is a second check at call time: **the target server's visibility is re-validated before connecting.** Even if it appeared in the tool catalog offered earlier in the same generation, a server that is not currently visible and active for the session's workspace folder — a project-scoped server belonging to a different project, for instance — makes the call fail as an error, **with no connection attempted**.

## Resource limits

Limits are enforced in **both** frontend contract validation and the native runtime, with the backend authoritative.

| Subject | Limit |
| --- | --- |
| One server configuration | 128 args / 128 environment entries / 128 headers; 256 KiB serialized transport configuration |
| One protocol frame | 2 MiB each for a JSON-RPC frame, an SSE event, and an HTTP response body |
| One server's tool catalog | 128 tools / 2 MiB serialized; 256-byte tool name; 8 KiB description; one input schema at most 128 KiB and 32 JSON levels deep |

Anything over the limit is rejected with the safe code `limit_exceeded`, **before the server is persisted or launched**. When protocol input exceeds its limit, the transport stops reading at limit-plus-one and performs bounded cleanup.

**An oversized tool catalog does not replace that server's cached catalog** — better to keep the old one than let a malformed response wash away good data.

When one server's cached catalog is malformed or over the limit, **only that server's tools are excluded**, with one bounded diagnostic, and the remaining servers are processed normally rather than failing the whole visible catalog.

## Notes and limits

- **Environment variables and headers are stored as plaintext.** They land in the local database as plaintext JSON, and **exports carry them as plaintext too**. Decide on that basis whether to put long-lived credentials here.
- **Status is cached** and does not represent current connectivity; test again to confirm.
- **The relay covers only Claude Code and Codex CLI**; the other three CLIs need their own configuration and their calls do not reach the execution trace.
- **Every MCP tool call needs approval**; there is no "trust this server" switch that skips it.
- **Names are globally unique and format-constrained**, and a duplicate is rejected rather than overwriting.

## Related

- The rest of the tool and extension configuration → [Tools and extensions](tooling.md)
- The approval flow and remembered scopes → [Permission approvals](permissions.md)
- What fidelity a relayed call gets in the trace → [Observability](observability.md)
- The MCP protocol itself: transports, core primitives, lifecycle, and the authorization model → [MCP technical architecture](../../../agent-infrastructure/mcp-architecture.md) (Simplified Chinese)
