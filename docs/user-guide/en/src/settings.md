# Settings center

How the settings centre is grouped, and the basic configuration it holds: interface language, theme, font size, the default permission template, launch at login, network proxy, and the data and log directories.

Settings that belong to one feature are documented in that feature's own chapter; this one covers the settings centre itself and the cross-cutting basics.

**Settings** in the activity bar opens the settings center: navigation on the left, the configuration page on the right. There are 20 settings pages:

| Settings page | What it holds |
| --- | --- |
| **Basic Configuration** | See [the next section](#basic-configuration) |
| **CLI Management** | Install detection, conflict diagnostics, and upgrades for each CLI — see [Install and authenticate a CLI](getting-started.md) |
| **CLI Parameters** | Launch flags per CLI Agent — see [Tools and extensions](agent-configuration.md#cli-parameters) |
| **Extension Capabilities** | Installing and enabling local multimodal capabilities — see [Tools and extensions](extensions.md#extension-capabilities) |
| **Plugin Integrations** | Built-in product integrations and readiness checks — see [Plugin integration](plugin-integration.md) |
| **MCP Servers** | MCP server configuration and per-Agent binding — see [MCP servers](mcp.md) |
| **Agent Configurations** | Provider, endpoint, and model per Agent, including OnePiece — see [Tools and extensions](agent-configuration.md#agent-configurations) |
| **Agent Policies** | Permission policy and approval templates — see [Permission approvals](permissions.md) |
| **Expert Roles** | Role fields, responsibilities, and review policy — see [Expert roles](expert-roles.md) |
| **AI Personalization** | Overview, Instructions, Memory, and Runtime Preview — see [Personalization](personalization.md) |
| **Skills** | Skill installation and binding — see [Manage Skills](skill-management.md) |
| **Prompt Hooks** | Hook management — see [Prompt Hooks](prompt-hooks.md) |
| **IM Connectors** | IM connector configuration — see [Remote and IM](im-connectors.md) |
| **SSH Connections** | Saved SSH connections — see [Remote and IM](remote-workspaces.md) |
| **Execution Observability** | Execution tracing and log collection policy — see [Observability](observability.md) |
| **Usage Statistics** | Token usage statistics — see [Usage statistics](usage-statistics.md) |
| **Code Intelligence** | Language server enablement, discovery, and workspace trust — see [LSP code intelligence](lsp-code-intelligence.md) |
| **Local Media** | Local OCR, speech recognition, and speech synthesis engines — see [Local media](local-media.md) |
| **About** | Version, update check, changelog, and repository links — see [Application updates](app-updates.md) |
| **Documentation** | Renders the bundled product documentation in your interface language |

## Basic configuration

**Settings → Basic Configuration** is the default landing page of the settings center, governing the application's own behavior — nothing here is specific to a given Agent.

![The Basic Configuration settings page](assets/screenshots/settings-basic-en.png)

| Group | Item | Notes |
| --- | --- | --- |
| **Appearance** | Interface language | The client defaults to following the host system's locale |
| | Theme, font size | Affects global rendering |
| **Security** | Default policy template | The default template for new sessions; see [Permission approvals](permissions.md) for the semantics |
| **Startup** | Launch at login | Tied to the [system tray](user-interface.md#system-tray) |
| | Floating assistant switch | The [floating assistant](user-interface.md#floating-assistant) window only exists once this is on |
| **Network** | Node info, network proxy | The proxy supports authentication |
| **Storage** | Data directory, log directory | Changing either requires a restart and rebuilds under the new directory; see [Troubleshooting](troubleshooting.md) for log-path details |
| | Folder opener | Decides what "Open in file manager" actually invokes |

> **Be careful changing the data directory.** When multiple worktrees share the same database, migration version numbers can collide across branches — see [Troubleshooting](troubleshooting.md).

## Related

- Permission templates → [Permissions](permissions.md)
- Agent and CLI configuration → [Agent and CLI configuration](agent-configuration.md)
- Main window layout → [User interface](user-interface.md)
