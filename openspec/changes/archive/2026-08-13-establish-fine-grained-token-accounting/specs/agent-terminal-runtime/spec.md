## ADDED Requirements

### Requirement: Interactive terminal usage reconciliation
The Agent Terminal runtime SHALL ingest the finest verified provider-native usage grain available and SHALL reconcile cumulative-only sources into idempotent interval observations.

#### Scenario: Provider log exposes per-turn usage
- **WHEN** a terminal provider log exposes stable per-turn or per-message usage identities
- **THEN** the runtime SHALL persist one reported observation per unique provider turn
- **AND** revised provider records SHALL supersede prior revisions without double counting

#### Scenario: Provider exposes cumulative usage only
- **WHEN** a terminal provider exposes only running session totals
- **THEN** the runtime SHALL compare the new snapshot with its persisted cursor and account only the valid non-negative delta as reported-derived usage

#### Scenario: Poll the same snapshot repeatedly
- **WHEN** periodic, exit-time, reopen, or recovery ingestion reads an unchanged cumulative snapshot
- **THEN** it SHALL add no new consumption

#### Scenario: Provider session resets or rotates
- **WHEN** cumulative counters decrease or the bound provider session source changes
- **THEN** the runtime SHALL start a new reconciliation epoch and preserve prior intervals
- **AND** it SHALL write a bounded redacted reason diagnostic

#### Scenario: Terminal source lacks verified usage
- **WHEN** a CLI has no verified event-level or cumulative usage source
- **THEN** the runtime SHALL report unsupported or estimated coverage explicitly
- **AND** it SHALL NOT parse ANSI terminal transcript text as usage

