## ADDED Requirements

### Requirement: Dependency update automation covers every built ecosystem
Automated dependency updates SHALL be configured for every package ecosystem this repository builds
from, and each configured ecosystem SHALL point at a directory that holds the manifest or lockfile
its updater reads. A configuration that names a directory holding neither SHALL fail validation
rather than run and produce nothing.

The failure this exists for is silent. An updater pointed at the wrong directory opens no pull
request, raises no error, and leaves a repository looking maintained while one of its ecosystems has
not been updated at all — which is indistinguishable, from the outside, from having no advisories.

#### Scenario: A lockfile moves and the configuration does not
- **WHEN** a lockfile moves to a different directory and the update configuration still names the old one
- **THEN** validation SHALL fail and name the ecosystem and the directory it could not read
- **AND** it SHALL NOT report the configuration as valid because the directory itself still exists

#### Scenario: Every built ecosystem is configured
- **WHEN** the repository builds from a package ecosystem
- **THEN** that ecosystem SHALL appear in the update configuration
- **AND** its directory SHALL resolve to a manifest or lockfile that exists

### Requirement: A test that cannot run reports why
A test whose prerequisites are absent SHALL report that it did not run, naming the prerequisite it
needed. It SHALL NOT return early in a way that is indistinguishable from having passed.

A skipped test and a passing test are different facts, and a reviewer reading a green run acts on
them differently. A bare early `return` reports the second while establishing the first, and the
longer it does so the more coverage the suite appears to have.

#### Scenario: A required prerequisite is missing
- **WHEN** a test cannot run because a browser, interpreter, or fixture it requires is absent
- **THEN** it SHALL emit a message naming the missing prerequisite
- **AND** it SHALL NOT assert anything that its early return has not established

#### Scenario: The prerequisite is present
- **WHEN** every prerequisite the test requires is available
- **THEN** the test SHALL run its full path and assert what it is named for
