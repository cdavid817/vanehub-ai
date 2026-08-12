## ADDED Requirements

### Requirement: Stable IM connector wire identities
Native and frontend IM services SHALL exchange the stable connector ids `feishu`, `telegram`, `dingtalk`, `wecom`, and `weixin` for descriptors, configuration, health events, and command inputs.

#### Scenario: Serialize DingTalk and WeCom views
- **WHEN** the native service returns DingTalk or WeCom connector data
- **THEN** every nested connector kind SHALL serialize as `dingtalk` or `wecom` respectively
- **AND** the frontend contract SHALL parse the complete connector list without an invalid-value error

#### Scenario: Deserialize stable command ids
- **WHEN** the frontend submits `dingtalk` or `wecom` to an IM command
- **THEN** the native command boundary SHALL deserialize it to the matching connector kind

