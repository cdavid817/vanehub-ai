# LSP code intelligence

> **Feature state:** Implemented for the Tauri desktop runtime and local workspaces. Web/mock mode provides deterministic settings and status previews only; it does not inspect files or launch a language server.

Language Server Protocol (LSP) integration lets the native API Agent ask a local language server for definitions, references, hover information, and current diagnostics. It is disabled by default and requires both language enablement and explicit trust for each local workspace.

## Supported servers and tools

| Language | Server | Default startup arguments | Project-root markers |
| --- | --- | --- | --- |
| Rust | `rust-analyzer` | none | nearest `Cargo.toml` |
| TypeScript and JavaScript | `typescript-language-server` | `--stdio` | nearest `tsconfig.json`, `jsconfig.json`, or `package.json` |
| Go | `gopls` | none | nearest `go.mod` |
| Python | `basedpyright-langserver`, else `pyright-langserver` | `--stdio` | nearest `pyproject.toml`, `setup.py`, `setup.cfg`, or `requirements.txt` |
| C and C++ | `clangd` | none | nearest `compile_commands.json`, or `build/compile_commands.json` |

C and C++ is the one language that will not fall back. Every other language treats the workspace root as its project root when no marker is found; `clangd` without a compilation database assumes default compiler flags and then answers definitions and diagnostics that are confidently wrong, so VaneHub AI reports the request as unavailable instead.

Python prefers `basedpyright-langserver` when both are installed. Installing the fork is a deliberate act in a way that installing the upstream server is not, and the discovery panel names the one it selected.

Which languages exist is decided by the desktop build, not by the settings page. The page renders one card per language the running build registers, so a language your build does not know about cannot be configured, and a language your build knows but cannot run on this operating system is shown as unsupported rather than as merely undetected.

The Agent can use nine read-only tools:

| Tool | Result | Every server offers it? |
| --- | --- | --- |
| `find_definition` | Workspace-relative definition locations and bounded previews | Yes, in practice |
| `find_references` | Deterministically sorted references, at most 50 returned | Yes, in practice |
| `get_hover` | Bounded type signature and documentation | Yes, in practice |
| `get_diagnostics` | Current or explicitly stale version-aware diagnostics | Always |
| `find_type_definition` | Where the *type* of the symbol is declared, at most 20 locations | No |
| `find_implementations` | Implementations of an interface, trait, or abstract member, at most 20 | No |
| `find_workspace_symbols` | Symbols matching a name across one project, at most 50 | No |
| `get_document_symbols` | A file's declarations, flattened, each naming what encloses it | No |
| `find_call_hierarchy` | Callers of, or calls made by, a function, at most 50 | No |

The last five are worth knowing about because a server may simply not offer them. The tool is still there and still answers — with an `unavailable` status rather than silence — because whether a server supports a method is discovered when it starts, not when the session begins. `gopls` and `rust-analyzer` offer all nine; older or smaller servers often stop at the first four. The runtime status card lists what the running server actually negotiated.

`find_workspace_symbols` takes a file path as well as a query. The path is not a filter: it says which project's index to search. A repository can hold several projects of the same language, and a language server indexes one of them at a time.

These tools are available in normal and Plan Mode sessions when the current local workspace is eligible. Java is not supported by this foundation.

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

VaneHub AI supplies `--stdio` as this server's default startup argument; you can replace it under **Startup arguments** if your installation needs something else. See the upstream [TypeScript Language Server project](https://github.com/typescript-language-server/typescript-language-server#installing) for its current prerequisites.

### Go

```bash
go install golang.org/x/tools/gopls@latest
gopls version
```

`gopls` installs into `$(go env GOPATH)/bin`, which is not always on the `PATH` a desktop application inherits. See the upstream [gopls installation guide](https://pkg.go.dev/golang.org/x/tools/gopls#section-readme).

### Python

```bash
npm install -g basedpyright   # or: npm install -g pyright
basedpyright-langserver --help
```

Either server works; VaneHub AI supplies `--stdio`. See [basedpyright](https://docs.basedpyright.com/) or [pyright](https://microsoft.github.io/pyright/#/installation) for their current prerequisites.

### C and C++

`clangd` ships with LLVM. Install it through your platform's package manager, then generate a compilation database for each project:

```bash
clangd --version
# CMake, in the project you want served:
cmake -S . -B build -DCMAKE_EXPORT_COMPILE_COMMANDS=ON
```

The generated `build/compile_commands.json` is what makes the project detectable. See the upstream [clangd installation guide](https://clangd.llvm.org/installation) for other build systems, including `bear` for Make-based projects.

## Enable LSP for a workspace

Use the desktop application for these steps:

1. Open **Settings > Agent Configurations** and find **Language server intelligence**.
2. Turn on **Enable LSP integration**, then enable Rust and/or TypeScript/JavaScript.
3. Select **Refresh discovery**. If the desktop process cannot see the executable, enter its absolute path in **Executable override**.
4. Leave **Startup arguments** blank to use the defaults in the table above. To pass your own, enter one argument per line — the list you enter replaces the defaults rather than adding to them, so a server that needs `--stdio` must still list it. An entered-but-empty list means "start this server with no arguments at all", which is not the same as leaving the field blank.
5. Keep initialization options as `{}` unless you need server-specific settings. The value must be a bounded JSON object.
6. Save the configuration.
7. Under **Test language server**, run the isolated test. Review the discovery, process start, initialization, and cleanup phases.
8. Read the trust disclosure, enter the absolute local workspace path under **Trusted workspaces**, and select **Trust workspace**.
9. Open a native API Agent session for that local workspace. The four LSP tools become eligible without requiring a code index.

Changing startup arguments changes the command line a server runs under, so any server already running for that configuration is drained and restarted before the next request. Testing a server uses an isolated minimal project; it does not grant workspace trust. Enabling a Tree-sitter code index also does not grant LSP trust.

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

### A C or C++ request reports a missing project marker

`clangd` is installed and discovery shows it as available, but the workspace has no compilation database, so there is nothing to serve the request with. Generate one — `cmake -DCMAKE_EXPORT_COMPILE_COMMANDS=ON`, `bear -- make`, or your build system's equivalent — and place it at the project root or in its `build` directory. This is deliberately distinct from an installation problem: the server is fine, the project is what cannot be read.

### A language is shown as unsupported on this operating system

That is different from an undiscovered executable. The running build registers the language but declares that its server does not run on this platform, so there is no executable to install and the enablement switch and isolated test are unavailable. An undiscovered executable, by contrast, reports **Unavailable** with a reason and can be fixed by installing the server or setting an override.

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
