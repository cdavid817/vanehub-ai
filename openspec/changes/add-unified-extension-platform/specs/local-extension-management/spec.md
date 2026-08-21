## ADDED Requirements

### Requirement: Built-in local capabilities appear in the unified extension catalog

Existing OCR, ASR, TTS, or other allowlisted local capability extensions SHALL be projected into the unified Extensions workspace as built-in capability items with current install/enable/disable/health state and deep links/actions. Their existing allowlist, package/process, and lifecycle ownership SHALL remain unchanged.

#### Scenario: User views installed local capability

* WHEN a current OCR capability is installed
* THEN the unified Installed/Contributions views show its built-in source and current state without treating it as an external `.vhext` package

### Requirement: Unified operations delegate to existing local-extension services

Where enable, disable, install, repair, or uninstall actions are exposed from the unified UI, they SHALL delegate to current local-extension application APIs and stable operations. Extension Platform SHALL not duplicate binaries, process state, or persistence.

#### Scenario: User disables local speech extension

* WHEN disable is invoked from unified UI
* THEN current local-extension lifecycle performs the operation and unified state reflects its result
