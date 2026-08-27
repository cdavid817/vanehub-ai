# managed-tool-installation Specification

## Purpose
TBD - created by archiving change extract-managed-tool-installation. Update Purpose after archive.
## Requirements
### Requirement: Host-allowlisted HTTPS artifact retrieval

The system SHALL retrieve a managed tool artifact only over HTTPS from a host on the artifact's declared allowlist, SHALL apply that allowlist to the initial URL and to every redirect target independently, and SHALL reject rather than follow a redirect that leaves the list.

#### Scenario: A redirect leaves the allowlist

- **WHEN** an artifact download is redirected to a host that is not on the declared allowlist
- **THEN** the retrieval SHALL fail with a validation outcome
- **AND** no request SHALL be issued to that host

#### Scenario: A URL disguises its host

- **WHEN** a candidate URL is not `https://`, carries userinfo before the host, or reaches an allowlisted name only through a port suffix
- **THEN** the URL SHALL be refused
- **AND** the refusal SHALL NOT depend on how many redirects have already been followed

#### Scenario: A redirect chain does not terminate

- **WHEN** a download follows more redirects than the retrieval policy admits
- **THEN** the retrieval SHALL fail rather than continue

### Requirement: Bounded, deadlined, cancellable retrieval

The system SHALL enforce the artifact's declared byte ceiling while reading rather than after the transfer completes, SHALL enforce a deadline that is checked both between redirect hops and while streaming, and SHALL observe cancellation at the same points.

#### Scenario: A response exceeds the byte ceiling

- **WHEN** an artifact's bytes exceed the declared ceiling
- **THEN** the retrieval SHALL fail before the excess bytes are retained
- **AND** the outcome SHALL NOT depend on whether the server declared a content length

#### Scenario: A server trickles bytes

- **WHEN** a server streams slowly enough that the declared timeout elapses mid-transfer
- **THEN** the retrieval SHALL fail on the deadline
- **AND** the operation SHALL NOT remain open past the declared timeout

#### Scenario: The caller cancels mid-transfer

- **WHEN** cancellation is signalled while an artifact is being retrieved
- **THEN** the retrieval SHALL stop and report cancellation
- **AND** the partially retrieved bytes SHALL be discarded

### Requirement: Digest verification before use

The system SHALL verify a retrieved artifact against its declared SHA-256 digest before the artifact is executed or extracted, and SHALL discard the artifact on mismatch.

#### Scenario: A digest does not match

- **WHEN** a retrieved artifact's SHA-256 differs from the declared digest
- **THEN** the artifact SHALL be discarded
- **AND** nothing from it SHALL be executed or extracted

#### Scenario: An artifact declares no digest

- **WHEN** an artifact declares no published digest
- **THEN** the retrieval SHALL still apply the allowlist, the byte ceiling, and the deadline
- **AND** the artifact SHALL be reported as unverified so a caller can withhold actions that require verified bytes

### Requirement: Owned temporary storage released on every exit

The system SHALL write a retrieved artifact only into storage it owns, and SHALL release that storage after success, failure, timeout, and cancellation alike.

#### Scenario: Retrieval fails after bytes are written

- **WHEN** a retrieval fails, times out, or is cancelled after writing some bytes
- **THEN** the owned storage SHALL be released
- **AND** no artifact bytes SHALL remain outside it

### Requirement: Exact-platform artifact selection without fallback

The system SHALL select a managed tool artifact by exact match on the current platform, and SHALL report that no artifact is available rather than selecting one declared for a different platform.

#### Scenario: No artifact is declared for the current platform

- **WHEN** a managed tool declares artifacts for other platforms but not the current one
- **THEN** selection SHALL yield no artifact
- **AND** the system SHALL NOT substitute an artifact declared for another platform, and SHALL NOT substitute a different acquisition source

### Requirement: Bounded archive extraction

The system SHALL extract a verified archive artifact only into a directory it owns, SHALL reject any entry whose path escapes that directory, and SHALL enforce declared limits on total extracted bytes and entry count.

#### Scenario: An archive entry escapes the destination

- **WHEN** an archive contains an entry whose path is absolute, contains a parent-directory component, or resolves outside the destination
- **THEN** extraction SHALL fail
- **AND** no entry from that archive SHALL be left in place

#### Scenario: An archive expands beyond its declared limits

- **WHEN** extraction would exceed the declared total byte ceiling or entry count
- **THEN** extraction SHALL fail before the limit is passed
- **AND** the partially extracted directory SHALL be removed

#### Scenario: Extraction succeeds

- **WHEN** a verified archive extracts within its declared limits
- **THEN** the resulting directory SHALL be reported to the caller
- **AND** the downloaded archive itself SHALL NOT be retained afterwards

### Requirement: Contributor-supplied trust policy

The system SHALL take the allowlist, byte ceiling, timeout, digest, and platform applicability from the contributing context's declaration rather than defining them centrally, and SHALL refuse a declaration that omits an allowlist or a ceiling.

#### Scenario: A contributor declares an artifact without bounds

- **WHEN** a contributed artifact declares no allowed hosts or no byte ceiling
- **THEN** the declaration SHALL be refused
- **AND** the refusal SHALL be observable at the point of declaration rather than only when a download is attempted

