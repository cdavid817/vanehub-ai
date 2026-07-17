## Why

The 775-line `settings-center-ui` specification combines seven domain-specific settings workflows with shared settings-shell requirements, exceeding the repository's maintenance budget and obscuring ownership.

## What Changes

- Move complete domain-specific Requirement and Scenario blocks into seven focused settings UI capabilities.
- Retain shared settings shell, style, orchestration, and cross-domain localization requirements in `settings-center-ui`.
- Preserve all normative behavior through a source-to-target mapping; no requirement or scenario is deleted.

## Capabilities

### New Capabilities
- `settings-cli-management-ui`: CLI settings workflow UI.
- `settings-basic-configuration-ui`: Basic settings, proxy, and log-management UI.
- `settings-skill-management-ui`: Skill management settings UI.
- `settings-usage-statistics-ui`: Usage statistics settings UI.
- `settings-extension-management-ui`: Extension capability settings UI.
- `settings-im-management-ui`: IM connector settings UI.
- `settings-floating-assistant-ui`: Floating assistant settings UI.

### Modified Capabilities
- `settings-center-ui`: Retain only shared settings-shell requirements after moving focused domain UI blocks.

## Impact

- OpenSpec documentation only; no runtime, frontend, Tauri, API, or behavior changes.
