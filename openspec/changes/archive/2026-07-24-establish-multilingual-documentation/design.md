## Context

The repository currently has three root README files with matching high-level sections, six English Markdown documents under `docs/`, and no documentation build or publication pipeline. The frontend already enforces Simplified Chinese and English locale-key parity, but repository documentation has no equivalent source-of-truth or freshness contract.

The native crate is primarily an application binary/library boundary: `lib.rs` publicly exposes `run()`, while most domain, application, command, and infrastructure modules are crate-private. Only a small number of Rust source files currently contain `///` documentation. A default `cargo doc` build would therefore expose too little, while attempting to turn every internal item into a stable public API would distort the architecture.

Multi-Agent coordination now has persisted native and Web/mock service contracts, but the create-session UI still labels Multi-Agent selection as unavailable. Documentation must not convert that service-layer capability into a claim that users can already complete the workflow through the product UI.

## Goals / Non-Goals

**Goals:**

- Give the three README files a canonical source, stable shared structure, and automated factual-parity checks.
- Keep root README content concise and route readers to task-oriented user and developer guides.
- Build an English developer guide with mdBook and a complementary Rustdoc API reference from the stable Rust toolchain.
- Produce English and Simplified Chinese user guides with reproducible screenshots and workflow examples.
- Make documentation builds and checks repeatable locally and in CI without adding application runtime dependencies.
- Ensure documentation explicitly distinguishes desktop-native, Web/mock, delivered, preview, and planned behavior.

**Non-Goals:**

- Translating application UI resources or adding a new runtime locale.
- Implementing the Multi-Agent coordination UI.
- Publishing internal Rust modules as a supported external library API.
- Automatically accepting machine translation without review.
- Replacing OpenSpec requirements, architecture records, or source-code contracts with prose documentation.
- Requiring a public hosting destination or repository-settings change before the documentation can build.

## Decisions

### 1. Treat English README content as canonical and translations as reviewed derivatives

`README.md` remains the canonical source. `README.zh-CN.md` and `README.ja.md` keep the same stable section identifiers, code blocks, relative links, version facts, and delivered/planned classifications while translating narrative copy.

A repository script validates machine-checkable parity. Stable HTML comments identify equivalent sections so translated headings do not need to match textually. The check compares section order, fenced command blocks, repository-relative link targets, version references, and roadmap state. It does not attempt to judge translation quality automatically.

Stable technical facts use named HTML markers rather than prose-oriented regular expressions. The checker compares those markers across locales and validates manifest-owned facts, such as the project, React, and Tauri versions, against `package.json`.

AI-assisted translation is permitted as an authoring step, but translated files remain reviewable source files. CI reports drift and never overwrites them.

Alternative considered: generate translated README files on every build. Rejected because silent regeneration makes translation regressions difficult to review and depends on nondeterministic or credentialed services.

### 2. Separate project entry points, user guidance, and developer guidance

The root README files retain the product summary, supported capabilities, concise installation/quick-start path, architecture sketch, roadmap summary, and links to detailed guides. Long operational, architecture, contribution, and troubleshooting material moves into purpose-specific books.

The documentation source layout is:

```text
docs/
  developer-guide/
    book.toml
    src/
      SUMMARY.md
      ...
  user-guide/
    en/
      book.toml
      src/
        SUMMARY.md
        ...
    zh-CN/
      book.toml
      src/
        SUMMARY.md
        ...
    assets/
      screenshots/
  architecture/
  release-signing.md
```

Existing architecture documents are linked or included from the developer guide rather than copied. Shared source inclusion must not create files that are only valid from one working directory.

Alternative considered: one large multilingual mdBook. Rejected because a book has one language declaration and one navigation tree, making locale routing and parity harder to reason about.

### 3. Build narrative documentation with mdBook and API semantics with Rustdoc

The developer guide explains architecture, context ownership, service/adapter boundaries, native persistence, unified logging, testing, packaging, and contribution workflows. Rust code snippets use mdBook includes where practical so examples remain tied to source.

Rust API reference is generated with the stable toolchain using:

```text
cargo doc --manifest-path src-tauri/Cargo.toml --no-deps --document-private-items
```

Focused `//!` and `///` documentation is added to the crate entry, published context facades, important domain contracts, application ports, and Tauri DTO/mapping boundaries. Visibility is not widened merely to improve generated documentation.

The assembled documentation site places the mdBook developer guide and Rustdoc output under stable sibling paths and links between them. mdBook does not parse or duplicate Rustdoc item semantics.

Alternative considered: create a custom mdBook preprocessor that converts `///` comments into Markdown chapters. Rejected because it would duplicate Rustdoc parsing, linking, type rendering, and visibility behavior.

### 4. Use one repository-owned build entry point

Repository scripts expose consistent commands for:

- checking README parity and documentation links;
- building both user-guide locales and the developer guide;
- running `mdbook test` for supported Rust snippets;
- building Rustdoc with warnings treated as failures;
- assembling a single output tree suitable for a CI artifact or later static hosting;
- capturing or checking documentation screenshots.

The mdBook version and any additional documentation-only tools are pinned in CI. They are installed as build tooling and are not added to frontend or native runtime dependencies. Generated site output remains ignored; authored Markdown and approved screenshots remain versioned.

### 5. Treat screenshots as generated evidence with deterministic inputs

Playwright captures documentation screenshots from the Web/mock runtime using fixed fixtures, viewport, language, theme, and reduced-motion settings. Dynamic timestamps, generated ids, local paths, and machine-specific information are replaced by deterministic fixtures or masked.

Committed screenshots are generated by named scenarios. A check mode captures into a temporary location and compares the expected inventory and approved snapshots. Each screenshot has localized alt text and must not contain credentials, tokens, personal paths, or raw logs.

The screenshot runner allocates an available loopback port for each invocation and passes it explicitly to Playwright and Vite. It never reuses an existing server, preventing Windows excluded-port failures and accidental capture from another worktree.

Desktop-only behavior that cannot be reproduced in Web/mock is documented with an explicit desktop label and either a separately reviewed desktop capture or no screenshot. Web/mock captures must not imply that native processes, SQLite writes, or operating-system actions occurred.

Alternative considered: manually maintained screenshots without scenarios. Rejected because they become stale without a reproducible relationship to product state.

### 6. Gate the Multi-Agent user workflow on a user-visible path

The representative workflow models a dependency graph such as:

```text
plan -> frontend ----\
      -> native ------> test -> review
```

It explains primary and fallback Agent selection, prerequisites, output propagation, progress, cancellation, and result review using stable Agent ids where configuration is shown.

The workflow may be labeled as a normal delivered task only when a Playwright scenario can start, observe, and complete or cancel it through user-visible controls. Until then, documentation may describe the underlying coordination contract only in the developer guide or mark a Web/mock/API demonstration as preview; it must not provide fictitious UI steps or screenshots.

### 7. Build artifacts in CI before considering external deployment

CI builds and validates the complete documentation tree and uploads it as an artifact. The output layout is compatible with later GitHub Pages or other static hosting, but enabling a public deployment, custom domain, or repository setting is deferred until explicitly authorized.

This keeps the change self-contained and verifiable without assuming external publication permissions.

The documentation job caches Cargo registries, the pinned mdBook binary, and Rust documentation build intermediates. The Rustdoc output directory itself is cleared before assembly so cache reuse cannot preserve stale reference pages.

## Risks / Trade-offs

- [Risk] Three README files can remain linguistically aligned while becoming factually stale. → Validate stable facts mechanically and require feature claims to link to implemented specs or tested guides.
- [Risk] `--document-private-items` produces an overwhelming Rustdoc tree. → Add documentation to selected architectural boundaries and treat Rustdoc as reference material, while mdBook remains the curated navigation layer.
- [Risk] Enabling missing-doc warnings crate-wide would create a very large unrelated cleanup. → Scope completeness checks to an explicit boundary inventory and expand it incrementally.
- [Risk] Screenshot comparisons can vary across operating systems and browser rendering. → Pin the CI browser/tool versions, generate authoritative assets on one CI platform, use fixed fonts and reduced motion, and compare inventory before pixel content.
- [Risk] English canonical ownership can delay translations. → CI reports translation drift clearly, while maintainers retain explicit review and may update all three files in one change.
- [Risk] User guides may overstate Web/mock behavior. → Require runtime and feature-state labels and back normal workflows with user-visible Playwright paths.
- [Risk] Existing `docs/architecture` links may break when assembled below a new base path. → Add a link checker that validates authored sources and the assembled output tree.

## Migration Plan

1. Add the documentation source directories, build configuration, pinned tool setup, and ignored generated-output directory.
2. Introduce stable README section markers and parity validation without materially changing product claims.
3. Refactor the three README files together and link them to the new guide entry points.
4. Create the developer-guide navigation, reuse existing architecture documents, add focused Rust documentation, and assemble Rustdoc beside mdBook.
5. Create English and Simplified Chinese user-guide foundations, deterministic screenshot scenarios, and truthful runtime/feature-state labels.
6. Add the Multi-Agent workflow as preview documentation unless the required user-visible coordination UI is available and tested during implementation.
7. Add local scripts and CI validation/artifact upload, then run the full project and strict OpenSpec validation suites.

Rollback removes the new documentation build job, scripts, book sources, and generated screenshot assets, then restores the previous README files. No database, runtime API, or user data migration is involved.

## Open Questions

- Public static hosting and its URL remain a follow-up decision; this change produces a deployable CI artifact only.
- Japanese user-guide translation remains a future extension; the first user-guide language set is English and Simplified Chinese.
