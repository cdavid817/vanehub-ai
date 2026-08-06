# native-runtime-architecture Specification (Delta)

## ADDED Requirements

### Requirement: Native custom instructions CLI injection precedes Prompt Hook assembly in the final effective prompt

The native runtime SHALL combine host-level custom instructions with the Prompt Hook pipeline's assembled output into one final effective prompt, before that text reaches the provider invocation builder. This requirement governs only where custom instructions are combined relative to the Prompt Hook pipeline; the "Native Prompt Hook pipeline" requirement's own hook evaluation, binding, and template rendering are unaffected.

#### Scenario: Combine custom instructions ahead of the Prompt Hook output
- **WHEN** a CLI chat invocation starts for `claude-code`, `codex-cli`, `gemini-cli`, or `opencode` with custom instructions enabled and non-empty
- **THEN** the native runtime SHALL place the custom-instructions section before the Prompt Hook pipeline's assembled content in the final effective prompt handed to the provider invocation builder

#### Scenario: No custom instructions leaves Prompt Hook assembly unchanged
- **WHEN** custom instructions are disabled or empty
- **THEN** the final effective prompt SHALL be exactly the Prompt Hook pipeline's own assembled output, unchanged from behavior before this requirement existed

#### Scenario: Custom instructions resolution failure does not block CLI invocation
- **WHEN** resolving custom instructions fails during a CLI chat invocation
- **THEN** the native runtime SHALL log the failure and proceed with the Prompt Hook pipeline's assembled output alone
- **AND** it SHALL NOT fail or delay the CLI invocation
