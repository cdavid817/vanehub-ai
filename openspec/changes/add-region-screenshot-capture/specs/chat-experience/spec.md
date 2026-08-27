## ADDED Requirements

### Requirement: Structured composers SHALL expose an accessible screenshot action
Desktop structured chat composers SHALL show a fixed-size screenshot action beside OCR, microphone, and speech actions. The action SHALL have a localized accessible name and tooltip, SHALL expose capturing/busy state without moving the toolbar, and SHALL preserve existing keyboard order, send/stop behavior, and narrow-width layout.

#### Scenario: Screenshot capture is available
- **WHEN** the desktop runtime supports capture, an active structured composer exists, and OCR is ready
- **THEN** the screenshot action SHALL be enabled and activating it SHALL open region selection

#### Scenario: OCR is not ready
- **WHEN** desktop capture is supported but the OCR engine is not ready
- **THEN** the screenshot action SHALL remain visible and disabled with guidance to configure Local Media

### Requirement: Web mode SHALL represent screenshot capture truthfully
The Web adapter SHALL NOT attempt host screen capture or fabricate a captured image. Web structured composers SHALL keep the screenshot action visible and disabled with a native-only explanation so layout and discoverability remain consistent.

#### Scenario: Composer runs in a browser
- **WHEN** the same structured composer renders through the Web adapter
- **THEN** the screenshot action SHALL be disabled and identify the feature as desktop-only

