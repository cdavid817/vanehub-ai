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
| [Single-Agent governance: the five control planes](single-agent-control-planes.md) | The analytical model unifying the five CLIs and OnePiece, the three execution paths, configuration effectivity rules |
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
| [Context compaction](context-compaction.md) | The token-aware trigger and character fallback, optimizer-first classification and low-cost reductions, on-demand structured summarization, post-verification and the compatibility fallback, cooldown and circuit breaking |
| [Cross-session memory](cross-session-memory.md) | The host-level shared pool, scope and audience, the candidate review lifecycle, the two read boundaries of injection versus recall, the four production paths |
| [Retrieval and vector search](retrieval.md) | Memory recall and workspace code search as two independent chains, sequential two-path RRF fusion, background reconciliation, degradation and logging boundaries |
| [Tree-sitter code indexing](tree-sitter-code-indexing.md) | The local and semantic pipelines, file admission, error-tolerant parsing, chunks with optional symbols, redaction, embedding confirmation |
| [LSP code intelligence](lsp-code-intelligence.md) | The registry-driven support matrix, process and protocol lifecycle, read-only tools, workspace trust and supply-chain limitations |

Tree-sitter code indexing and LSP solve different problems; their responsibilities compare as follows (details in each chapter):

| Dimension | Tree-sitter code indexing (`search_code`) | LSP code intelligence |
| --- | --- | --- |
| Main purpose | Text or semantic search over workspace code chunks | Definitions/references/types/hover/diagnostics at precise positions |
| External process dependency | No (parsers are built in) | Yes (third-party language-server child processes) |
| Persistent index | Yes (per-workspace manifests, chunks, FTS, optional vectors) | No (ephemeral processes, document leases, diagnostic caches) |
| Offline capability | Local mode is fully offline; only the semantic channel needs vector embeddings | Not applicable (no search; the server itself runs locally) |
| Cross-file semantics | Syntax-structure level, no type resolution | Compiler/language-server-grade cross-file semantics |
| Workspace requirements | Local workspace with per-workspace index enablement | Local workspace with explicit workspace trust |
| Security boundary | Unredacted chunks never enter the index/embedding/logs/results | Read-only tool catalog + workspace filtering; the server remains an unsandboxed third-party process |
| Degradation | Missing vectors fall back to FTS (local mode is not degraded); failures soften to "temporarily unavailable" | Per-method capability negotiation; failures soften to warming/timeout/unavailable |

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
| [Desktop release verification](../../desktop-release-verification.md) | The per-platform verification procedure a desktop release must pass before publication |
| [Runtime performance budgets](../../runtime-performance-budgets.md) | The declared runtime budgets and how a regression against them is reported |
| [CLI Agent global configuration](../../cli-agent-global-configuration.md) | How VaneHub AI writes each CLI's own global configuration, and how tests isolate it |

### Provider SDK

The provider SDK documents live under `docs/provider-sdk/` and are the contract a third-party provider plugin implements. `openspec/specs/provider-plugin-sdk` requires them to exist at that location.

| Document | What it covers |
| --- | --- |
| [Provider contract](../../provider-sdk/contract.md) | The interface a provider implements and the guarantees it must uphold |
| [Manifest](../../provider-sdk/manifest.md) | The manifest schema, its required fields, and version compatibility |
| [Example provider](../../provider-sdk/example-provider.md) | A test-only reference implementation walked end to end |
| [Conformance testing](../../provider-sdk/conformance-testing.md) | The conformance workflow a provider runs before submission |
| [Security rules](../../provider-sdk/security-rules.md) | The restrictions a provider plugin operates under |

### Point-in-time surveys

**These are snapshots, not maintained narrative.** They describe the system as of the revision they name, and their `file:line` references are anchored to that revision — which is where they are most likely to have drifted. Read them for how a subsystem was shaped, and treat the chapters above and the specs as current.

| Document | Written against |
| --- | --- |
| [VaneHub AI 技术架构深度解析](../../VaneHub-AI-技术架构深度解析.md) (Simplified Chinese) | Commit `bb3d28d8`, 2026-08 |

## Documentation status

This guide documents the `main` branch architecture. A feature is not considered user-delivered merely because a service or native command exists; a user-visible path and its verification evidence must also exist.

The map in [Native bounded contexts](native-contexts.md) is enforced against `src-tauri/src/contexts/` by `npm run docs:links:check`: adding a context without adding its row to the map fails validation.
