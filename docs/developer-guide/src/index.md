# VaneHub AI Developer Guide

This guide is the curated entry point for contributors working on VaneHub AI. It explains ownership and integration boundaries; source code, OpenSpec main specifications, and generated Rustdoc remain the authoritative detail.

Use this guide when you need to answer:

- Where should a frontend or native change live?
- Which runtime behavior is real on desktop and which is simulated in Web preview?
- Which bounded context owns data, processes, and logs?
- How are changes specified, verified, packaged, and released?

## Chapters

| Chapter | What it covers |
| --- | --- |
| [Repository orientation](repository-orientation.md) | Where frontend, native, and specification work lives |
| [Runtime and service boundaries](runtime-boundaries.md) | The service layer, and which behavior is real on desktop |
| [Native bounded contexts](native-contexts.md) | What each Rust context owns |
| [Persistence and unified logging](persistence-and-logging.md) | SQLite, migrations, and the redaction rules |
| [Testing, packaging, and release](testing-and-release.md) | Gates, coverage thresholds, and packaging targets |
| [OpenSpec workflow](openspec-workflow.md) | Proposing, applying, and archiving a change |
| [Native API reference](native-api-reference.md) | Generated from Rust `//!` and `///` documentation |

The reference chapter is generated, and is intentionally separate from this narrative guide.

## Other documents in this repository

These live outside the guide's chapter list but are part of the repository's documentation.

| Document | What it covers |
| --- | --- |
| [CLI Agent global configuration](../../cli-agent-global-configuration.md) | User-level provider profiles for Claude Code, OpenCode, and Codex CLI, and why saving one never changes the active Agent or Session |
| [Native build performance](../../build-performance.md) | Platform linker requirements, release-profile behavior, and measured build evidence |
| [Release signing](../../release-signing.md) | The signing and verification chain for published artifacts |

### Point-in-time surveys

**These are snapshots, not maintained narrative.** They describe the system as of the revision they name, and their `文件:行号` references are anchored to that revision — which is where they are most likely to have drifted. Read them for how a subsystem was shaped, and treat the chapters above and the specs as current.

| Document | Written against |
| --- | --- |
| [VaneHub AI 技术架构深度解析](../../VaneHub-AI-技术架构深度解析.md) (Simplified Chinese) | Commit `bb3d28d8`, 2026-08 |

## Documentation status

This guide documents the `main` branch architecture. A feature is not considered user-delivered merely because a service or native command exists; a user-visible path and its verification evidence must also exist.
