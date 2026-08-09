## MODIFIED Requirements

### Requirement: Memory scoping
The system SHALL treat stored memories as a single host-level pool shared by every agent — OnePiece and all CLI-wrapped agents (`claude-code`, `codex-cli`, `gemini-cli`, `opencode`, `antigravity-cli`) alike — rather than scoping them to the agent or workspace folder that produced them. The system SHALL record which agent and, when available, which workspace folder produced each memory as provenance metadata on the stored record, without using either as a filter for injection, listing, or management.

#### Scenario: Memory scoped to agent and folder
- **WHEN** a memory is saved during a session with a workspace folder, whether by OnePiece's explicit tool, OnePiece's automatic extraction, or a CLI-wrapped agent's automatic extraction
- **THEN** the system SHALL record the producing agent id and that workspace folder as provenance metadata alongside the memory content
- **AND**, unlike before `add-cli-memory-support`, neither value SHALL restrict which future generations or management views can read that memory

#### Scenario: Memory scoped to agent only when no folder is available
- **WHEN** a memory is saved during a session with no workspace folder
- **THEN** the system SHALL still save it into the shared pool, recording no folder rather than rejecting the save
- **AND** a generation or management view operating in any workspace folder, or with no workspace folder at all, SHALL still be able to read, inject, and manage it

#### Scenario: Memories do not cross agents
- **WHEN** two different agents produce memories, whether or not they share a workspace folder
- **THEN**, reversing the isolation this scenario previously guaranteed, the system SHALL make each agent's memories visible to every other agent's generations and management views via the shared host-level pool, exactly as if they had produced it themselves
