## ADDED Requirements

### Requirement: IM session source identification
The workspace session navigation SHALL identify sessions created from IM bindings without exposing external identity values.

#### Scenario: Render IM-owned session
- **WHEN** a session has IM source metadata
- **THEN** its session card SHALL show a compact localized source indicator for Feishu, Telegram, DingTalk, WeCom, or personal WeChat alongside the existing Agent identity

#### Scenario: Protect external identifiers
- **WHEN** the session card or session details render an IM-owned session
- **THEN** they SHALL NOT display the raw external chat id, external user id, credentials, or authorization tokens

#### Scenario: Render in both styles
- **WHEN** an IM session indicator renders in `futuristic` or `minimal`
- **THEN** it SHALL use semantic tokens and stable dimensions without resizing, overlapping, or obscuring existing session actions

### Requirement: IM session actions remain consistent
IM-owned sessions SHALL use the existing session selection, rename, pin, archive, restore, and delete interactions.

#### Scenario: Select IM-owned session
- **WHEN** the user selects an IM-owned session card
- **THEN** the workspace SHALL display its persisted transcript through the existing Agent service

#### Scenario: Delete IM-owned session
- **WHEN** the user confirms deletion of an IM-owned session
- **THEN** the existing deletion interaction SHALL complete and the UI SHALL not require a platform-specific deletion flow

