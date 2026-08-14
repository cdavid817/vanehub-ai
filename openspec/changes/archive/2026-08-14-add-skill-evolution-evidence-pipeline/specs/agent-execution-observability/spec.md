## ADDED Requirements

### Requirement: Evidence-safe execution projection
The native observability boundary SHALL project registered execution outcomes into a bounded versioned evidence source envelope after applying existing metadata privacy rules. Evidence projection SHALL be local, asynchronous, non-blocking, and independent of optional OTLP export.

#### Scenario: Native run projects Skill revisions
- **WHEN** a native API run reaches a registered tool, verification, delegation, or terminal outcome
- **THEN** the projection SHALL include safe correlation, fidelity, status, counts, and exact effective Skill revision associations observed by that run

#### Scenario: CLI projection preserves fidelity
- **WHEN** a managed or interactive CLI run emits a registered observable outcome
- **THEN** the projection SHALL preserve native, proxied, inferred, or opaque fidelity plus only the binding and mount facts actually captured for that run

#### Scenario: OTLP disabled
- **WHEN** optional OTLP export is disabled
- **THEN** local evidence projection MAY continue according to local evidence policy

#### Scenario: Evidence projection fails
- **WHEN** projection or enqueue fails
- **THEN** the execution run and its observability timeline SHALL continue normally and a rate-limited redacted diagnostic MAY be emitted

### Requirement: Observed Skill revision set
Execution metadata SHALL record the bounded set of canonical Skill revisions actually injected, successfully loaded, delegated, or actively mounted for each eligible run stage, with association kind and observation time.

#### Scenario: Eager Skill recorded
- **WHEN** an eager Skill is included in the final native API prompt
- **THEN** observability SHALL record its canonical id, effective revision hash, and `injected` association for that generation

#### Scenario: On-demand Skill recorded
- **WHEN** `load_skill` returns effective instructions successfully
- **THEN** observability SHALL record the canonical id, effective revision hash, and `loaded` association for that generation

#### Scenario: Utility recorded
- **WHEN** a delegated Utility child begins
- **THEN** observability SHALL record its canonical id, effective revision hash, and `delegated` association on parent and child topology

#### Scenario: CLI configured but not mounted
- **WHEN** a CLI Skill binding exists but no active mount snapshot was captured
- **THEN** observability SHALL NOT label that Skill as used or mounted

