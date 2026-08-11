## ADDED Requirements

### Requirement: Skill registry repository security
Remote Skill distribution SHALL use threshold-signed, versioned, expiring repository metadata with trusted root bootstrap and rotation, delegated publisher namespaces, target hashes and sizes, and rollback/freeze protection. Product builds SHALL pin and review the repository-security implementation and cryptographic dependencies.

#### Scenario: Registry security dependency changes
- **WHEN** the repository-security or cryptographic implementation is added or updated
- **THEN** dependency review covers licenses, advisories, algorithm support, maintained status, and verification test vectors before release

### Requirement: Skill package provenance and integrity evidence
Every installed Registry Skill snapshot SHALL retain verifiable source, publisher namespace, package/version identity, metadata versions, target hashes, complete extracted content manifest, validation result, installation time, and operation correlation. Evidence SHALL be sufficient to recheck local integrity without trusting mutable display metadata.

#### Scenario: Installed package integrity is audited
- **WHEN** an operator inspects a Registry Skill
- **THEN** the system can compare the immutable local snapshot with its retained target and content-manifest hashes and report a redacted result

### Requirement: Adversarial registry verification
The release verification suite SHALL cover invalid thresholds, unknown keys, expired metadata, key rotation, rollback, freeze, mix-and-match, delegation escape, target substitution, length/hash mismatch, redirect abuse, decompression bombs, traversal, links, path collisions, reserved paths, partial transaction failure, revocation, and offline stale metadata.

#### Scenario: Security fixture is rejected
- **WHEN** an adversarial metadata or package fixture violates one of the protected conditions
- **THEN** the test suite proves no active Registry snapshot or trusted metadata state is created from it

