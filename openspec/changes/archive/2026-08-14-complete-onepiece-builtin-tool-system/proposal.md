## Why

OnePiece already has a native multi-turn tool loop, but its built-in catalog is limited to workspace files, search, shell, memory, Skills, MCP, and conditional LSP access. It cannot yet complete common research and delivery workflows that require controlled browser automation, Web retrieval, isolated code execution, OCR, durable Artifact publication, or bounded delegation to an installed coding CLI, and adding those capabilities without a common registration and governance foundation would fragment permissions, lifecycle, logging, and runtime-adapter behavior.

## What Changes

- Introduce a fixed handler registry for native API-agent tools with stable provider schemas, explicit per-Agent eligibility, dispatch-time revalidation, centralized limits, cancellation, permission classification, safe result envelopes, and unified observability. Existing file and command tools move behind this registry without changing their established behavior.
- Register every new capability in this change only for the built-in Agent whose stable id is `onepiece`; ordinary custom API Agents and CLI-wrapped Agents do not receive Browser, Web, code-execution, OCR, Artifact-publication, or CLI-delegation tools.
- Add Playwright-backed browser automation for navigation, bounded page inspection, screenshots, JavaScript evaluation, content extraction, and explicit human handoff, using isolated browser sessions and approval-gated risky actions.
- Add DuckDuckGo-backed Web search plus guarded HTTP fetching, redirect and network-policy enforcement, bounded extraction, source metadata, and citation-ready results without granting arbitrary network access to other tools.
- Add a dedicated `code_execution` tool that runs supported code in an independently owned sandbox with hard CPU, memory, duration, process, filesystem, network, input, and output limits instead of routing generated code through the general shell tool.
- Add OCR consumption for approved image/PDF Artifact inputs through the managed local PaddleOCR capability, with readiness checks, bounded page/image processing, provenance, confidence metadata, and no implicit remote inference fallback.
- Add immutable, content-addressed Artifact creation, inspection, publication, download, retention, and provenance contracts. Artifact publication exposes reviewable outputs through application-owned references rather than arbitrary host paths or provider-authored URLs.
- Add bounded `delegate_cli` execution for OnePiece to delegate `analyze` or `edit` tasks to installed Claude Code or Codex CLI instances in independent temporary Git clones. Delegation uses CLI-owned authentication, strict structured protocols, explicit context snapshots, process-tree cancellation, mode-specific readiness, and immutable result/ChangeSet Artifacts.
- Add `apply_delegation_changes` as a separate once-only, exact-Artifact approval flow. It applies a complete reviewed ChangeSet only to the same clean repository baseline, never stashes, rebases, merges, commits, pushes, or partially applies changes, and uses verified rollback/recovery semantics.
- Expose native readiness, progress, approvals, results, Artifacts, delegation attempts, and recovery states through the shared frontend service boundary. Web/mock adapters preserve the contracts with deterministic simulation or an explicit desktop-runtime-required result and never claim that native effects occurred.
- Persist only bounded, redacted operational metadata through the unified logging and task/operation infrastructure; raw credentials, hidden reasoning, unrestricted browser content, full external CLI transcripts, and unbounded tool payloads are excluded.

## Capabilities

### New Capabilities

- `onepiece-tool-governance`: Fixed handler registration, OnePiece-only eligibility, dispatch revalidation, shared lifecycle, limits, permissions, cancellation, observability, and Web/runtime honesty for the new built-in tools.
- `onepiece-browser-automation`: Playwright navigation, inspection, screenshots, JavaScript execution, extraction, approvals, session isolation, and human handoff.
- `onepiece-web-research`: DuckDuckGo search and guarded Web fetching with bounded extraction, provenance, citations, and network safety controls.
- `onepiece-code-execution`: Dedicated isolated code-execution sandbox, supported runtimes, resource budgets, result capture, and cleanup.
- `onepiece-ocr-tool`: Local OCR requests over approved Artifact inputs with framework readiness, bounded processing, provenance, and structured results.
- `onepiece-artifact-publishing`: Immutable Artifact storage, safe inspection, application-owned publication, download, retention, and lineage.
- `onepiece-cli-delegation`: Claude Code/Codex CLI readiness, prompt and context isolation, delegated attempt lifecycle, structured protocol adapters, ChangeSet review, exact application, rollback, and compatibility governance.

### Modified Capabilities

- `agent-tool-execution`: Replace the monolithic native catalog/dispatch assumption with fixed handler registration and explicit per-Agent eligibility while preserving existing tool behavior and provider translation.
- `agent-chat-configuration`: Define how Plan/read-only execution policy filters the new OnePiece-only catalog and prevents mutating Browser, sandbox, Artifact, and delegation operations.
- `onepiece-native-agent`: Extend OnePiece capability/readiness metadata and safe defaults for the new first-party tools without exposing them to user-created API Agents.
- `local-extension-management`: Extend managed PaddleOCR from management/self-test readiness to an explicitly bounded local inference consumer used by the OnePiece OCR tool while retaining loopback ownership and no-remote-fallback guarantees.

## Impact

- **Desktop runtime:** New Rust application/domain/infrastructure boundaries for tool registration, browser ownership, guarded HTTP access, sandbox workers, Artifact storage, OCR inference, external CLI delegation, ChangeSet application, recovery, readiness, and SQLite persistence. New native dependencies are limited to reviewed libraries required by these adapters; Playwright and managed CLI/runtime dependencies remain explicitly detected and versioned.
- **Frontend/Web runtime:** Add service contracts, Tauri and Web/mock adapter parity, readiness and operation projections, browser/human-handoff controls, Artifact review/publication surfaces, and ChangeSet review/application UI. React components continue to depend only on `AgentService` and related service interfaces.
- **Security and permissions:** New actions/resources and risk classifications flow through the existing unified permission and approval engine. Tool availability never grants authority; every dispatch rechecks Agent identity, execution mode, workspace, readiness, limits, and current policy.
- **Persistence and logging:** Add bounded operation, Artifact, delegation-attempt, apply-attempt, and recovery records in SQLite. All durable diagnostics use unified logging with pre-persistence redaction and retain page-visible task output separately from logs.
- **Compatibility:** Full native execution remains desktop-only. Web/mock behavior is deterministic and contract-compatible but performs no filesystem, browser, network, OCR, sandbox, Artifact-publication, or external-CLI side effects.
