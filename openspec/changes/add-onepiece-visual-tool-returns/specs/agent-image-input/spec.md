## MODIFIED Requirements

### Requirement: Tools that may return an image
The file tool's read operation SHALL return a reviewed image type as an image rather than refusing it as binary content, subject to the same workspace, hidden-path, and file-size rules it already applies. The Browser screenshot operation and the OCR tool SHALL return their produced image alongside the Artifact reference they already return and, for OCR, alongside the extracted text. No other tool SHALL return an image. Every image-returning tool SHALL prepare its image through the same reviewed-type, bound, downscale, and per-request-budget path rather than a tool-specific one. When the active model does not accept images, each of these tools SHALL return its existing non-image result rather than failing.

#### Scenario: File read of a reviewed image type
- **WHEN** the native agent reads a file whose type is a reviewed image type
- **THEN** the system SHALL return it as an image instead of refusing it as binary content
- **AND** the existing workspace, hidden-path, and size rules SHALL still apply

#### Scenario: File read of a non-image binary
- **WHEN** the native agent reads a binary file that is not a reviewed image type
- **THEN** the system SHALL continue to refuse it with an explicit reason

#### Scenario: Screenshot returns what it captured
- **WHEN** the Browser screenshot operation succeeds and the active model accepts images
- **THEN** the result SHALL carry the captured image alongside its Artifact reference

#### Scenario: OCR returns the page it read
- **WHEN** an OCR call succeeds and the active model accepts images
- **THEN** the result SHALL carry the rendered page image alongside the extracted text and Artifact reference

#### Scenario: A produced image exceeds its bounds
- **WHEN** a screenshot or rendered page exceeds the declared dimension or byte bounds
- **THEN** the system SHALL apply the same downscale-then-refuse behavior it applies to a file read
- **AND** it SHALL NOT send an image that bypassed those bounds

#### Scenario: File read on a text-only model
- **WHEN** the active model does not accept images and the native agent reads a reviewed image type
- **THEN** the file tool SHALL return its existing non-image result
- **AND** it SHALL NOT fail because an image could not be attached

#### Scenario: Screenshot or OCR on a text-only model
- **WHEN** the active model does not accept images and a screenshot or OCR call produces an image
- **THEN** the tool SHALL return its existing Artifact reference and, for OCR, its extracted text
- **AND** it SHALL NOT fail because an image could not be attached

#### Scenario: Per-request budget spans every producer
- **WHEN** file reads, screenshots, and OCR calls together would carry more images than the declared per-request maximum
- **THEN** the system SHALL refuse the calls past that maximum rather than dropping images silently

### Requirement: Image redaction and transport
Durable logs SHALL contain only an image's hash, media type, dimensions, and byte count; they SHALL NOT contain image bytes or any encoding of them. Images passed between tools and turns SHALL be referenced by Artifact id rather than by host filesystem path. The persisted transcript SHALL carry an Artifact reference rather than embedding image bytes or any encoding of them in message text.

#### Scenario: Image-bearing request is logged
- **WHEN** the system logs a generation that carried an image
- **THEN** the durable log SHALL contain the image's hash, media type, dimensions, and byte count only

#### Scenario: Tool-to-tool image transfer
- **WHEN** one tool produces an image another consumes
- **THEN** the transfer SHALL use the image's Artifact id rather than a host path

#### Scenario: Transcript persistence
- **WHEN** a message carrying an image is persisted
- **THEN** the stored message SHALL reference the image's Artifact rather than embedding its bytes

## ADDED Requirements

### Requirement: Image sources are never caller-addressed
An image a tool returns SHALL come from content that tool itself produced in the same call: bytes it already holds, or a file inside its own sandbox workspace. No tool SHALL attach an image identified by a caller-supplied Artifact id, host path, or any other addressable reference. Where a tool resolves stored content to attach it, that resolution SHALL verify the stored content hash before returning bytes and SHALL NOT expose a host filesystem path.

#### Scenario: A produced image is attached
- **WHEN** a tool produces an image during a call and the active model accepts images
- **THEN** it SHALL attach that image from the content it produced

#### Scenario: A caller-supplied reference is refused
- **WHEN** a tool call supplies an Artifact id, path, or other reference naming an image to attach
- **THEN** the system SHALL reject it rather than resolving and attaching it

#### Scenario: Integrity failure while resolving stored content
- **WHEN** a tool resolves stored content whose hash does not match its bytes
- **THEN** the resolution SHALL fail rather than returning content that failed verification

#### Scenario: Host path is never exposed
- **WHEN** a tool returns an image
- **THEN** the result SHALL NOT contain a host filesystem path for it
