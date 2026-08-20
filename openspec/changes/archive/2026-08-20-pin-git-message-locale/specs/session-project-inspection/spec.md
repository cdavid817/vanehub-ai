## ADDED Requirements

### Requirement: Git inspection outcomes are locale-independent

The native runtime SHALL execute Git inspection commands with a pinned message locale so that outcome classification — non-Git detection and untracked-path detection in particular — does not depend on the host system's display language. User-facing presentation of the classified outcome SHALL remain localized as specified elsewhere in this capability.

#### Scenario: Non-Git directory on a non-English host

- **WHEN** the selected session root is not a Git repository and the host locale is not English
- **THEN** the runtime SHALL classify it as the non-Git case
- **AND** Changes SHALL show the localized non-Git empty state rather than a raw command failure

#### Scenario: Untracked path on a non-English host

- **WHEN** an untracked path is probed on a host whose locale is not English
- **THEN** the runtime SHALL classify it as untracked rather than as a Git command failure

#### Scenario: Caller-supplied environment still applies

- **WHEN** a Git invocation is made with explicit caller-supplied environment variables
- **THEN** those variables SHALL take precedence over the pinned locale default for that invocation
