# VaneHub AI Developer Guide

This guide is the curated entry point for contributors working on VaneHub AI. It explains ownership and integration boundaries; source code, OpenSpec main specifications, and generated Rustdoc remain the authoritative detail.

Use this guide when you need to answer:

- Where should a frontend or native change live?
- Which runtime behavior is real on desktop and which is simulated in Web preview?
- Which bounded context owns data, processes, and logs?
- How are changes specified, verified, packaged, and released?

The [native API reference](native-api-reference.md) is generated from Rust `//!` and `///` documentation. It is intentionally separate from this narrative guide.

## Documentation status

This guide documents the `main` branch architecture. A feature is not considered user-delivered merely because a service or native command exists; a user-visible path and its verification evidence must also exist.
