# Native bounded contexts

Native code is organized by ownership rather than by UI page.

| Context | Owns |
| --- | --- |
| `agent_runtime` | Agent catalog, provider execution, terminal sessions, loops, Multi-Agent coordination |
| `sessions` | Sessions, messages, categories, chat configuration, export, usage |
| `workspaces` | Projects, worktrees, bounded file/Git queries, PTY shells |
| `tooling` | CLI, MCP, SDK, extensions, plugins, Skills, Prompt Hooks |
| `communications` | Connector configuration, credentials, routing, inbound delivery |
| `desktop` | Settings, paths, startup, window, tray, and floating lifecycle |
| `operations` | Observable operations and unified diagnostic/operation logging contracts |

A context publishes an `api.rs` facade for in-process consumers. Other contexts must not reach into its repository or infrastructure modules. Bootstrap modules compose concrete dependencies at the application edge.

Tauri commands are transport adapters, not business services. Cross-command error values are mapped to safe strings or explicit transport error DTOs.

For the full implemented context and command inventory, read [`src-tauri/ARCHITECTURE.md`](../reference/native-architecture.md) alongside the generated [native API reference](native-api-reference.md).
