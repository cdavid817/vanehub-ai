## Why

VaneHub AI already maintains English, Simplified Chinese, and Japanese README files, but it has no enforceable synchronization contract, published developer guide, generated native API reference, or task-oriented user guide. As the runtime and Multi-Agent coordination capabilities grow, maintainers and users need documentation that remains aligned with implemented behavior instead of drifting into duplicated or aspirational copy.

## What Changes

- Establish English as the canonical README source while retaining reviewed Simplified Chinese and Japanese translations with matching structure, commands, links, version facts, and delivery status.
- Reframe the README files as concise project entry points and route installation, first-run, architecture, contribution, and workflow detail into purpose-specific guides.
- Add an English mdBook developer guide for architecture, service boundaries, native contexts, persistence, logging, testing, and release workflows.
- Generate a Rustdoc API reference from selected Rust documentation boundaries and publish it alongside the mdBook output without duplicating Rust API semantics in handwritten Markdown.
- Add English and Simplified Chinese user guides with deterministic screenshots and a representative Multi-Agent coding workflow.
- Require user-facing guides to distinguish delivered UI, Web/mock demonstrations, and planned behavior; the Multi-Agent creation workflow may be presented as a normal user workflow only after an implemented UI exposes the existing coordination service.
- Add repeatable local and CI checks for README parity, internal links, mdBook/Rustdoc builds, documentation code samples, and deterministic screenshot freshness.

## Capabilities

### New Capabilities

- `multilingual-readme`: Defines canonical-source ownership, supported README languages, structural and factual parity, translation review, and navigation into detailed documentation.
- `native-developer-documentation`: Defines the mdBook developer guide, Rustdoc API reference boundary, unified publishing layout, and documentation build validation.
- `user-guide-documentation`: Defines localized task-oriented guides, truthful feature-state labeling, representative Multi-Agent workflow coverage, and deterministic screenshot governance.

### Modified Capabilities

None. This change documents existing contracts and adds documentation governance and publishing automation without changing application runtime requirements.

## Impact

- Documentation: updates the three root README files and reorganizes material under a structured documentation source tree.
- Rust: adds focused crate, module, and item documentation to selected published application/API boundaries; it does not widen Rust visibility or change command behavior solely for documentation.
- Tooling and CI: adds pinned mdBook/Rustdoc build commands, documentation validation scripts, deterministic Playwright screenshot capture, and CI artifacts or publishing steps.
- Desktop and Web runtimes: no behavior change. Documentation covers both runtimes and explicitly labels Web/mock-only demonstrations and desktop-only capabilities.
- Architecture boundaries: React/Tauri adapter isolation, SQLite ownership, and unified logging contracts remain unchanged.
- Dependencies: introduces documentation build tooling only; no application runtime dependency is added.
