## 1. Database Upgrade

- [x] 1.1 Make migration 24 idempotently add remote workspace port columns alongside the SSH profile schema.
- [x] 1.2 Add a pre-version-24 upgrade regression test that verifies port columns and preserved records.

## 2. Credential Consistency

- [x] 2.1 Compensate newly written or removed credentials when an SSH profile update fails.
- [x] 2.2 Restore profile metadata when credential deletion fails after deleting an SSH profile.
- [x] 2.3 Add Rust application-service tests for credential update and deletion failure paths.

## 3. Frontend State and Validation

- [x] 3.1 Refresh SSH connection queries after both successful and failed connection tests.
- [x] 3.2 Validate save-as-connection user and authentication fields before create-session submission and show actionable feedback.
- [x] 3.3 Add frontend regression tests for failed-test refresh and save-as-connection validation while preserving manual-session behavior.

## 4. Verification

- [x] 4.1 Run frontend lint, targeted tests, full tests, and production build.
- [x] 4.2 Run Rust formatting, tests, check, and clippy.
- [x] 4.3 Run strict OpenSpec change and main-spec validation.
