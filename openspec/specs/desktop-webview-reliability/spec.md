# Desktop WebView Reliability

## Purpose

Define how VaneHub keeps its desktop and browser surfaces observable, recoverable, and usable when native WebView processes or frontend bootstrap operations fail.

## Requirements

### Requirement: Desktop startup remains continuously visible
The desktop runtime SHALL delay the main window's first presentation until its opaque branded startup document is ready, then keep that startup surface visible until the selected React surface has rendered visible application content. The startup surface SHALL use a light neutral branded background rather than a black frame, SHALL contain the VaneHub AI icon, an activity indicator, and `Starting...`, and SHALL NOT expose an intermediate blank, transparent, or unrelated-color frame.

#### Scenario: Native window is created before frontend assets load
- **WHEN** the native main window is created before the frontend document and styles have loaded
- **THEN** the runtime SHALL keep the main window hidden until the startup document finishes loading
- **AND** the native window and first frontend document backgrounds SHALL match the opaque startup surface background

#### Scenario: Settings and localization are still hydrating
- **WHEN** React has started but the application root has not rendered visible surface content because settings, theme, localization, or initial routing is still loading
- **THEN** the branded startup surface SHALL remain visible
- **AND** frontend readiness SHALL remain `starting`

#### Scenario: Application content becomes visible
- **WHEN** the selected React surface renders its first application element into the application root
- **THEN** the startup surface SHALL be removed
- **AND** frontend readiness SHALL transition to `ready` without an intermediate blank or unrelated-color frame

### Requirement: Native WebView failures are observable
The Windows desktop runtime SHALL observe WebView2 process failures for the main application WebView and SHALL write a redacted unified diagnostic containing the normalized failure kind and selected recovery action.

#### Scenario: Main WebView process failure
- **WHEN** WebView2 reports a process failure for the main application WebView
- **THEN** the native runtime SHALL write the failure through the unified logging service
- **AND** it SHALL NOT create a feature-local log file or expose sensitive process arguments

#### Scenario: Browser Web/mock mode
- **WHEN** VaneHub runs through the browser Web/mock adapter
- **THEN** native WebView process observation SHALL NOT be required
- **AND** the existing Web/mock behavior SHALL remain usable without a desktop API

### Requirement: Fatal main-surface failures recover automatically
The Windows desktop runtime SHALL recover failures that can leave the main application surface permanently blank while avoiding disruptive recovery for process kinds that WebView2 recovers automatically.

#### Scenario: Main-frame renderer exits
- **WHEN** WebView2 reports that the main-frame renderer exited unexpectedly
- **THEN** the runtime SHALL reload the main WebView
- **AND** it SHALL restart the application if the reload request fails

#### Scenario: Browser process exits
- **WHEN** WebView2 reports that the browser process exited unexpectedly
- **THEN** the runtime SHALL restart the desktop application because the existing WebView cannot be reused

#### Scenario: Renderer is repeatedly unresponsive
- **WHEN** WebView2 reports main-frame renderer unresponsiveness twice within 45 seconds
- **THEN** the runtime SHALL reload the main WebView
- **AND** an isolated first report SHALL only be recorded

#### Scenario: Auto-recoverable process exits
- **WHEN** WebView2 reports a GPU, utility, sandbox helper, plugin, subframe, or unknown process exit
- **THEN** the runtime SHALL record the failure without reloading or restarting the main application surface

### Requirement: Page switching preserves a recoverable terminal surface
The workspace SHALL preserve the mounted active agent terminal across retained session-page switches so that application restoration or WebView reload can reattach to the retained native terminal without starting a duplicate agent process.

#### Scenario: Switch away from and back to the active terminal page
- **WHEN** a user visits another retained session page and returns to the agent terminal page
- **THEN** the terminal panel SHALL remain available and be refitted after activation
- **AND** the runtime SHALL reuse or reattach to the retained terminal session rather than intentionally starting a duplicate process

#### Scenario: Restore the desktop application after page switching
- **WHEN** a user switches session pages, leaves the Windows application, and restores it
- **THEN** the main surface SHALL remain rendered or recover automatically from a reported fatal WebView failure
- **AND** the navigation shell SHALL remain usable after recovery

### Requirement: Frontend bootstrap failures remain recoverable
Every frontend surface entry point SHALL handle failures that occur before the React application mounts and SHALL present a visible recovery action instead of leaving an empty application root.

#### Scenario: Surface bootstrap rejects before mount
- **WHEN** loading or rendering the selected main or floating frontend surface rejects before the application mounts
- **THEN** the application root SHALL contain a visible localized recovery surface
- **AND** the surface SHALL provide an action to retry the current entry point

#### Scenario: User retries frontend bootstrap
- **WHEN** the user activates the bootstrap recovery action
- **THEN** the runtime SHALL reload the current entry point
- **AND** the navigation surface SHALL become usable if the required frontend modules load successfully

#### Scenario: Bootstrap diagnostics are unavailable
- **WHEN** frontend bootstrap fails and the diagnostic service cannot be loaded or invoked
- **THEN** the recovery surface SHALL remain visible and usable
- **AND** diagnostic reporting failure SHALL NOT replace or suppress the recovery action

#### Scenario: Bootstrap fails in Web/mock mode
- **WHEN** frontend bootstrap fails while VaneHub runs through the browser Web/mock adapter
- **THEN** the same recovery surface SHALL be available without a desktop API
