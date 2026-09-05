# notification-system Specification

## Purpose
Defines the application-wide notification publishing contract, lifecycle, scope, presentation, localization, and first-version persistence boundary.

## Requirements

### Requirement: Unified notification publishing contract
The application SHALL expose a typed, application-wide notification API through React context that allows descendant components to publish success, error, warning, and informational notifications without depending on presentation markup or runtime-specific APIs.

#### Scenario: Component publishes a notification
- **WHEN** a descendant component publishes a notification with a semantic type, localized title, and optional localized message
- **THEN** the framework assigns stable runtime identity and creation metadata and makes the notification available to all notification presentations

#### Scenario: Runtime-neutral publication
- **WHEN** the same component runs in the Tauri desktop runtime or Web runtime
- **THEN** it uses the same notification API without calling a Tauri command or runtime-specific adapter

### Requirement: Bounded notification lifecycle
The framework SHALL retain a bounded set of recent in-memory notifications, SHALL mark new entries unread, and SHALL manage toast visibility separately from retained history.

#### Scenario: Toast expires
- **WHEN** a notification's configured toast duration elapses
- **THEN** its toast leaves the viewport and its recent-history entry remains available in the notification center

#### Scenario: Notification volume exceeds the bound
- **WHEN** publishing a notification would exceed the configured history or visible-toast limit
- **THEN** the framework removes or hides the oldest eligible items while retaining the newest entries

#### Scenario: User manages history
- **WHEN** the user marks entries read, removes an entry, marks all entries read, or clears the center
- **THEN** notification state and unread count update consistently

#### Scenario: Toast timer survives navigation
- **WHEN** the user navigates between workspace tabs, sessions, or routes while a toast's dismiss timer is running
- **THEN** the timer SHALL keep running against its original configured duration
- **AND** the toast SHALL still leave the viewport once that duration elapses instead of persisting indefinitely

### Requirement: Global and session notification scopes
The framework SHALL support global notifications and session-scoped notifications identified by stable session id.

#### Scenario: Relevant toast scope
- **WHEN** the toast viewport has an active session
- **THEN** it presents global toasts and toasts for that session and omits toasts scoped to other sessions

#### Scenario: All-scope notification history
- **WHEN** the user opens the notification center
- **THEN** the center presents retained notifications from all scopes without discarding notifications from inactive sessions

### Requirement: Accessible and theme-consistent presentation
The framework SHALL present notifications using existing visual tokens, semantic icons, and accessible controls in both futuristic and minimal themes, and SHALL anchor the toast viewport where it does not cover primary workspace controls.

#### Scenario: Semantic status presentation
- **WHEN** a notification is displayed
- **THEN** its type is identifiable through text or icon semantics in addition to color and its controls have accessible names

#### Scenario: Theme change
- **WHEN** the active theme changes between futuristic and minimal
- **THEN** toast and notification-center surfaces remain readable and visually consistent with the active application shell

#### Scenario: Narrow viewport
- **WHEN** notifications are displayed on a narrow viewport
- **THEN** toast and center content remain within the viewport without overlapping essential workspace controls

#### Scenario: Toast viewport placement
- **WHEN** one or more toasts are visible on a workspace-width viewport
- **THEN** the toast viewport SHALL be anchored to the top center of the application viewport, below the top bar
- **AND** it SHALL NOT overlap the top bar, the session sidebar, the composer send control, or the information panel tab strip
- **AND** toasts SHALL remain individually dismissible and SHALL stack without hiding the newest entry

### Requirement: Localized notification experience
The framework SHALL provide Simplified Chinese and English translations for all framework-owned visible text and accessible labels, and notification producers SHALL provide localized user-facing content.

#### Scenario: Locale selection
- **WHEN** the active locale is Simplified Chinese or English
- **THEN** the notification center, empty state, management controls, and accessible labels use the selected locale

### Requirement: First-version persistence boundary
The first version SHALL keep notification records in frontend memory and SHALL NOT require SQLite, Tauri commands, Web Push, or operating-system notification permissions.

#### Scenario: Application reload
- **WHEN** the application reloads or restarts
- **THEN** previous first-version notification records are not restored

#### Scenario: Future persistence integration
- **WHEN** durable notification storage is introduced later
- **THEN** desktop storage is accessed through the frontend service boundary and Rust-managed SQLite while the Web adapter exposes interface-aligned behavior

### Requirement: Curator governance notifications
The notification system SHALL publish sanitized, deduplicated events for new reviewable candidates, deferral review dates, supersession, rejection, successful application, and application failure with navigation to the relevant Curator candidate.

#### Scenario: Reviewable candidate is enqueued
- **WHEN** a candidate first becomes ready for human review
- **THEN** the system publishes at most one pending-review notification for that candidate revision

#### Scenario: Application succeeds
- **WHEN** an approved draft commits to an Overlay
- **THEN** the system publishes a success notification containing safe Skill identity, scope, candidate id, and Overlay history navigation

#### Scenario: Sensitive reason data exists
- **WHEN** a candidate or failure contains bounded user notes or sensitive source context
- **THEN** the notification excludes that content and uses a stable localized summary

### Requirement: Curator notification actions are non-mutating
Curator notifications SHALL navigate to review or history but MUST NOT approve, reject, retry, apply, resume, or otherwise mutate a candidate directly.

#### Scenario: User opens pending review notification
- **WHEN** the user activates the notification
- **THEN** the application opens the candidate review surface without performing a decision

### Requirement: Evolution orchestration notifications
The notification system SHALL publish sanitized, deduplicated notifications for partial or failed runs requiring attention, successful automatic application, probation regression, and circuit-breaker opening or recovery, with navigation to the relevant Skill Evolution view.

#### Scenario: Automatic application succeeds
- **WHEN** learned guidance commits automatically
- **THEN** one notification identifies the safe Skill, application id, probation end, and navigation target without including guidance or diff content

#### Scenario: Routine run completes
- **WHEN** a run completes without mutation or attention-required outcome
- **THEN** the system does not produce repetitive success notifications unless policy explicitly requests them

### Requirement: Orchestration notification actions are non-mutating
Notification actions SHALL only navigate and MUST NOT enable policy, close a breaker, cancel a run, approve Curator work, or revert an Overlay.

#### Scenario: User opens breaker notification
- **WHEN** the notification is activated
- **THEN** the application opens breaker health details without acknowledging it

### Requirement: Generation notifications
The notification system SHALL publish sanitized, deduplicated notifications for review-ready generation, generation failure requiring attention, cancellation, and supersession with navigation to the generation job or Curator candidate.

#### Scenario: Draft becomes reviewable
- **WHEN** a generation job packages a validated draft
- **THEN** one notification identifies safe Skill or proposal identity, draft kind, job id, and Curator navigation without including generated content

#### Scenario: Routine stage advances
- **WHEN** a job moves between normal internal stages
- **THEN** the system does not emit repetitive notifications

### Requirement: Generation notification actions are non-mutating
Generation notification actions SHALL only navigate and MUST NOT enable consent, regenerate, cancel, approve, install, or apply a draft.

#### Scenario: User activates review-ready notification
- **WHEN** the notification is opened
- **THEN** the application navigates to the review surface without changing job or Curator state

### Requirement: Canonical evolution projection notifications
Evolution notifications SHALL be derived from the same canonical safe activity envelope used by system sessions and dashboards, with independent target delivery receipts and user threshold/digest policy.

#### Scenario: Attention event is projected
- **WHEN** an event meets notification policy
- **THEN** the notification references the same event id, safe parameters, severity, and navigation descriptor as the system timeline

#### Scenario: Event was already notified
- **WHEN** catch-up or rebuild sees its source again
- **THEN** the existing notification receipt prevents duplicate publication

### Requirement: Notification and activity read coordination
Opening a notification SHALL navigate to its activity or detail target and MAY advance the associated system-session read cursor only after the referenced item becomes visible. Dismissing a notification MUST NOT delete activity.

#### Scenario: Notification target is not yet projected
- **WHEN** the user opens a notification while timeline delivery is delayed
- **THEN** the UI opens the relevant detail or pending state without falsely marking unseen activity read

### Requirement: Evolution notification digests
The notification service SHALL support bounded per-scope digests for non-urgent evolution outcomes while security, integrity, apply failure, regression, and breaker events remain individually attention eligible according to policy.

#### Scenario: Several informational results occur
- **WHEN** digest mode is enabled
- **THEN** the system emits one bounded summary with counts and navigation rather than repetitive individual notifications

### Requirement: Projected notification actions remain non-mutating
Notifications derived from activity envelopes SHALL only navigate or adjust notification/read presentation state. They MUST NOT execute any evolution action.

#### Scenario: Automatic application notification is opened
- **WHEN** the user activates it
- **THEN** the application opens its system activity or Overlay history without reverting or approving content
