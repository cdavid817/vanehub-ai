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

## Documentation status

This guide documents the `main` branch architecture. A feature is not considered user-delivered merely because a service or native command exists; a user-visible path and its verification evidence must also exist.
