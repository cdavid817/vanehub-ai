## ADDED Requirements

### Requirement: Chat history displays context compaction evidence
The chat experience SHALL render each successful automatic context compaction as an accessible rich card in chronological message history, showing content-free before/after measurements, savings, trigger source, compaction path, and policy version.

#### Scenario: Successful compaction occurs during streaming
- **WHEN** the active OnePiece generation emits a successful compaction evidence card
- **THEN** the card SHALL appear with the corresponding assistant message without interrupting token streaming

#### Scenario: Token count is unavailable
- **WHEN** the evidence reports token measurements as unavailable
- **THEN** the card SHALL clearly distinguish unavailable token evidence from a zero token count

#### Scenario: Reload conversation history
- **WHEN** a conversation containing compaction evidence is restored
- **THEN** the evidence card SHALL retain its field values and chronological position

