## Why

Several high-frequency management surfaces currently expose avoidable visual noise or incomplete extension paths, while CLI profiles and Skill drift recovery do not fully match the capabilities users expect from the managed tools. Addressing them together restores a coherent minimal-first experience and removes a persistent upgrade defect that makes successful Skill synchronization appear ineffective.

## What Changes

- Make the conversation transcript and attached composer one continuous surface, removing the nested rounded/shadowed frame that presents as a second chat box.
- Make `minimal` the first-use and failure-fallback visual style while preserving persisted user theme choices.
- Redesign the Agent Configuration create-profile dialog into a compact, clearly grouped provider-selection and configuration flow with accessible sticky actions and responsive layout.
- Repair Zhipu GLM provider icon resolution and add a clearly labeled custom endpoint/provider choice for providers absent from the preset catalog.
- Expand the safe editable parameter catalogs for all managed CLIs from current official command references and local help contracts, while continuing to exclude secrets, prompts, runtime-owned transport/session flags, and policy-owned approval or sandbox controls.
- Make Skill drift synchronization converge for legacy built-in Skill records and derived-cache snapshots, including `api-doc-generation`, `code-review`, `code-security-scan`, and `readme-generation`, and report any genuinely unrepaired item explicitly.
- Add desktop and Web/mock parity tests, focused migration regressions, localization coverage, and representative minimal/futuristic UI behavior tests.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `main-layout-ui`: require one visually continuous conversation/composer surface without nested chat framing.
- `app-settings`: make the minimal style the default only when no valid persisted choice can be restored.
- `cli-agent-config-management`: require a compact create-profile flow, reliable preset branding, and an extensible custom endpoint/provider entry.
- `cli-parameter-management`: broaden and source-audit the safe editable catalog for every managed CLI while preserving the existing reserved and policy-governed boundaries.
- `skill-management`: require drift synchronization to converge after legacy built-in cache or registry mismatches and return truthful per-item failures.
- `settings-skill-management-ui`: require the Skill page to refresh from the post-synchronization state and avoid continuing to present resolved drift as active.

## Impact

- Frontend: chat composer framing, theme defaults, Agent Configuration dialog/catalog/icon rendering, CLI parameter controls and locale resources, and Skill synchronization feedback.
- Desktop runtime: authoritative CLI parameter definitions/builders and Skill registry/cache reconciliation in the existing `tooling` bounded context, with SQLite compatibility preserved.
- Web runtime: matching provider-profile, CLI parameter, settings-default, and Skill synchronization behavior through existing adapters without claiming native filesystem or process work.
- Boundaries: React remains dependent on service interfaces; both Tauri and Web/mock adapters retain matching contracts. No new UI library, state manager, database, or logging path is introduced.
- External references: editable CLI definitions are audited against the official documentation for Claude Code, Codex CLI, Gemini CLI, OpenCode, and Antigravity CLI, with source URLs and review date recorded alongside the catalog contract.
