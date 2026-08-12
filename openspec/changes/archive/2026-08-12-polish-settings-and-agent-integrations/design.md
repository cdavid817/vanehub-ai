## Context

See `proposal.md` for motivation. The workspace already has a shared theme token system and Agent icon component, but OnePiece falls through to a generic icon. Scheduled Tasks currently filters and validates only CLI launch kinds even though OnePiece has a native API runner. CLI Parameters intentionally excludes policy-governed flags, yet its independent TypeScript and Rust catalogs can drift. Agent Configuration supports four payload kinds and must add Gemini through the existing service/native boundary. The IM domain's automatic kebab-case enum serialization disagrees with its own stable `as_str()` ids for acronym-style variants.

## Goals / Non-Goals

**Goals:**

- Keep all UI changes token-based, accessible, responsive, and shared by desktop and Web modes.
- Reuse stable Agent ids for OnePiece and Gemini instead of display-name matching.
- Make Gemini profile behavior conform to the existing profile lifecycle, secret isolation, atomic projection, drift detection, discovery, and Web parity.
- Make parameter metadata and connector wire identities mechanically testable across language/runtime boundaries.

**Non-Goals:**

- Expose every diagnostic, resume, output-format, or one-shot prompt flag offered by each upstream CLI.
- Duplicate approval/sandbox controls from Agent Policies on the CLI Parameters page.
- Introduce old-id aliases or migrate unsupported legacy Gemini profile payloads.
- Add a new UI library, state manager, or direct Tauri invocation from React.

## Decisions

### Use a one-pixel semantic workspace overlay

The divider remains an absolutely positioned, pointer-transparent `bg-border` line inside the workspace frame. This produces the reference screenshot's separation without consuming layout height or introducing OS-specific colors. A thicker footer or shadow was rejected because it would compete with compact desktop content.

### Add OnePiece to the shared vector icon switch

OnePiece receives a simple inline vector mark in the existing icon component, so every current consumer picks it up automatically with `currentColor`, sizing classes, titles, and fallback behavior. A generated raster asset was rejected because the icon must remain crisp and theme-compatible at 14–20px.

### Give Scheduled Tasks an explicit execution-kind decision

Eligibility is `launch_kind == cli || agent_id == onepiece`; execution selects API mode for OnePiece and CLI mode otherwise. This avoids pretending OnePiece is a CLI in the registry and keeps unsupported API Agents rejected. Desktop validation and runner tests cover both branches; Web mock mirrors selection behavior.

### Treat the native editable catalog as the behavioral source of truth

The audit boundary is the parameters VaneHub persistently manages, not every transient upstream flag. Existing official CLI documentation and locally installed `--help` output are used to correct aliases, descriptions, and known values. Contract tests compare the TypeScript catalog with a checked-in native fixture/serialized catalog so drift is detected. Approval and sandbox flags stay filtered by the existing policy-governed classification.

### Model Gemini as an exclusive, endpoint-capable profile

Gemini profiles use `kind: gemini-cli`, `baseUrl`, `model`, `authStrategy` (`preserve-official` or `api-key`), and `advancedEnv`. API keys remain in the native credential store. The native adapter projects VaneHub-managed values to Gemini's supported global environment configuration and preserves unrelated entries; it inspects `~/.gemini/settings.json` and the global Gemini environment file for discovery and drift. The Web adapter simulates the same lifecycle without filesystem access. This matches Gemini CLI's documented `GEMINI_API_KEY`, `GOOGLE_GEMINI_BASE_URL`, `GEMINI_MODEL`, and user settings behavior. Treating Gemini as a generic OpenAI-compatible provider was rejected because Gemini CLI uses Google-specific configuration and authentication.

### Reorder existing Settings definitions without changing ids

The array order remains the sole navigation order and prioritizes recurring work over one-time setup. The exact sequence is Basic, Agent Configuration, Agent Policies, CLI Parameters, MCP, Skills, Personalization, Prompt Hooks, Expert Roles, CLI Management, Extensions, Plugin Integrations, IM, SSH Connections, Observability, Usage Statistics, and About. Lazy loaders, visited-page keep-alive behavior, route ids, and deep links remain unchanged.

Agent behavior and reusable capability pages stay near the top because users revisit them while tuning daily workflows. CLI installation management moves below customization because it is primarily a setup and repair destination. External integrations, remote access, diagnostics, and product information remain progressively lower-frequency destinations.

### Serialize connector ids explicitly

`DingTalk` and `WeCom` receive explicit serde names matching `ConnectorKind::as_str()`. Tests enumerate every connector and assert both serialization and deserialization. Expanding the frontend schema to accept `ding-talk` and `we-com` was rejected because those values contradict the established stable ids and would preserve a broken wire contract.

## Risks / Trade-offs

- [Upstream CLI flags and model aliases evolve] → Keep the managed surface curated, allow custom model text, record the audited upstream semantics in tests, and update both catalogs together.
- [Gemini global files contain unrelated user settings] → Parse before writing, mutate only managed keys, use the existing atomic write/backup path, and fail closed on malformed input.
- [A scheduled OnePiece run follows a different runtime path] → Select interaction mode explicitly and add runner-level success/failure tests rather than reusing a hard-coded CLI mode.
- [A broad Settings reorder disrupts muscle memory] → Preserve all labels, ids, icons, and deep links; change only ordering and lock it with one explicit test.

## Migration Plan

1. Ship additive Gemini payload support before presenting its navigation entry.
2. Existing profile tables accept the new tagged JSON payload without schema migration; no old Gemini payload exists to convert.
3. Apply the UI ordering, summary, icon, and divider changes after service parity is in place.
4. Rollback removes the Gemini entry and payload handlers; existing non-Gemini profiles and IM data remain unchanged.
