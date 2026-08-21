## ADDED Requirements

### Requirement: External extension packages have verifiable provenance and immutable evidence

Every external `.vhext` installation SHALL bind publisher trust, signature envelope, canonical manifest digest, package content hash, exact version, compatibility, requested capability manifest, and immutable snapshot identity. Provenance evidence SHALL be retained across enable, disable, update, rollback, quarantine, and uninstall according to audit/retention policy.

#### Scenario: Same version has different bytes

* WHEN a package claims an already-known extension id/version but has a different content hash
* THEN the system treats it as distinct suspicious evidence and does not silently replace the prior snapshot

### Requirement: Archive and package parsing share application security ceilings

Extension package parsing SHALL use application-owned safe archive/path/canonicalization policies and SHALL reuse shared package-security primitives when available. A source or manifest MAY tighten but SHALL NOT widen application ceilings.

#### Scenario: Manifest requests larger archive allowance

* WHEN an extension declares limits above the application maximum
* THEN the application maximum remains effective

### Requirement: Developer Mode is explicit and contained

Installing unsigned extension content SHALL require explicit Developer Mode, persistent warning, Strict disabled-by-default state, audit, and no automatic updates/startup activation. Developer Mode SHALL NOT disable archive, path, compatibility, Permissions, Hook, rule, connector, logging, or runtime limits.

#### Scenario: Developer Mode is turned off

* WHEN unsigned packages remain installed
* THEN they become or remain ineligible for new activation until signed/trusted or Developer Mode is explicitly re-enabled, without silently deleting evidence
