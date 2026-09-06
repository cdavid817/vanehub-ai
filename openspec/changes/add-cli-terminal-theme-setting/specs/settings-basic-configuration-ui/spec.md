## ADDED Requirements

### Requirement: CLI terminal theme control
The Basic Configuration common-preferences group SHALL offer a "CLI 会话主题" select directly below the application theme, with the localized options light and dark, saved through the shared settings provider.

#### Scenario: Render the control
- **WHEN** Basic Configuration renders common preferences
- **THEN** it SHALL show a labelled select for the CLI terminal theme with the current value and a description that it applies to single-Agent CLI terminals independently of the application theme and takes effect immediately
- **AND** while the light value is selected the row SHALL also show a short note that some CLIs paint their own colors
- **AND** the control SHALL be keyboard operable, focus visible, and disabled while its own save is in progress, without disabling unrelated controls

#### Scenario: Save through the provider
- **WHEN** the user picks another value
- **THEN** the page SHALL save `cliTerminalTheme` through the settings provider without calling a Tauri command directly
- **AND** a rejected save SHALL restore the previous selection and show the existing settings error

#### Scenario: Localize in every registered locale
- **WHEN** the control renders in any registered application locale
- **THEN** its label, options, description, and note SHALL come from that locale's resources
