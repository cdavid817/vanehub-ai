## Why

Several frequently used desktop and Settings surfaces are visually ambiguous, incomplete for OnePiece and Gemini CLI, or inconsistent with the native contracts they present. The IM connector identity mismatch currently prevents the page from loading at all, while CLI parameter metadata and configuration coverage need an explicit, tested source of truth.

## What Changes

- Add a theme-aware bottom divider between the VaneHub workspace and the operating-system taskbar/window boundary.
- Give OnePiece a dedicated reusable brand icon everywhere Agent identity is rendered.
- Allow OnePiece to be selected and executed by Scheduled Tasks while retaining validation for unsupported Agents.
- Simplify CLI Management to one installed-count summary instead of duplicating installed and missing counts.
- Audit the user-editable parameter catalog for Claude Code, Codex CLI, OpenCode, Antigravity CLI, and Gemini CLI against current CLI help and official documentation; correct supported values, scopes, descriptions, previews, and frontend/native parity without exposing policy-governed controls twice.
- Add Gemini CLI to Agent Configuration with profile persistence, validation, application, discovery/import, desktop projection, and Web/mock parity.
- Reorder Settings navigation by expected usage frequency: basic and daily Agent behavior first, reusable tool/customization capabilities next, one-time CLI installation and external integrations after that, and diagnostics/product information last.
- Fix DingTalk and WeCom connector serialization to emit the stable frontend contract ids `dingtalk` and `wecom`, with contract regression coverage.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `main-layout-ui`: require a visible theme-aware divider at the bottom edge of the workspace.
- `visual-design-system`: require a dedicated OnePiece identity icon across Agent surfaces.
- `scheduled-task-management`: permit the native OnePiece Agent as a scheduled-task execution target.
- `settings-cli-management-ui`: reduce the CLI installation summary to one non-duplicative installed metric.
- `cli-parameter-management`: require audited, synchronized, accurately described user-editable CLI parameter metadata.
- `cli-agent-config-management`: add Gemini CLI as a fully supported global configuration profile kind.
- `settings-center-ui`: define a frequency-oriented Settings navigation order.
- `im-connector-management`: require stable serialized ids for DingTalk and WeCom across the native/frontend boundary.

## Impact

- Frontend: workspace shell, Agent icon component, Scheduled Tasks dialog, Settings navigation and pages, localization, service contracts, Web/mock state, and component/E2E tests.
- Native: scheduled-task Agent validation/execution, CLI configuration domain and live-file projection for Gemini CLI, CLI parameter metadata tests, and IM connector serialization.
- Runtime adapters: Tauri and Web implementations remain behind existing service interfaces and gain Gemini configuration parity; no React component calls Tauri directly.
- Data: the existing CLI configuration profile schema can store the new tagged payload; no legacy-data compatibility layer or migration is introduced.
- Dependencies: no new runtime package or UI component library is added.
