## ADDED Requirements

### Requirement: Separately listed system activity sessions
The session service SHALL expose system activity sessions through a separate listing operation and discriminated session kind. They SHALL NOT be included in interactive active/archived lists, categories, automatic inactive archival, multi-session deletion, Agent discovery, or workflow selection.

#### Scenario: List all session groups
- **WHEN** the UI requests interactive and system session collections
- **THEN** it receives separately typed collections without inferring system identity from titles

#### Scenario: Select system activity view
- **WHEN** a system session is opened
- **THEN** view selection changes without updating active interactive session or workflow Agent state

### Requirement: System session mutation refusal
The session service SHALL reject normal create, rename, pin, unpin, archive, restore, category assignment, delete, send, stop, terminal, provider-resume, and chat-configuration operations for system activity sessions.

#### Scenario: Bulk delete includes system session
- **WHEN** a client includes a system activity id in an interactive deletion request
- **THEN** the service rejects or excludes it with an explicit immutable-system-session result and preserves the session

### Requirement: System activity search separation
Ordinary historical Agent-session message search SHALL exclude system activity by default. System activity SHALL use its dedicated safe search and MAY appear in global search only under an explicit system-activity result kind.

#### Scenario: Ordinary session search matches an activity label
- **WHEN** the user searches interactive session history without enabling system activity
- **THEN** system activity is not returned as an Agent session result

