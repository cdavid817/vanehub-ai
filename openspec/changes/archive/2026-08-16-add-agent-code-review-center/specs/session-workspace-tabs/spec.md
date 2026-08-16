## ADDED Requirements

### Requirement: Changes tab Review Center workflow
The existing Changes tab SHALL be the session-scoped Review Center entry and SHALL preserve lazy mounting and keep-alive behavior while adding review creation/recovery, comments, findings, decisions, feedback, and guarded actions.

#### Scenario: Open Review Changes from a session
- **WHEN** a local session has workspace modifications and the user activates Changes
- **THEN** the tab SHALL open or recover its review through the frontend service boundary without adding a ninth workspace tab

#### Scenario: Keep review state while switching tabs
- **WHEN** the user leaves and returns to Changes in the same session
- **THEN** selected file, view mode, draft comment, and loaded review data SHALL remain available under the existing keep-alive lifecycle

#### Scenario: Review on narrow workspace
- **WHEN** the center panel is narrow
- **THEN** changed-file selection and the diff/comment surface SHALL remain keyboard accessible without unrecoverable page overflow
