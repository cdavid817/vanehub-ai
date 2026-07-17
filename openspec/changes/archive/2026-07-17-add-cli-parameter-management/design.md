## Context

VaneHub manages four stable CLI agent ids (`claude-code`, `codex-cli`, `gemini-cli`, and `opencode`) but currently constructs their process arguments inside native provider-specific branches. The settings center has no way to inspect or persist launch defaults, the generic application `settings` table only stores scalar application preferences, and the chat composer already submits model, permission, and reasoning configuration that the native `send_message` path does not yet apply.

The change crosses React settings UI, the frontend agent service, Tauri and Web/mock adapters, SQLite, native command validation, interactive launch, and session-scoped chat execution. It must preserve provider-required headless output, resume, prompt-delivery, and session arguments. It must also avoid shell command construction, keep sensitive values out of logs, and remain usable in both visual themes and both supported languages.

Clowder AI demonstrates two relevant patterns: provider-owned conversion of structured settings such as reasoning effort, and generic extra argument lists. This design adopts the structured provider-owned pattern but does not copy free-form argument splitting because that requires fragile quoting, deduplication, and reserved-argument filtering.

### Current and target flow

```text
Current
React chat config ───────────────┐
                                ├─> native send_message ignores config
Hard-coded provider arguments ──┘

Target
CLI parameter page ─> AgentService ─> Tauri adapter ─> SQLite selections
                                  └─> Web adapter ────> localStorage selections

Chat config ───────────────────────────────┐
Persisted defaults ────────────────────────┼─> provider argument builder ─> Command::args
Provider/VaneHub required arguments ───────┘
```

## Goals / Non-Goals

**Goals:**

- Provide a dedicated settings page for typed, documented launch defaults for all four managed CLIs.
- Use enum dropdowns for parameters with one selected value, multi-select controls only for catalog entries that are explicitly repeatable, and switches for presence/absence flags.
- Persist desktop selections in a dedicated SQLite table and provide equivalent Web/mock persistence through the same service contract.
- Apply saved values to the next interactive launch and the next chat-runtime process, including resume invocations, without mutating a process already running.
- Make per-message chat configuration override the same persisted logical default when a provider mapping exists.
- Keep VaneHub protocol arguments and dangerous one-shot bypass flags outside user control.
- Provide localized names, detailed descriptions, option descriptions, validation feedback, save/reset states, and a safe argument preview.
- Record the intentional first-version limitations and concrete expansion seams.

**Non-Goals:**

- Parsing arbitrary user-authored command strings or supporting unrestricted raw arguments.
- Dynamically scraping `--help`, probing every installed CLI feature, or changing the catalog based on a CLI version in the first version.
- Persisting API keys, tokens, prompts, system prompts, or other secret-bearing values as CLI parameters.
- Restarting an active CLI child process when settings are saved.
- Supporting project-, workspace-, session-, or named-profile parameter scopes in the first version; selections are global per managed CLI.
- Replacing the existing CLI installation/version management page.
- Guaranteeing that every vendor CLI flag is exposed.

## Decisions

### 1. Add a separate settings page with explicit per-CLI commits

The settings navigation gains a `cli-parameters` page immediately after CLI Management. The page uses four stable-id tabs or compact selectors in the fixed order Claude Code, Codex CLI, Gemini CLI, and OpenCode. Each parameter row shows a localized name, literal flag, detailed localized description, control, and selected-value explanation.

Changes remain local draft state until the user activates Save for the selected CLI. Restore Defaults deletes the saved overrides for that CLI after confirmation. Explicit commits make validation failures and grouped changes visible and avoid partially persisted profiles.

Alternative considered: save on every toggle. This is simpler but creates many writes, makes grouped conflict validation harder, and provides no reliable cancel/dirty state.

### 2. Extend `AgentService` with typed parameter profiles

CLI parameter behavior belongs with agent launch behavior rather than generic application preferences. Add typed contracts equivalent to:

```ts
type CliParameterControl = "enum" | "boolean" | "multi-enum";
type CliParameterValue = string | boolean | string[];

interface CliParameterDefinition {
  id: string;
  agentId: string;
  flag: string;
  control: CliParameterControl;
  labelKey: string;
  descriptionKey: string;
  options?: Array<{ value: string; labelKey: string; descriptionKey: string }>;
  defaultValue: CliParameterValue;
  launchScopes: Array<"interactive" | "chat">;
  risk: "normal" | "warning";
}

interface CliParameterProfile {
  agentId: string;
  definitions: CliParameterDefinition[];
  selections: Record<string, CliParameterValue>;
  previewArgs: string[];
}
```

The service exposes bounded list, save, and reset methods. React components only call this service. `tauri-agent-client.ts` owns `invoke()` calls, while `web-agent-client.ts` provides contract-equivalent mock behavior.

Alternative considered: add the profiles to `AppSettings`. That would force structured provider data through the existing scalar setting validator and couple CLI launch semantics to unrelated theme/language preferences.

### 3. Use a dedicated normalized SQLite table

Add one additive migration after the current schema version:

```sql
CREATE TABLE cli_parameter_settings (
  agent_id TEXT NOT NULL,
  parameter_id TEXT NOT NULL,
  enabled INTEGER NOT NULL DEFAULT 1,
  value_json TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  PRIMARY KEY (agent_id, parameter_id),
  FOREIGN KEY (agent_id) REFERENCES agents(id) ON DELETE CASCADE
);
```

Saving a profile validates the complete selection against the native catalog and writes it in one transaction. Reset deletes rows for one `agent_id`. Loading ignores no data silently: an unknown or invalid legacy row falls back to the catalog default and emits a redacted warning with agent id and parameter id.

Alternative considered: one JSON blob per CLI in the generic `settings` table. A blob is easier initially but makes individual validation, migrations, diagnostics, and future scoped overrides harder.

### 4. Keep the first-version catalog curated and backend-authoritative

The native layer owns the allowed logical parameters, option values, scope, risk metadata, and argv conversion. The frontend receives definitions rather than hard-coding provider branches. Translation keys resolve through `zh-CN` and `en` resources. The Web/mock catalog mirrors the native contract and contract tests assert stable ids, control kinds, and defaults remain aligned.

The first-version catalog targets common, non-secret runtime controls:

| CLI | Initial logical controls | First-version treatment |
|---|---|---|
| Claude Code | model, effort, permission mode | curated dropdowns; bypass-permissions values excluded |
| Codex CLI | model, sandbox, approval policy, ephemeral, strict config | curated dropdowns/switches; `danger-full-access` and combined bypass flag excluded |
| Gemini CLI | model, approval mode, sandbox | dropdown/switch; existing YOLO default remains representable and is marked warning to avoid a silent behavior change |
| OpenCode | agent, thinking visibility, automatic approval | curated dropdown/switches; dynamic `provider/model` values deferred |

The implementation may narrow a listed value when the installed CLI contract cannot safely support it in both fresh and resume paths, but each of the four CLIs must retain at least one enum and one boolean or equivalent presence control in the delivered catalog. Truly repeatable parameters use `multi-enum`; arbitrary repeatable strings remain deferred.

Alternative considered: dynamically parse `--help`. Help output is not a stable machine-readable schema, is localized/versioned, and cannot supply VaneHub's translated descriptions, conflict rules, or security classification.

Implementation note (2026-07-17): the first-version OpenCode catalog narrows the planned model control to the documented `--agent` choices plus `--thinking` and `--auto`. OpenCode model ids require a live `provider/model` source and would become stale if hard-coded, so model selection remains in the deferred dynamic-value-source optimization. The other three catalogs use only flags verified against the locally installed Claude Code 2.1.126, Codex CLI 0.144.5, Gemini CLI 0.50.0 help contracts; OpenCode flags were verified against its official CLI reference.

### 5. Compose arguments by logical setting, not generic append

Provider builders accept persisted selections and optional per-message chat configuration, then place arguments at the location required by the provider grammar. They never join values into a shell command. Values are passed as distinct `Command::arg` or `Command::args` entries; Codex TOML configuration values use a dedicated serializer.

The precedence order is:

```text
per-message mapped ChatConfig
    > persisted CLI profile
    > VaneHub/provider default
```

VaneHub-required arguments remain authoritative regardless of this order. At minimum these include provider subcommands, structured output flags, prompt transport, resume/session identifiers, and stdin markers. A selected logical value that conflicts with a required argument is rejected by catalog validation rather than deduplicated after the fact.

For existing chat selectors, only explicit provider mappings are applied. Unsupported combinations are rejected or omitted with a concise reason; they are never guessed from display names. Model, reasoning, and permission mappings are keyed by stable agent id and covered by provider-specific tests.

Alternative considered: append user arguments after the hard-coded list and let the vendor decide precedence. This breaks positional grammars such as Codex subcommands and OpenCode prompts and allows output/session flags to be overridden.

### 6. Apply changes only to new child processes

Saving settings does not restart or signal an already running child. Interactive launch reads the current profile immediately before spawn. Chat generation reads the current profile and current message configuration immediately before each provider CLI spawn, including a resume spawn for an existing VaneHub session.

This gives "next launch" deterministic semantics: a saved value can affect the next message in an existing VaneHub session because that message creates a new provider process, but it cannot alter a process already streaming.

### 7. Show a safe preview and keep diagnostics redacted

The profile response contains the effective user-controlled argument segment after validation and provider conversion. The page shows this as separate escaped tokens, not as an executable shell command. It omits VaneHub prompt/session values and never contains secrets because secret-bearing definitions and free text are out of scope.

Save/reset failures and provider rejection diagnostics use the unified logging service. Command audits continue to redact prompt and sensitive patterns before persistence. The UI shows concise errors without raw stderr.

### 8. Preserve Web/mock parity, i18n, and both themes

The Web adapter persists profiles in a namespaced localStorage entry and uses the same normalization semantics without claiming it can launch a local process. It returns preview data so browser testing exercises the full page.

All user-visible copy, including parameter and value explanations, save/reset states, warnings, empty states, and errors, uses matching `zh-CN` and `en` keys. Literal provider names, flags, and stable ids remain untranslated. The page uses existing semantic tokens and shared controls; there are no page-specific theme branches or inline styles. Visual QA covers both `futuristic` and `minimal` at desktop and narrow widths.

### 9. Short-term implementation layout

The first implementation should stay additive while moving new native code toward the documented project layout:

- Frontend contracts/types: add CLI parameter types alongside agent contracts.
- Service boundary: add list/save/reset methods to `AgentService` and both runtime adapters.
- Settings UI: add one page plus small parameter-row and CLI-selector components so no file exceeds 300 lines.
- Native commands: add a CLI-parameter command module rather than putting new commands directly in a React component or generic settings handler.
- Native storage: add an additive migration/repository for `cli_parameter_settings`.
- Runtime: refactor only the argument-construction seam needed to accept typed profiles and per-message mappings; avoid a broad unrelated rewrite of the existing native monolith.
- Tests: catalog/normalization unit tests, service adapter tests, page tests, persistence/migration tests, provider argv tests, resume/fresh-launch tests, precedence tests, security tests, and Playwright theme/locale checks.

### 10. Deferred optimization and extension points

The following are explicitly documented for later changes and are not first-version acceptance criteria:

1. **Catalog version adaptation:** associate definitions with detected CLI version ranges and hide or disable unsupported values before launch.
2. **Runtime capability discovery:** combine curated metadata with machine-readable provider schemas if vendors expose them; do not scrape human help text as the only source.
3. **Dynamic value sources:** query installed models, profiles, agents, extensions, MCP servers, and Codex config profiles instead of relying on maintained dropdown options.
4. **Scoped profiles:** add global, project, workspace, session, and named profile layers with explicit precedence and import/export.
5. **Advanced custom arguments:** offer an expert mode only after defining a tokenizer, reserved-argument policy, secret classification, audit redaction, conflict resolution, and recovery UI.
6. **Catalog single-source generation:** generate Rust and TypeScript catalog representations from a validated schema rather than maintaining Web/mock parity by tests.
7. **Live compatibility feedback:** validate selections against the detected executable version and show unsupported/deprecated states before saving.
8. **Richer previews:** show interactive and chat/resume previews separately and explain which layer supplied each effective value.
9. **Provider builder extraction:** move all four builders and parser contracts out of the current native monolith into provider modules once the parameter seam is stable.
10. **Configuration history:** add revision history, diff, rollback, and audit attribution if profile administration becomes multi-user or remotely managed.

### 11. First-version implementation record

The delivered short-term implementation deliberately keeps the integration additive:

- Rust owns the authoritative desktop SQLite rows, validation, logging, command registration, and launch-time argument composition. The Web adapter uses a versioned localStorage key only to preserve browser-preview parity.
- The TypeScript and Rust catalogs are maintained separately in this version and are guarded by catalog/contract tests. Generating both from one schema remains the preferred follow-up once the catalog stabilizes.
- Chat settings override persisted values only in memory for the child process being created. The stored profile is not rewritten; a process already streaming keeps its original argv, while the next fresh or resume process reloads current rows.
- Model ids from the existing chat UI are mapped through an explicit stable-id table. Unknown ids are omitted rather than forwarded as arbitrary text. Claude aliases, Codex model ids, and Gemini model ids are included; OpenCode dynamic `provider/model` discovery is deferred and its first version exposes the documented Agent selector instead.
- Codex reasoning is serialized only from a closed enum to the distinct token `model_reasoning_effort=\"...\"`; the UI's `max` depth maps to the vendor's `xhigh` value. No free-form TOML or raw argument field is available.
- Gemini retains the pre-existing effective YOLO behavior through a warning-marked catalog default. Changing it requires an explicit saved value or a supported per-message permission mapping.
- The settings page remains below the 300-line project limit, uses shared page parts and semantic theme tokens, and includes a generic `multi-enum` renderer for future repeatable parameters.

Near-term follow-up priority is: version-aware catalog compatibility, dynamic model/Agent value discovery, schema-generated cross-runtime catalogs, then project/session-scoped profiles. Raw custom arguments remain intentionally later because their tokenizer, reserved-argument, secret-redaction, and recovery contracts must be designed together.

### 12. First-version verification record

Verification completed on 2026-07-17:

- `npm run test`: 22 files and 69 tests passed.
- `npm run build`: TypeScript and Vite production build passed; the existing bundle-size advisory remains.
- Playwright Web/mock coverage: 2 scenarios passed, covering profile draft switching, save, reload restore, reset confirmation, validation-visible storage errors, preview updates, keyboard/switch semantics, desktop futuristic Chinese layout, and narrow minimal English layout.
- `cargo test --manifest-path src-tauri/Cargo.toml`: 87 tests passed before integration, including the additive CLI parameter migration, repository atomicity, catalog safety, catalog-ordered multi-select normalization, all provider fresh/resume shapes, interactive snapshot reload, availability isolation, ChatConfig precedence, and next-process semantics. During integration, the local-extension and usage-monitoring migrations retained versions 10 and 11 respectively, while the CLI parameter migration was reassigned to version 12 to preserve all migrations in order and remain compatible with databases that already applied the remote usage migration.
- `cargo check --manifest-path src-tauri/Cargo.toml`: passed.
- `cargo clippy --manifest-path src-tauri/Cargo.toml`: passed with two pre-existing warnings in the native monolith (`type_complexity` and `too_many_arguments`); `-D warnings` therefore remains a repository-wide cleanup rather than part of this feature.
- `rustfmt --check` passed for the changed Rust files. The repository-wide `cargo fmt --check` still reports pre-existing formatting differences in unrelated logging, MCP, SDK, and Skill modules; those files were deliberately not reformatted in this change.
- The repository has no `lint` npm script or ESLint configuration, so `npm run lint` cannot run until a separate repository-level lint decision is made. TypeScript strict checking is covered by `npm run build`.
- Both strict OpenSpec validation commands passed.

No real vendor CLI process was launched during automated verification because that could authenticate, mutate a workspace, or consume provider quota. Desktop behavior is covered proportionally by native SQLite/repository tests and provider argv tests for interactive, fresh chat, and resume paths; a signed-in manual smoke remains appropriate before release packaging.

## Risks / Trade-offs

- **[Vendor CLI flags change across versions]** → Keep a small curated catalog, fail with concise provider errors, test the supported baseline, and reserve version-aware definitions for a later change.
- **[Static model options become stale]** → Reuse one maintained option source in the short term and document dynamic model discovery as the first catalog optimization.
- **[Saved values break resume grammar]** → Test fresh and resumed argv independently for every provider and insert logical settings through provider-specific builders.
- **[Per-message and persisted values conflict]** → Use the documented precedence order and show the effective preview; do not deduplicate raw flags.
- **[Gemini approval behavior changes accidentally]** → Preserve the current effective default, expose it with warning metadata, and require explicit save to change it.
- **[Dangerous settings weaken safety]** → Exclude explicit bypass flags and unsafe sandbox values; mark approval-expanding choices as warnings and never store secrets.
- **[Web and native catalogs drift]** → Add adapter contract tests now and leave schema-driven generation as a planned optimization.
- **[Native monolith grows further]** → Put new commands/storage/catalog logic in modules and limit edits in `lib.rs` to integration seams.
- **[Argument previews leak data]** → Preview only catalog-controlled tokens and omit prompts, session ids, free text, and secret-bearing values.

## Migration Plan

1. Add the SQLite table through an additive migration; existing databases receive no profile rows and therefore preserve current provider defaults.
2. Add catalog, repository, and bounded list/save/reset commands before exposing the page.
3. Add service contracts and both adapters, then add the settings navigation/page and translations.
4. Refactor provider argument construction behind a typed selection input and preserve existing argv when no profile rows exist.
5. Connect mapped per-message chat settings and verify precedence for fresh and resume invocations.
6. Run focused tests, full project validation, and visual QA in both themes and languages.

Rollback removes the page and stops reading the new table; the additive table may remain unused without affecting older binaries. No destructive down migration is required.

## Open Questions

No blocking product question remains for the first version. Exact curated model/value lists must be checked against the supported CLI baselines during implementation, and any value that cannot be mapped safely in both fresh and resume flows must be excluded and recorded in the implementation notes rather than guessed.
