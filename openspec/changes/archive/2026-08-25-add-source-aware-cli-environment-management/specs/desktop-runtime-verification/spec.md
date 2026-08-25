## ADDED Requirements

### Requirement: Two desktop verification gates with distinct prerequisites

Desktop verification SHALL be split into a Required Hermetic Desktop Gate and an External Provider Desktop Suite, and every desktop spec SHALL belong to exactly one of them.

The split exists because a single suite cannot be both. A gate every pull request must pass cannot depend on a real CLI Agent, a real credential, or a real vendor download, and a suite that verifies the real thing cannot be hermetic. Merging them means either the gate silently requires a developer's machine — which is what made `desktop-smoke` fail on all three hosted runners for want of `codex` on PATH — or the real-integration cases quietly stop running.

#### Scenario: Required gate runs on an ordinary pull request

- **WHEN** the Required Hermetic Desktop Gate runs on Windows, macOS, or Linux
- **THEN** it SHALL run against a temporary HOME, PATH, user-data directory, and SQLite database
- **AND** it SHALL resolve every CLI Agent to a fixture executable rather than a host installation
- **AND** it SHALL NOT contact a real provider, read a credential store, download from a vendor, or read the user's application state
- **AND** any failing required spec SHALL fail the gate

#### Scenario: Required gate cannot silently degrade

- **WHEN** a required spec cannot run because a CLI Agent, package manager, or other fixture-resolvable prerequisite is missing
- **THEN** the gate SHALL report `FAILED` rather than skipping the spec
- **AND** the missing prerequisite SHALL be treated as a defect in the fixture, not as an environment block

#### Scenario: Required spec reports a genuinely external prerequisite

- **WHEN** part of a required spec depends on something no fixture can stand in for, such as a live vendor release endpoint
- **THEN** that part MAY record a `BLOCKED` reason and continue
- **AND** the reason SHALL name the prerequisite
- **AND** the gate SHALL still report `PASSED` only if no required assertion failed

#### Scenario: External provider suite runs outside the gate

- **WHEN** the External Provider Desktop Suite is dispatched
- **THEN** it SHALL be triggered manually, on a schedule, or by a protected label rather than by an ordinary pull request
- **AND** it SHALL NOT be a required check for merging

#### Scenario: External provider suite lacks its prerequisites

- **WHEN** a real CLI Agent, credential, or provider endpoint the suite needs is absent
- **THEN** it SHALL record `BLOCKED` with the specific missing prerequisite
- **AND** it SHALL NOT record `PASSED`
- **AND** the `BLOCKED` result SHALL NOT count toward the Required Hermetic Desktop Gate

### Requirement: Every desktop spec is classified and the classification is enforced

Each desktop spec SHALL carry exactly one classification of `required-fixture`, `external-provider`, or `duplicate-replaced`, recorded in a manifest that automated tests check.

A classification kept only in prose drifts the first time a spec is added or renamed. Enforcing it mechanically is what keeps "every spec is classified" true rather than aspirational.

#### Scenario: A spec is added without a classification

- **WHEN** a desktop spec file exists that the manifest does not classify
- **THEN** the desktop verification tests SHALL fail and name the unclassified spec

#### Scenario: The manifest names a spec that no longer exists

- **WHEN** a manifest entry has no corresponding spec file
- **THEN** the desktop verification tests SHALL fail and name the stale entry

#### Scenario: A required spec declares an external prerequisite

- **WHEN** a spec classified `required-fixture` declares a real credential, a real provider, or vendor network access
- **THEN** the desktop verification tests SHALL fail

#### Scenario: An external spec reaches the required command

- **WHEN** a spec classified `external-provider` is included in the Required Hermetic Desktop Gate's spec set
- **THEN** the desktop verification tests SHALL fail

#### Scenario: A replaced spec names no replacement

- **WHEN** a spec is classified `duplicate-replaced`
- **THEN** the manifest SHALL name the spec or layer that covers the same behaviour
- **AND** the desktop verification tests SHALL fail if that replacement does not exist

### Requirement: Fixture-resolvable behaviour belongs to the required gate

A desktop spec that verifies CLI process lifecycle, standard output or error handling, session creation, tab, drawer or dialog behaviour, operations, cancellation, error reporting, persistence, PATH resolution, or the Agent Runtime call boundary SHALL be classified `required-fixture` and driven by a fixture CLI.

These behaviours are properties of this application, not of any vendor's binary. Verifying them against a real CLI Agent buys nothing and costs the ability to run the gate anywhere.

#### Scenario: A spec needs an installed Agent to exercise application behaviour

- **WHEN** a required spec needs a CLI Agent to be present
- **THEN** the gate SHALL place fixture executables for the managed Agent names ahead of the inherited PATH
- **AND** the spec SHALL exercise the same production resolution, launch, and persistence paths against them

#### Scenario: Only vendor-specific truth is external

- **WHEN** a spec verifies a real provider login, real account permissions, a real server response, real model output, or a real vendor CLI's current-version compatibility
- **THEN** it SHALL be classified `external-provider`
- **AND** it SHALL declare its prerequisites and the reason it is blocked without them
