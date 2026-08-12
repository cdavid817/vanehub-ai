## ADDED Requirements

### Requirement: Skill Overlay management operations
The Skill management service SHALL provide scoped operations to preview effective Overlay replay, inspect diffs and history, add exact patches, add learned guidance, manage supporting files, import and promote trust, disable and revert mutations, and reconcile conflicts. Every operation SHALL address a canonical Skill id and use conflict witnesses.

#### Scenario: Desktop Overlay operation
- **WHEN** the desktop frontend submits an Overlay operation through the frontend service boundary
- **THEN** the Tauri adapter SHALL invoke the native Skill service and return the shared structured result

#### Scenario: Web Overlay operation
- **WHEN** the Web frontend submits an Overlay operation through the same service boundary
- **THEN** the Web/mock adapter SHALL model the same revision, trust, validation, and conflict behavior without requiring native filesystem access

#### Scenario: Missing effective Skill
- **WHEN** an Overlay operation targets a canonical Skill id that has no effective definition in the requested context
- **THEN** the service SHALL reject the operation without creating orphaned Overlay state

### Requirement: Overlay-aware Skill responses
Skill list, detail, preview, usage, resource, and drift responses SHALL include bounded Overlay scope summaries, trust state, active mutation counts, pinned state, effective/base hashes, and conflict or reconciliation status when applicable.

#### Scenario: Healthy Overlay summary
- **WHEN** a Skill has an applicable trusted healthy Overlay
- **THEN** its management response SHALL distinguish base content from Overlay-applied effective content and identify the active scopes

#### Scenario: Untrusted Overlay summary
- **WHEN** a Skill has an imported untrusted Overlay
- **THEN** its response SHALL identify the quarantined Overlay without presenting its content as effective

#### Scenario: Conflicted Overlay summary
- **WHEN** an applicable Overlay needs reconciliation
- **THEN** its response SHALL expose a bounded conflict count and safe reason while returning the fallback effective content selected by Overlay governance

