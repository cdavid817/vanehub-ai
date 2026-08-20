## Context

See `proposal.md` for motivation. The affected behavior crosses the workspace layout, common settings defaults, Agent profile management, CLI parameter profiles, and the native Skill reconciliation transaction. Existing service contracts already cover all required operations; the change must preserve React/Tauri isolation, Web/mock parity, SQLite compatibility, unified logging, and the no-new-UI-library constraint.

The CLI catalog is intentionally curated rather than a raw argv editor. The current catalog contains only a few entries and already reserves prompts, structured output, session identity, approvals, permissions, and sandbox posture for runtime or Agent Policy ownership. The expanded catalog must keep that safety boundary while using current official references as evidence.

The native Skill path supports both historical mutable sources and newer immutable revision-pinned system packages. Drift detection compares the source on disk with the registry witness, while synchronization currently begins from a pre-repair report. Upgrade states can therefore require both cache materialization and a post-repair observation before synchronization can be considered converged.

## Goals / Non-Goals

**Goals:**

- Preserve one visual owner for workspace chat framing and establish a compact minimal-first hierarchy for Agent profile creation.
- Reuse the provider preset/profile contracts for custom endpoints and deterministic icon fallback.
- Make CLI catalog breadth evidence-based, typed, localized, and identical in desktop and Web/mock modes.
- Make built-in Skill repair idempotent and prove convergence against realistic legacy state.

**Non-Goals:**

- Adding a UI framework, arbitrary raw CLI arguments, secrets to CLI parameter profiles, or new Agent Policy controls.
- Replacing valid saved user theme choices or removing the futuristic theme.
- Treating a custom endpoint as a reviewed preset or promising native provider access in Web/mock mode.
- Overwriting mutable user/imported Skill content to match a built-in package.

## Decisions

### 1. The workspace shell owns chat framing

`ChatTab` remains the single outer surface. The attached composer keeps one separator and internal input affordances, but loses any second outer card/shadow treatment. Runner controls become part of the same attached composer region. This avoids theme-specific branches and follows the existing design rule against nested cards.

Alternative considered: round the entire transcript and keep the current composer card. That retains two competing owners and does not address the reported mixed square/rounded appearance.

### 2. Theme default changes at the shared settings model

The default constant and corruption/load fallback become `minimal` in the shared settings contract and both runtime adapters. Hydration still applies a valid stored value before showing the formal surface. This changes first-use behavior without migrating or rewriting an explicit `futuristic` preference.

Alternative considered: force minimal at the React theme provider. That would override persisted state and create divergence between displayed settings and effective styling.

### 3. Provider selection and profile fields are separate dialog regions

The add flow uses a compact searchable provider list/selector, a selected-provider summary, and grouped Agent-specific fields. Shared dialog/button/input primitives and semantic Tailwind tokens provide layout and accessibility. Zhipu uses the existing reviewed asset map with alias normalization fixed at the provider identity component. A stable `custom` catalog item selects the existing custom-profile path instead of inventing a separate persistence model.

Alternative considered: add special Zhipu and custom-provider branches inside the page. Central alias/catalog handling is less fragile and keeps both Agent types and runtimes aligned.

### 4. CLI expansion uses an allowlisted evidence manifest

Each managed CLI receives a source manifest containing official reference URL, review date, and reviewed version or rolling-reference note. Definitions stay in the shared frontend/native catalog contract and use existing typed controls unless a small new safe control is required. Candidate flags are admitted only when they are documented, accepted in the provider's launch grammar, non-secret, non-policy-owned, and do not replace VaneHub-owned prompt/output/session tokens.

Codex evidence comes from official OpenAI documentation. Claude Code, Gemini CLI, OpenCode, and Antigravity evidence comes from their respective first-party documentation or repositories. Local `--help` output may detect version skew but cannot by itself justify a new editable flag.

Alternative considered: dynamically expose every `--help` flag. Installed versions differ, help text lacks ownership metadata, and blindly exposing transport or bypass flags would violate existing security boundaries.

### 5. Skill synchronization verifies the committed target state

For immutable built-ins, synchronization always resolves the current system package descriptor and materializes its revision-pinned cache before updating the registry witness. For legacy hash formats, comparison/adoption is explicit and one-way toward the current immutable witness. User and imported sources retain the current adoption behavior.

After filesystem repairs are staged and records are derived, the application computes a post-repair drift report from the target records/filesystem state and persists that report in the same repository transaction as the records. If a reliable post-repair observation cannot be produced, the item remains failed instead of being optimistically cleared. The returned result retains the original `resolvedFrom` report for auditability and exposes remaining failures through the refreshed overview.

Alternative considered: clear the banner only in React. That would conceal persistent backend drift and reappear after refresh or restart.

## Risks / Trade-offs

- [Official CLI references change after review] -> Record source and review date, keep contract fixtures, and require a deliberate catalog update rather than runtime scraping.
- [More CLI controls increase settings density] -> Group controls and keep compact descriptions, responsive layout, and safe preview tests.
- [Legacy Skill sources may contain intentional user edits] -> Restore only immutable built-in sources; mutable user/imported records are adopted or reported, never silently overwritten as system content.
- [Post-repair inspection can fail after filesystem work] -> Keep repair and registry persistence transactional where supported, report partial failures explicitly, and cover rollback/failure injection.
- [Provider icon assets vary between themes] -> Use a centralized alias map plus deterministic accessible fallback and asset-resolution tests.

## Migration Plan

1. Ship the minimal default without rewriting existing valid theme rows or Web storage.
2. Add catalog/profile changes additively; existing saved profiles and selections normalize through unchanged stable ids and defaults.
3. On the next Skill overview/synchronization, recognize legacy built-in witnesses and reconcile them to the current immutable system revision. No destructive database migration is required.
4. If rollback is required, UI/catalog additions can be reverted without deleting persisted settings. Registry records already reconciled to a valid immutable package remain readable by the previous model; user/imported Skill content is untouched.
