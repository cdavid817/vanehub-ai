## ADDED Requirements

### Requirement: Effective Skill configuration metadata
The effective Skill model SHALL expose whether the winning revision is configurable, its schema hash and revision, available scopes, required-configuration readiness, drift status, and redacted User/Project configuration summaries. Shadowed revisions MUST NOT determine the active configuration schema.

#### Scenario: Winning revision has a configuration schema
- **WHEN** Skill details are requested for a configurable effective revision
- **THEN** the response identifies the exact active schema and redacted readiness without returning stored secret values

#### Scenario: Higher-priority revision changes the schema
- **WHEN** a higher-priority Skill becomes the winning revision
- **THEN** configuration is re-evaluated against that winning revision rather than the shadowed schema

### Requirement: Skill lifecycle refreshes configuration readiness
Skill creation, import, enablement, replacement, scope precedence change, restore, archive, and deletion SHALL refresh configuration schema validation and effective readiness. An invalid configuration SHALL disable only configuration-dependent activation and MUST NOT corrupt the Skill package or unrelated Skills.

#### Scenario: Skill replacement introduces required configuration
- **WHEN** a replacement revision adds a required property with no valid effective value
- **THEN** the Skill remains manageable but configuration-dependent activation is unavailable until repaired

