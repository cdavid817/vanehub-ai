## ADDED Requirements

### Requirement: Multi-connector session authorization verification
Desktop verification SHALL exercise connector-scoped session authorization through the rendered Tauri client and native persistence boundary without requiring live external credentials.

#### Scenario: Verify non-Feishu default denial
- **WHEN** the deterministic desktop layer injects a Telegram, DingTalk, WeCom, or personal WeChat direct message for a session without matching enabled access
- **THEN** it SHALL observe no Agent execution and a safe disabled outcome

#### Scenario: Verify selected connector isolation
- **WHEN** the layer enables one connector for a session while another connector remains disabled
- **THEN** pairing and inbound delivery SHALL succeed only for the enabled connector

#### Scenario: Verify persisted connector choice
- **WHEN** the layer selects and enables a non-Feishu connector and relaunches the desktop client
- **THEN** the information panel SHALL restore that connector's native persisted access state
- **AND** the layer SHALL NOT use browser storage as persistence evidence
