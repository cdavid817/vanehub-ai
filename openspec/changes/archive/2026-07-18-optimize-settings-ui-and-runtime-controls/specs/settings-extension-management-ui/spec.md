## MODIFIED Requirements

### Requirement: Extension Capabilities settings page
The settings center SHALL provide a service-backed Extension Capabilities page as a lower advanced-capability navigation entry for managing OCR, ASR, and TTS frameworks.

#### Scenario: Navigate to Extension Capabilities
- **WHEN** the settings sidebar renders
- **THEN** it SHALL include a localized Extension Capabilities entry below the higher-frequency agent, skill, and IM management entries

#### Scenario: Display capability overview
- **WHEN** the Extension Capabilities page loads
- **THEN** it SHALL show localized summary counts and grouped OCR, ASR, and TTS framework cards from the extension service rather than hard-coded page data

#### Scenario: Search extensions
- **WHEN** the user enters a settings search term on the Extension Capabilities page
- **THEN** the visible framework cards SHALL be filtered by localized capability, framework, description, requirement, and status text
