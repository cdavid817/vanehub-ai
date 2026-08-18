## ADDED Requirements

### Requirement: Canonical Runs retain bounded Runner ownership
Every accepted Agent generation Run SHALL persist one immutable runner kind and stable bounded runner reference plus versioned capability, authority, and recovery witnesses needed by its owner. Runner metadata SHALL contain no credential, raw environment, prompt, output, unrestricted path, or transport secret.

#### Scenario: Create a Local or SSH Run
- **WHEN** Agent generation is accepted for an eligible Runner
- **THEN** runner identity and recovery classification are committed with the canonical Run before it enters running

#### Scenario: Read an existing Run without runner metadata
- **WHEN** a legacy Run snapshot is loaded after migration
- **THEN** it remains readable and is conservatively projected as legacy Local only where existing ownership proves that classification
- **AND** no remote capability or live state is fabricated

### Requirement: Runner-aware canonical cancellation and recovery
Canonical cancellation SHALL delegate owned process or channel termination through the Run owner's Runner handle, and startup recovery SHALL use runner inspection evidence before choosing reconnect, interrupted failure, or attention-required state. A Runner MUST NOT create a second Run lifecycle authority.

#### Scenario: Cancel an SSH Run
- **WHEN** canonical cancellation wins the Run version race
- **THEN** the owning remote process/channel receives cancellation and late Runner or provider completion cannot reverse the terminal state

#### Scenario: Restart with an unverifiable Runner
- **WHEN** a non-terminal Run's Runner cannot prove live ownership
- **THEN** canonical recovery stops presenting it as running and records one idempotent safe recovery outcome

