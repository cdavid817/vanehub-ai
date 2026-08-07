# native-runtime-architecture Specification (Delta)

## ADDED Requirements

### Requirement: Native memory injection follows custom instructions and precedes Prompt Hook assembly in the final CLI effective prompt

The native runtime SHALL combine the shared host-level memory pool with the Prompt Hook pipeline's assembled output into the final effective prompt for CLI-wrapped agents, placed after any custom-instructions section and before the Prompt Hook pipeline's own assembled content, before that text reaches the provider invocation builder. This requirement governs only where the memory section sits relative to custom instructions and the Prompt Hook pipeline; the "Native Prompt Hook pipeline" requirement's own hook evaluation, binding, and template rendering are unaffected, and the "Native custom instructions CLI injection precedes Prompt Hook assembly in the final effective prompt" requirement's own ordering guarantee is unaffected.

#### Scenario: Combine memory content between custom instructions and the Prompt Hook output
- **WHEN** a CLI chat invocation starts for `claude-code`, `codex-cli`, `gemini-cli`, or `opencode` with the memory enablement toggle on and at least one memory in the shared pool
- **THEN** the native runtime SHALL place the memory section after the custom-instructions section (if present) and before the Prompt Hook pipeline's assembled content in the final effective prompt handed to the provider invocation builder

#### Scenario: No memory content leaves the rest of the effective prompt unchanged
- **WHEN** the memory enablement toggle is off, or the shared memory pool is empty
- **THEN** the final effective prompt SHALL be exactly what it would have been without this requirement, unchanged from behavior before this requirement existed

#### Scenario: Memory resolution failure does not block CLI invocation
- **WHEN** resolving the shared memory pool fails during a CLI chat invocation
- **THEN** the native runtime SHALL log the failure and proceed with the rest of the effective prompt (custom instructions and Prompt Hook output) unaffected
- **AND** it SHALL NOT fail or delay the CLI invocation
