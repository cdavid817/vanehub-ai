## ADDED Requirements

### Requirement: Curator service boundary
The Skill management service SHALL expose scoped Curator queue, candidate detail, audit history, policy, draft revision, preview, defer, resume, reject, approve, and retry operations through matching desktop/Tauri and Web runtime adapters. React components MUST NOT invoke native commands directly.

#### Scenario: Desktop Curator action
- **WHEN** the desktop UI performs a Curator action through the Skill service
- **THEN** the Tauri adapter invokes the native command and returns typed state, conflict, validation, or application results

#### Scenario: Web Curator action
- **WHEN** the Web UI performs the same action
- **THEN** the Web adapter returns behaviorally equivalent mock or backend results with matching version and error semantics

### Requirement: Conflict-safe Curator commands
Every mutating Curator service operation SHALL require the expected candidate version and all action-specific witnesses and SHALL return the current safe state on a stale conflict.

#### Scenario: Two review surfaces mutate one candidate
- **WHEN** one surface advances the candidate before the second submits its action
- **THEN** the second receives a stale conflict and cannot overwrite the first decision

### Requirement: Bounded Curator payloads
Curator service responses SHALL return sanitized, paginated, size-bounded summaries, diffs, draft data, and audit events and SHALL exclude raw prompts, terminal output, credentials, provider payloads, and rejected unsafe content.

#### Scenario: Diff exceeds response limit
- **WHEN** an effective diff exceeds the service response budget
- **THEN** the service returns a paginated or truncated representation with explicit completeness metadata and does not weaken approval requirements

