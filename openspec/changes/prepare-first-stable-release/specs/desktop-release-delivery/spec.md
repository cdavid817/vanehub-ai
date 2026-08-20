## ADDED Requirements

### Requirement: Reviewed stable release narrative
A stable GitHub Release SHALL prepend a version-appropriate, repository-tracked release narrative to generated change notes. The narrative MUST identify supported platform packages, the evidence available to verify them, update-channel behavior, support and security-reporting routes, and material known limitations without claiming verification that the release workflow did not produce.

#### Scenario: Stable tag is published
- **WHEN** a stable version tag passes every required build and verification gate
- **THEN** the GitHub Release SHALL include the reviewed stable narrative before generated change notes
- **AND** the narrative SHALL describe Windows and macOS signing evidence and Linux integrity/provenance evidence according to their actual verification status

#### Scenario: Stable release notes are changed
- **WHEN** stable installation guidance, supported packages, verification evidence, or known limitations change
- **THEN** the narrative SHALL be updated as a reviewed repository file before the version tag is created

#### Scenario: Preview tag is published
- **WHEN** a semantic-version prerelease tag is published
- **THEN** the existing preview-specific installation and unsigned-package guidance SHALL be used instead of the stable narrative

### Requirement: Stable release readiness record
Before creating a stable version tag, maintainers SHALL verify synchronized release metadata, complete the repository's required validation commands, complete a non-publishing package rehearsal for every declared target, and confirm that the protected release environment exposes every required credential name. Publication results MUST be reported per platform as `PASSED`, `FAILED`, `BLOCKED`, or `NOT RUN` and MUST NOT infer one platform's result from another.

#### Scenario: Stable release is ready to tag
- **WHEN** synchronized version checks and all required repository validations pass, every declared rehearsal target succeeds, and all protected credential names are provisioned
- **THEN** the reviewed commit SHALL be eligible for an annotated stable version tag

#### Scenario: Protected credential is absent
- **WHEN** any updater, Windows signing, or Apple signing/notarization credential required by the stable workflow is not provisioned
- **THEN** release readiness SHALL be `BLOCKED`
- **AND** maintainers SHALL NOT create the stable version tag

#### Scenario: Platform verification has not run
- **WHEN** a native rehearsal or release verification did not execute on a declared platform
- **THEN** that platform SHALL be reported as `NOT RUN`
- **AND** another platform's result SHALL NOT be used as substitute evidence
