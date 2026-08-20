# LSP code intelligence

> **Feature state:** Implemented for the Tauri desktop runtime and local workspaces. Web/mock mode provides deterministic settings and status previews only; it does not inspect files or launch a language server.

Language Server Protocol (LSP) integration lets the native API Agent ask a local language server for definitions, references, hover information, and current diagnostics. It is disabled by default and requires both language enablement and explicit trust for each local workspace.

## Supported servers and tools

| Language | Server | Project-root markers |
| --- | --- | --- |
| Rust | `rust-analyzer` | nearest `Cargo.toml` |
| TypeScript and JavaScript | `typescript-language-server --stdio` | nearest `tsconfig.json`, `jsconfig.json`, or `package.json` |

The Agent can use four read-only tools:

| Tool | Result |
| --- | --- |
| `find_definition` | Workspace-relative definition locations and bounded previews |
| `find_references` | Deterministically sorted references, at most 50 returned |
| `get_hover` | Bounded type signature and documentation |
| `get_diagnostics` | Current or explicitly stale version-aware diagnostics |

These tools are available in normal and Plan Mode sessions when the current local workspace is eligible. Python, Go, Java, C, and C++ language servers are not supported by this foundation.

## Install a server

### Rust

Install `rust-analyzer` with the current stable Rust toolchain:

```bash
rustup component add rust-analyzer
rustup component add rust-src
rust-analyzer --version
```

See the upstream [rust-analyzer installation guide](https://rust-analyzer.github.io/book/rust_analyzer_binary.html) for platform-specific alternatives.

### TypeScript and JavaScript

Install the language server and TypeScript runtime with npm:

```bash
npm install -g typescript-language-server typescript
typescript-language-server --version
```

VaneHub AI supplies the required `--stdio` argument. See the upstream [TypeScript Language Server project](https://github.com/typescript-language-server/typescript-language-server#installing) for its current prerequisites.

## Enable LSP for a workspace

Use the desktop application for these steps:

1. Open **Settings > Agent Configurations** and find **Language server intelligence**.
2. Turn on **Enable LSP integration**, then enable Rust and/or TypeScript/JavaScript.
3. Select **Refresh discovery**. If the desktop process cannot see the executable, enter its absolute path in **Executable override**.
4. Keep initialization options as `{}` unless you need server-specific settings. The value must be a bounded JSON object.
5. Save the configuration.
6. Under **Test language server**, run the isolated test. Review the discovery, process start, initialization, and cleanup phases.
7. Read the trust disclosure, enter the absolute local workspace path under **Trusted workspaces**, and select **Trust workspace**.
8. Open a native API Agent session for that local workspace. The four LSP tools become eligible without requiring a code index.

Testing a server uses an isolated minimal project; it does not grant workspace trust. Enabling a Tree-sitter code index also does not grant LSP trust.

## Understand workspace trust

A language server is a local executable running with your operating-system account permissions. Workspace trust controls VaneHub AI's automatic activation and filters Agent-visible results to the current workspace, but it is not an operating-system sandbox. Trust only repositories and executable paths you understand.

Revoking trust rejects new requests and stops language-server processes owned by that workspace. Remote and SSH workspaces are not eligible for local LSP activation.

## Read lifecycle status

| State | Meaning |
| --- | --- |
| **Not running** | No active process exists; an eligible tool call may start one on demand. |
| **Starting** | The executable is being launched. |
| **Initializing** | LSP capabilities and document behavior are being negotiated. |
| **Ready** | Protocol requests are allowed; background project indexing may still continue. |
| **Waiting to restart** | An unexpected exit triggered bounded exponential backoff. |
| **Failed** | The restart budget was exhausted or a terminal safety failure occurred. |
| **Stopping** | VaneHub AI is performing protocol shutdown and process cleanup. |

A ready server with no requests or document leases is closed after ten minutes of inactivity. Configuration or trust changes drain affected processes before replacement. Desktop shutdown sends `shutdown`, then `exit`, and forcibly terminates a process only if bounded cleanup cannot complete.

## Choose `search_code` or LSP

| Need | Tree-sitter `search_code` | Live LSP tools |
| --- | --- | --- |
| Discover code by name, text, or meaning | Best fit | Requires an exact file position or diagnostic target |
| Compiler-aware definitions and references | No | Yes |
| Current types, hover docs, and diagnostics | No | Yes |
| Works without an external server process | Yes | No |
| Persists an index across sessions | Yes | No |
| Initial language coverage | Eight language families | Rust and TypeScript/JavaScript |

The capabilities are complementary. A successful Agent file edit invalidates an open LSP document and also queues targeted reconciliation when a code index is enabled.

## Limits and result states

- Disk content is authoritative; VaneHub AI does not maintain unsaved editor buffers.
- There is no filesystem watcher in this foundation. Agent writes invalidate exact paths immediately, while shell, Git, or external-editor changes are detected before the next semantic query.
- Definitions are capped at 20 and references at 50. Results preserve `total` and `truncated` metadata.
- Hover text, previews, diagnostics, protocol frames, queues, and complete tool output have hard limits.
- `ready` with an empty result means the server found nothing. `warming`, `timeout`, `unavailable`, `failed`, and stale diagnostics are distinct degraded states.
- Completion, rename, formatting, code actions, workspace edits, call/type hierarchy, and persistent LSP enrichment are not included.
- Portable process memory and indexed-file counts are not reported because LSP servers do not standardize them.

## Troubleshooting

### The executable is not discovered

Run the version command in a regular terminal. Desktop applications may receive a different `PATH` than interactive shells, especially when launched from an icon. Restart VaneHub AI after changing `PATH`, or configure an absolute executable override.

### The isolated server test fails

Use the failed phase to narrow the cause:

- **Discovery:** the executable is absent or the manual override is invalid.
- **Process start:** dependencies, permissions, or the executable itself prevented launch.
- **Initialization:** the server rejected the minimal project, returned malformed capabilities, or timed out.
- **Cleanup:** graceful shutdown failed and forced cleanup could not finish.

### The Agent does not receive LSP tools

Confirm that you are using the desktop runtime, the master and matching language switches are enabled, discovery is available, the current canonical local workspace is trusted, and the session file uses a supported language. A remote session or browser preview cannot activate native LSP tools.

### Results are warming, stale, or truncated

`warming` usually means the process is still starting or the project is indexing. A stale diagnostic belongs to an older disk document version; request diagnostics again after analysis catches up. A truncated result is valid but bounded—use a narrower symbol or inspect the reported `total`.

### The server repeatedly enters backoff or failed

Review the safe reason shown in **Runtime status**, fix the executable, project, or initialization options, then use the isolated test again. Revoking and re-granting trust stops old processes; it does not repair an invalid server installation.

## Related

- How the persistent index and live LSP divide the work → [Code indexing](code-indexing.md)
- The settings page holding the language-server toggles → [Tools and extensions](tooling.md#agent-configurations)
- The LSP protocol itself: layering and lifecycle, capability negotiation, the text synchronization model → [LSP technical architecture](../../../agent-infrastructure/lsp-architecture.md) (Simplified Chinese)
