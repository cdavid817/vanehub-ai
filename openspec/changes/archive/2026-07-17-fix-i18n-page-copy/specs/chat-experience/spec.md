## ADDED Requirements

### Requirement: Localized chat interface text
The chat UI SHALL render user-visible chat labels, selectors, placeholders, role labels, and status text through synchronized zh-CN and en translation resources.

#### Scenario: Chat composer and message labels localized
- **WHEN** the chat surface renders in Simplified Chinese or English
- **THEN** composer placeholders, send/enhance/stop actions, loading labels, message status labels, role labels, thinking labels, scroll controls, and welcome messages SHALL use the active locale

#### Scenario: Chat configuration selectors localized
- **WHEN** chat provider, agent, model, mode, permission, reasoning, or configuration controls render user-visible labels or descriptions
- **THEN** frontend-owned labels, button titles, and descriptions SHALL use the active locale
- **AND** provider names, model names, and Agent display names MAY remain literal identifiers

#### Scenario: Chat timestamps localized
- **WHEN** chat messages display timestamps
- **THEN** timestamp formatting SHALL use the active application language rather than a fixed locale
