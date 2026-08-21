# VaneHub AI Developer Guide

This guide is the curated entry point for contributors working on VaneHub AI. It explains ownership and integration boundaries; source code, OpenSpec main specifications, and generated Rustdoc remain the authoritative detail.

Use this guide when you need to answer:

- Where should a frontend or native change live?
- Which runtime behavior is real on desktop and which is simulated in Web preview?
- Which bounded context owns data, processes, and logs?
- How are changes specified, verified, packaged, and released?

## Read these three chapters first

If you're new to this codebase, read these in order before anything else — they determine whether every later chapter makes sense.

| Chapter | What it covers |
| --- | --- |
| [Repository orientation](repository-orientation.md) | Where frontend, native, and specification work lives |
| [Runtime and service boundaries](runtime-boundaries.md) | The service layer, and which behavior is real on desktop |
| [Native bounded contexts](native-contexts.md) | What each of the 21 contexts owns, and how they talk to each other |

## Agent execution

| Chapter | What it covers |
| --- | --- |
| [Agent lifecycle and provider runtime](agent-lifecycle.md) | Registered Agent edits, stable provider resolution, capability declarations |
| [OnePiece native Agent](onepiece-native-agent.md) | Built-in API Agent identity, Profile lifecycle, provider directory |
| [OnePiece built-in tools](onepiece-builtin-tools.md) | Release gates, dependencies, and rollback triggers for the extended native toolset |
| [CLI lifecycle and global configuration](cli-lifecycle.md) | CLI detection, conflict resolution, upgrade eligibility, and the constraints on rewriting each CLI's configuration file |
| [Terminal and PTY runtime](terminal-runtime.md) | Session-scoped Agent Terminal, auto-start/attach, remote terminals |
| [SSH connections and the remote runtime](ssh-connections.md) | Host-key trust, bounded-field validation, remote channel events, and connection-pool limits |
| [Tool registry and execution](tool-registry.md) | Fixed native tool catalog, per-interface_format translation, multi-turn tool loop |
| [Extended tool contexts](extended-tool-contexts.md) | Sandboxed execution, the browser, web research, and artifact storage, and each one's gates and isolation constraints |
| [CLI delegation and the ChangeSet pipeline](cli-delegation.md) | Isolated execution, ChangeSet capture and sealing, one-time exact apply, and the recovery capsule |
| [Multi-Agent group chat](multi-agent-group-chat.md) | Seat model, mid-session add/remove, turn routing, durable presence |
| [Loop runtime and session Plan mode](loop-and-plan-runtime.md) | Durable Loop execution plus the read-only Plan mode inside OnePiece sessions |
| [Goals and the work board](goals-and-work-board.md) | The goal state machine and derived acceptance, and the board's idempotent reconciliation |
| [Session recovery](session-recovery.md) | Recovery status orthogonal to lifecycle, durable execution identity and ownership |

## Context and retrieval

| Chapter | What it covers |
| --- | --- |
| [Context compaction](context-compaction.md) | The token-aware trigger and character fallback, summarization compaction, cooldown and circuit breaking |
| [Cross-session memory](cross-session-memory.md) | Host-level shared pool, provenance metadata, OnePiece tool vs CLI auto-extraction |
| [Retrieval and vector search](retrieval.md) | Host-level shared memory pool, workspace code index, graceful degradation |
| [Tree-sitter code indexing](tree-sitter-code-indexing.md) | Grammar parsing, bounded chunks, symbol metadata, grammar version, redaction |
| [LSP code intelligence](lsp-code-intelligence.md) | In-session LSP integration, workspace trust, and capability negotiation |

## Tools and extensions

| Chapter | What it covers |
| --- | --- |
| [Skill management](skill-management.md) | Dual scope, SKILL.md contract, drift, built-in seeding/reconciliation |
| [Effective Skill runtime](effective-skill-runtime.md) | How the runtime resolves multiple sources into one effective Skill set |
| [Skill Overlay governance](skill-overlay-governance.md) | Overlay priority, conflict handling, and governance rules |
| [Skill evolution evidence](skill-evolution-evidence.md) | Attribution rationale and eligibility tiers, signal classification, sanitization, and encrypted storage |
| [MCP tools and clients](mcp-tools.md) | Transport and configuration model, MCP tools in the native catalog |
| [IM connectors](im-connectors.md) | Five built-in connectors, first-version direct-message scope, inbound routing |

## Policy and observability

| Chapter | What it covers |
| --- | --- |
| [Permission model](permission-model.md) | Unified decision point, explicit-Deny-first, approval broker, CLI flag projection, Claude Code hook bridge |
| [Execution observability and Agent evaluation](execution-observability.md) | Runs/spans/timelines, the four fidelity tiers and sanitization caps, and the evaluation arena's judgment rules |
| [Persistence and unified logging](persistence-and-logging.md) | SQLite, migrations, and the redaction rules |
| [Usage statistics](usage-statistics.md) | Reported tokens vs. estimated characters, time ranges, per-Agent breakdown |

## Engineering process

| Chapter | What it covers |
| --- | --- |
| [OpenSpec workflow](openspec-workflow.md) | Proposing, applying, and archiving a change |
| [Testing, packaging, and release](testing-and-release.md) | Gates, coverage thresholds, and packaging targets |

## Reference

| Chapter | What it covers |
| --- | --- |
| [Native API reference](native-api-reference.md) | Generated from Rust `//!` and `///` documentation |

The reference chapter is generated, and is intentionally separate from this narrative guide.

## Other documents in this repository

These live outside the guide's chapter list but are part of the repository's documentation.

| Document | What it covers |
| --- | --- |
| [CLI Agent global configuration](../../cli-agent-global-configuration.md) | User-level provider profiles for all five CLI Agents, and why saving one never changes the active Agent or Session |
| [Built-in model provider catalog](../../model-providers.md) (Simplified Chinese) | Endpoint protocols, default models, and credential storage for 25 providers |
| [Agent infrastructure technical documentation](../../agent-infrastructure/README.md) (Simplified Chinese) | MCP, LSP, Function Calling, RAG, and other **protocols and technologies themselves** — not VaneHub AI's implementation of them |
| [Native build performance](../../build-performance.md) | Platform linker requirements, release-profile behavior, and measured build evidence |
| [Release signing](../../release-signing.md) | The signing and verification chain for published artifacts |

### Point-in-time surveys

**These are snapshots, not maintained narrative.** They describe the system as of the revision they name, and their `file:line` references are anchored to that revision — which is where they are most likely to have drifted. Read them for how a subsystem was shaped, and treat the chapters above and the specs as current.

| Document | Written against |
| --- | --- |
| [VaneHub AI 技术架构深度解析](../../VaneHub-AI-技术架构深度解析.md) (Simplified Chinese) | Commit `bb3d28d8`, 2026-08 |

## Documentation status

This guide documents the `main` branch architecture. A feature is not considered user-delivered merely because a service or native command exists; a user-visible path and its verification evidence must also exist.

The map in [Native bounded contexts](native-contexts.md) is enforced against `src-tauri/src/contexts/` by `npm run docs:links:check`: adding a context without adding its row to the map fails validation.
