## Purpose

Provides OnePiece with bounded Playwright browser automation, inspectable evidence, and explicit human handoff while keeping navigation and page actions inside governed sessions.

## ADDED Requirements

### Requirement: Owned browser automation sessions
The system SHALL provide OnePiece with browser sessions owned by the originating native session and generation, using isolated browser contexts with bounded lifetime, page count, storage, download, and event retention. A browser session SHALL NOT expose or attach to an unrelated user browser profile by default.

#### Scenario: Start a browser session
- **WHEN** OnePiece requests browser work and browser readiness passes
- **THEN** the system SHALL create or reuse only the bounded automation context owned by that OnePiece session according to the declared reuse policy

#### Scenario: Parent generation ends
- **WHEN** the owning generation is cancelled or its browser session expires
- **THEN** the system SHALL stop active automation, close owned pages and contexts, and finalize bounded evidence

### Requirement: Supported browser operations
The browser capability SHALL support bounded navigation, history navigation, semantic page inspection, element interaction, text entry, screenshot capture, JavaScript evaluation, and visible-content extraction. Every operation SHALL validate a versioned input schema and return stable page, frame, and action references instead of exposing raw Playwright objects.

#### Scenario: Inspect an interactive page
- **WHEN** OnePiece requests an inspection snapshot
- **THEN** the system SHALL return a bounded semantic representation of visible and interactive content with stable references usable by later actions

#### Scenario: Capture a screenshot
- **WHEN** OnePiece requests a viewport or permitted full-page screenshot
- **THEN** the system SHALL store the bounded image as an Artifact and return its Artifact reference and capture metadata

#### Scenario: Evaluate JavaScript
- **WHEN** an approved JavaScript evaluation runs in the active page
- **THEN** the system SHALL execute it only in that page context and return a serializable bounded result without granting host-process access

### Requirement: Guarded navigation and page-origin policy
Browser navigation SHALL permit only supported HTTP(S) targets that pass URL normalization, redirect revalidation, DNS/address safety checks, and current network policy. It SHALL reject local files, unsupported schemes, embedded credentials, blocked private or link-local destinations, and redirect chains that leave the allowed policy.

#### Scenario: Public HTTPS navigation
- **WHEN** a normalized public HTTPS URL passes current policy
- **THEN** the browser MAY navigate to it subject to the session's time, redirect, content, and download limits

#### Scenario: Redirect reaches a blocked address
- **WHEN** any redirect or resolved address targets a blocked local, private, link-local, metadata, or otherwise denied destination
- **THEN** the system SHALL stop navigation before disclosing content from that destination

#### Scenario: Page opens an extra window
- **WHEN** a page attempts to create a popup or additional tab beyond the declared page policy
- **THEN** the system SHALL block it or attach it as a bounded owned page according to policy and SHALL never leave it unmanaged

### Requirement: Risk-based browser action approval
Navigation and passive inspection SHALL follow their configured read-only policy, while JavaScript evaluation, file upload, download retention, credential or sensitive-text entry, form submission, and actions likely to mutate external state SHALL require the unified permission decision for their exact action and resource. Browser content SHALL NOT be allowed to approve its own requested action.

#### Scenario: Page asks the Agent to submit a form
- **WHEN** page content instructs OnePiece to perform an external-state mutation
- **THEN** the system SHALL treat that content as untrusted data and SHALL still require the applicable permission or approval

#### Scenario: Approval binds an action
- **WHEN** a browser action is approved
- **THEN** the approval SHALL bind the session, page origin, action category, safe target summary, and input hash and SHALL become stale when those facts change

### Requirement: Human handoff and resume
The system SHALL allow a user to take control of an owned browser page for authentication, CAPTCHA, consent, or other human-only interaction. Automation SHALL pause while control is handed off, the UI SHALL clearly identify the controlled session, and resumption SHALL require an explicit user action and a fresh page inspection.

#### Scenario: Agent requests human assistance
- **WHEN** automation reaches a human-only interaction
- **THEN** the system SHALL expose the owned page through an application-controlled handoff surface and pause subsequent automation actions

#### Scenario: User resumes automation
- **WHEN** the user explicitly returns control
- **THEN** prior element references SHALL be invalidated and OnePiece SHALL obtain a new inspection before performing another referenced action

### Requirement: Browser evidence and privacy boundaries
The system SHALL bound and redact browser progress, console, network, screenshot, extraction, and error data before persistence. Cookies, authorization headers, password-field values, unrestricted DOM, and browser profile secrets SHALL NOT be written to unified logs or ordinary chat messages.

#### Scenario: Page contains a password field
- **WHEN** inspection encounters a password or equivalent sensitive input
- **THEN** the returned semantic snapshot and persisted diagnostics SHALL omit its value

#### Scenario: Extraction is truncated
- **WHEN** extracted visible content exceeds its declared limit
- **THEN** the result SHALL explicitly report truncation and preserve source URL and capture metadata

