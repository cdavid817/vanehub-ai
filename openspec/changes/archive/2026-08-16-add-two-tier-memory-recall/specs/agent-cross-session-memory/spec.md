## ADDED Requirements

### Requirement: Relevance-selected memory bodies

The system SHALL, for each OnePiece turn while the memory enablement toggle is on, select a bounded number of memories judged relevant to that turn and inject their bodies alongside the always-present index. Selection SHALL operate on a manifest of each memory's name, type, description, and age rather than on memory bodies, so that its cost scales with the number of memories rather than their size. Selection SHALL be instructed to return nothing when no memory is clearly useful, rather than returning its most plausible guess. A selection failure SHALL degrade to index-only injection without failing the generation.

#### Scenario: Relevant memories are injected in full

- **WHEN** a turn runs with the memory enablement toggle on and selection judges some memories relevant
- **THEN** the system SHALL inject those memories' bodies for that turn, up to the selection bound
- **AND** the index SHALL still be present, unchanged by the selection

#### Scenario: Nothing is clearly relevant

- **WHEN** selection judges no stored memory clearly useful for the turn
- **THEN** the system SHALL inject no memory bodies for that turn, without treating this as a failure
- **AND** the index SHALL still be present

#### Scenario: Selection never exceeds its bound

- **WHEN** selection judges more memories relevant than the bound allows
- **THEN** the system SHALL inject no more than the bound

#### Scenario: Selection fails

- **WHEN** the selection step errors, times out, or returns an unusable result
- **THEN** the system SHALL log the failure and inject the index alone
- **AND** the generation SHALL proceed unaffected

#### Scenario: Selection names a memory that does not exist

- **WHEN** selection returns a name with no corresponding memory file
- **THEN** the system SHALL discard that name and inject the remaining selected memories

#### Scenario: No selection when memory is disabled

- **WHEN** the memory enablement toggle is off
- **THEN** the system SHALL NOT run selection and SHALL NOT make a selection call

### Requirement: Already-surfaced memories are excluded from selection

The system SHALL track, for the duration of a session, which memories have already had their bodies injected, and SHALL exclude those from a later selection in the same session before the selection step runs. Exclusion before selection rather than after ensures the bounded selection budget is spent on candidates the model has not seen yet.

#### Scenario: A memory surfaced earlier is not re-selected

- **WHEN** a memory's body was injected on an earlier turn of the same session and selection runs again
- **THEN** that memory SHALL NOT be offered to selection as a candidate
- **AND** the selection bound SHALL be available for memories not yet surfaced

#### Scenario: A new session starts fresh

- **WHEN** a new session begins
- **THEN** every memory SHALL again be eligible for selection

#### Scenario: A corrected memory becomes eligible again

- **WHEN** a memory whose body was already surfaced in this session is subsequently updated
- **THEN** it SHALL become eligible for selection again, since its content is no longer the content the model saw

### Requirement: Injected memories carry age and staleness caveats

The system SHALL annotate each injected memory body with a human-readable elapsed time rather than a raw timestamp, because a timestamp alone does not reliably trigger staleness reasoning. A memory older than the staleness threshold SHALL additionally carry a caveat stating that memories are point-in-time observations, that claims about code or file locations may be outdated, and that they are to be verified against current state before being asserted as fact. Memories within the threshold SHALL NOT carry the caveat, since a caveat on fresh content is noise.

#### Scenario: A stale memory is annotated

- **WHEN** a memory older than the staleness threshold is injected
- **THEN** its injected text SHALL include its elapsed age in human-readable form
- **AND** it SHALL include the verify-before-asserting caveat

#### Scenario: A fresh memory is not caveated

- **WHEN** a memory within the staleness threshold is injected
- **THEN** its injected text SHALL include its elapsed age
- **AND** it SHALL NOT include the staleness caveat

### Requirement: Web runtime parity for memory selection

The Web/mock runtime SHALL expose the same observable memory-selection behavior as the desktop runtime through the same event and service contracts, and SHALL NOT call a real provider to produce it. Selection SHALL be gated by the memory enablement toggle in the Web runtime exactly as it is on the desktop.

#### Scenario: Mock selection emits the same events

- **WHEN** a mock generation runs with the memory enablement toggle on and stored memories present
- **THEN** the Web adapter SHALL simulate index injection and body selection through the same contracts the desktop runtime uses
- **AND** it SHALL NOT issue a provider request to do so

#### Scenario: Selection suppressed when memory is disabled

- **WHEN** the memory enablement toggle is off during a mock generation
- **THEN** the Web adapter SHALL NOT simulate a selection event

## MODIFIED Requirements

### Requirement: Memory injection into the system prompt

The system SHALL inject the memory index into OnePiece's generation requests as part of the system prompt while the memory enablement toggle is on, and SHALL never write memory content into the turns list context compaction manipulates. The always-present surface SHALL be the index — one pointer line per memory — rather than memory bodies; bodies reach the request only through the separate "Relevance-selected memory bodies" requirement. Index injection SHALL be bounded by both a line cap and a byte cap, since a small number of overlong lines defeats a line cap alone, and when either cap truncates the index the injected text SHALL say so explicitly rather than silently presenting a partial index as complete. Index injection SHALL NOT require any embedding or retrieval configuration. This requirement governs only OnePiece's system-prompt injection; CLI-wrapped agents are governed instead by the separate "Memory injection into CLI prompts" requirement.

#### Scenario: Memories injected alongside Skill content

- **WHEN** a generation runs for an agent with both bound Skills and stored memories in scope, and the memory enablement toggle is on
- **THEN** the system prompt SHALL include both, as distinct sections

#### Scenario: The index is what is always present

- **WHEN** a generation runs with the memory enablement toggle on and stored memories present
- **THEN** the system prompt SHALL contain one index line per memory
- **AND** it SHALL NOT contain memory bodies except those the relevance selection contributed for that turn

#### Scenario: Injected memories are bounded

- **WHEN** the memory pool is large enough that its index would exceed a cap
- **THEN** the system SHALL bound what it injects rather than including the index unbounded
- **AND**, replacing this scenario's previous single character budget over memory bodies, the bound SHALL be the paired line and byte caps applied to the index

#### Scenario: A corrected memory becomes the most recent

- **WHEN** a memory that was written long ago is updated during a session
- **THEN** the next injection SHALL order its index entry ahead of memories that have not been modified since
- **AND** truncation SHALL therefore drop the least recently modified entries first

#### Scenario: Index exceeds the line cap

- **WHEN** the index holds more lines than the line cap allows
- **THEN** the system SHALL inject the index up to the cap
- **AND** the injected text SHALL state that the index was truncated and why

#### Scenario: Index exceeds the byte cap within the line cap

- **WHEN** the index is within the line cap but exceeds the byte cap because individual entries are long
- **THEN** the system SHALL inject the index up to the byte cap, cut at an entry boundary rather than mid-entry
- **AND** the injected text SHALL state that the index was truncated and why

#### Scenario: Memory content survives compaction

- **WHEN** context compaction triggers during a generation with injected memory content
- **THEN** the injected memory content SHALL remain present, complete, and unchanged on every subsequent request of that generation

#### Scenario: No injection when memory is disabled

- **WHEN** the memory enablement toggle is off
- **THEN** the system SHALL NOT read the memory directory for injection and SHALL send the request without a memory section

### Requirement: Memory injection into CLI prompts

The system SHALL prepend the memory index to the Prompt-Hook-assembled effective prompt for every message sent to a CLI-wrapped agent (`claude-code`, `codex-cli`, `gemini-cli`, `opencode`), after the custom-instructions section and before the Prompt Hook pipeline's own assembled content, while the memory enablement toggle is on. This surface SHALL have its own injection bound, separate from OnePiece's: OnePiece's index is injected once into a system prompt that is cached across a generation, while this one is prepended to every message sent to a subprocess, so the two SHALL NOT be required to share a single limit. Relevance-selected bodies SHALL NOT be injected here, since a CLI-wrapped agent's turn boundary is not visible to VaneHub in the way OnePiece's is. This requirement governs only the CLI delivery mechanism; OnePiece's own system-prompt injection remains governed by the "Memory injection into the system prompt" requirement.

#### Scenario: Memory section precedes the Prompt Hook assembly

- **WHEN** a message is sent to a CLI-wrapped agent with the memory enablement toggle on and at least one memory in the shared pool
- **THEN** the final text delivered to that CLI process SHALL contain the memory index after the custom-instructions section (if any) and before the Prompt-Hook-assembled content

#### Scenario: CLI bound is applied independently

- **WHEN** an index fits within OnePiece's bound but exceeds the CLI bound
- **THEN** the text delivered to the CLI process SHALL be truncated to the CLI bound and say so
- **AND** OnePiece's own injection SHALL be unaffected

#### Scenario: Disabled or empty produces no injection

- **WHEN** the memory enablement toggle is off, or the shared memory pool is empty
- **THEN** the text delivered to the CLI process SHALL be unchanged by this requirement

#### Scenario: Injection query failure does not block the CLI message

- **WHEN** resolving memories fails while sending a message to a CLI-wrapped agent
- **THEN** the system SHALL log the failure and send the message without the memory section
- **AND** it SHALL NOT fail or delay the message send
