## ADDED Requirements

### Requirement: Checksum manifest verifies published assets
A published checksum manifest SHALL identify each asset by the name under which it is served, so that a downloader who places the manifest beside a downloaded asset can verify it with a standard checksum tool and no renaming. The publishing job SHALL fail rather than publish a manifest in which two entries share a name.

#### Scenario: Downloader verifies an asset
- **WHEN** a downloader places the published manifest in the same directory as a downloaded asset and runs a standard checksum check
- **THEN** that asset SHALL be verified against its recorded digest

#### Scenario: Asset name differs from its build-time path
- **WHEN** the release host serves an asset under a name that differs from its path on the build machine
- **THEN** the manifest SHALL record the served name

#### Scenario: Two assets would share a published name
- **WHEN** two assets would be served under the same name
- **THEN** the publishing job SHALL fail before creating the release

#### Scenario: Maintainer reviews a release run
- **WHEN** the publishing job generates the manifest
- **THEN** its contents SHALL appear in the job log
