# custom-model-display Specification

## Purpose
TBD - created by archiving change dynamic-llm-model-discovery. Update Purpose after archive.
## Requirements
### Requirement: Display known models with catalog labels
When a model ID matches an entry in `PROVIDER_MODELS`, the system SHALL display the catalog-defined friendly label.

#### Scenario: Known Claude model displayed
- **WHEN** the effective model ID is `claude-opus-4-8` and the provider is `anthropic`
- **THEN** the info panel and model selector SHALL display `Opus 4.8`

#### Scenario: Known Codex model displayed
- **WHEN** the effective model ID is `gpt-5-5` and the provider is `openai`
- **THEN** the info panel and model selector SHALL display `GPT-5.5`

#### Scenario: Known Gemini model displayed
- **WHEN** the effective model ID is `gemini-2-5-pro` and the provider is `google`
- **THEN** the info panel and model selector SHALL display `Gemini 2.5 Pro`

### Requirement: Display unknown models with normalized raw ID
When a model ID does not match any entry in `PROVIDER_MODELS`, the system SHALL normalize and display the raw model ID rather than silently falling back to the default model label.

#### Scenario: Custom model from native config
- **WHEN** the effective model ID is `deepseek-chat` and no catalog entry matches
- **THEN** the info panel and model selector SHALL display `Deepseek Chat`

#### Scenario: Custom model with dots and hyphens
- **WHEN** the effective model ID is `gpt-4.1-mini` and no catalog entry matches
- **THEN** the info panel and model selector SHALL display `GPT 4.1 Mini`

#### Scenario: Model ID is just a provider/version string
- **WHEN** the effective model ID is `claude-opus-4-9-20250715` and no catalog entry matches
- **THEN** the system SHALL display the raw ID with hyphens and dots replaced by spaces and each word capitalized: `Claude Opus 4 9 20250715`

#### Scenario: Unknown model in model selector dropdown
- **WHEN** the effective model ID is not in the catalog and the user opens the model selector dropdown
- **THEN** the current unknown model SHALL appear as a selected entry at the top of the list
- **AND** the catalog's known models SHALL appear below it as alternative choices

### Requirement: Model label resolution is a shared frontend utility
The normalization logic SHALL be implemented as a single exported function so that both the session info panel and the model selector apply identical display rules.

#### Scenario: Single function serves both components
- **WHEN** `resolveModelLabel(providerId, modelId)` is called with a known model ID
- **THEN** it SHALL return the catalog label
- **WHEN** called with an unknown model ID
- **THEN** it SHALL return the normalized raw ID
- **WHEN** called with a null or undefined model ID
- **THEN** it SHALL return the provider's default model label

#### Scenario: Function is pure and synchronous
- **WHEN** `resolveModelLabel` is called with the same arguments
- **THEN** it SHALL always return the same result without side effects or async operations

### Requirement: Custom model display applies uniformly across chat surfaces
All chat surfaces that display model information—the main session info panel, mini chat, and model selector—SHALL use the same `resolveModelLabel` resolution for both known and unknown models.

#### Scenario: Mini chat displays same model label as main panel
- **WHEN** a session has an unknown model ID and both the main info panel and mini chat are visible
- **THEN** both SHALL display the identical normalized model label

### Requirement: Provider support information for unknown models
When a model is not in the catalog, the system SHALL apply conservative defaults for capability flags rather than claiming unsupported capabilities.

#### Scenario: Unknown model reasoning depth
- **WHEN** querying the supported reasoning depths for an unknown model ID
- **THEN** the system SHALL return only `"low"` as the maximum reasoning depth

#### Scenario: Unknown model long context support
- **WHEN** an unknown model ID is active
- **THEN** the `longContext` flag SHALL default to `false`
- **AND** the long-context toggle SHALL remain available but display a descriptive tooltip that capability information is unavailable for the custom model

