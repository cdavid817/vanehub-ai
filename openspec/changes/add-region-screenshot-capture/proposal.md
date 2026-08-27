## Why

The Local Media composer can recognize an existing image or PDF, but users cannot capture the part of the desktop they are discussing without leaving VaneHub. A native region-capture flow makes screenshot-to-OCR immediate while keeping pixels local and preserving the existing review-before-append contract.

## What Changes

- Add a desktop-only screenshot action beside the existing OCR, microphone, and speech controls in structured chat composers.
- Add a native capture flow that temporarily hides VaneHub, presents a full-screen translucent selection surface across the available desktop, and lets the user drag a rectangular region.
- Support keyboard cancellation with `Escape`, explicit cancellation, minimum selection bounds, HiDPI coordinate conversion, and multiple-monitor coordinates.
- Route the captured PNG through the existing bounded local OCR staging and review flow; never send, append, or persist recognized text without the existing explicit confirmation.
- Keep screenshot pixels, selected coordinates, display names, and full temporary paths out of unified logs, and delete operation-owned capture files on success, cancellation, failure, session change, and startup cleanup.
- Expose a truthful disabled/native-only screenshot action in Web mode without fabricating a capture.

## Capabilities

### New Capabilities

- `desktop-region-screenshot`: Native desktop capture lifecycle, region-selection interaction, coordinate safety, privacy, cancellation, and cleanup.

### Modified Capabilities

- `local-media-runtime`: Admit a native captured region as a bounded OCR source and preserve the existing OCR review and cleanup guarantees.
- `chat-experience`: Add the screenshot action to the structured composer with accessible state, native/Web availability, and non-disruptive cancellation behavior.

## Impact

- Desktop runtime: new screenshot capture application port, platform adapter, Tauri commands/events or a dedicated capture window, and platform permission/error mapping.
- Frontend: additive service contracts in both Tauri and Web adapters, a screenshot composer action, and a region-selection surface that uses existing Tailwind/UI primitives.
- Security and privacy: captured pixels remain local and operation-owned; diagnostics carry only stable outcome/reason/duration categories.
- Cross-platform behavior: Windows, macOS, and Linux require independent native verification; unsupported compositor or permission states fail safely with actionable localized errors.
- Existing OCR, microphone, TTS, send/stop controls, and single-Agent CLI terminal behavior remain unchanged.
