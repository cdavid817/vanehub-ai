# Settings Center Spec Compression Report

## Corpus Result

- Target: `openspec/specs/settings-center-ui/spec.md`
- Size: 775 lines, 55 requirements
- Default budget: 500 lines or 8,000 tokens
- Finding: no identical Requirement headings and no safe deletion candidates. The issue is mixed UI-domain ownership, not duplicate normative behavior.

## Recommended Split

Retain `settings-center-ui` for shared shell, navigation, styles, scrolling, common orchestration, and localization. Move domain-specific UI requirements into capability specs that own their corresponding domain:

| Target capability | Source requirement group | Rationale |
| --- | --- | --- |
| `settings-cli-management-ui` | CLI parameter, CLI management, status, version actions, feedback, cards, CLI localization | Cohesive management workflow; references agent and SDK domain specs without duplicating them. |
| `settings-basic-configuration-ui` | Basic configuration, proxy, log management | Cohesive application-preference workflow. |
| `settings-skill-management-ui` | Skills page, statistics, mount paths, scope, search, cards, dialogs, drift | UI projection of `skill-management`. |
| `settings-usage-statistics-ui` | Usage statistics and localization | UI projection of `usage-statistics`. |
| `settings-extension-management-ui` | Extension capabilities, lifecycle, visual style, localization | UI projection of `local-extension-management`. |
| `settings-im-management-ui` | IM page, routing, connector rows, credentials, authorization, visual style, localization | UI projection of `im-connector-management`. |
| `settings-floating-assistant-ui` | Floating assistant setting | UI projection of `desktop-floating-assistant`. |

The expected result is a shell spec below 250 lines and domain specs below 200 lines each. No SHALL, MUST, Requirement, or Scenario is removed.

## Duplicate Candidates

The following are related but not duplicates and must remain in separate layers:

- `settings-center-ui` CLI requirements and `agent-tool-registry` / `sdk-dependency-management`: the first defines UI behavior; the latter define domain contracts.
- Skills, usage, extensions, and IM settings requirements and their respective domain specs: each has a UI projection and a service/runtime contract.

## Review Gate

Approve this split only if the mapping and coverage report confirm every source Requirement and Scenario has one target. The follow-up implementation change must use MODIFIED and REMOVED delta operations for `settings-center-ui` and add the target capability specs.
