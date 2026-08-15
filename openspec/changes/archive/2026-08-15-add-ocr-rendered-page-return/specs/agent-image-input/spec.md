## MODIFIED Requirements

### Requirement: Tools that may return an image
The file tool's read operation SHALL return a reviewed image type as an image rather than refusing it as binary content, subject to the same workspace, hidden-path, and file-size rules it already applies. The Browser screenshot operation SHALL return the image it captured, and the OCR tool SHALL return the page it read, each alongside the Artifact reference they already return and, for OCR, alongside the extracted text. Where OCR rasterizes its source, the page it read is the rendered page, and OCR SHALL return it only when the call rendered exactly one page. No other tool SHALL return an image. Every image-returning tool SHALL prepare its image through the same reviewed-type, bound, downscale, and per-request-budget path rather than a tool-specific one. When the active model does not accept images, each of these tools SHALL return its existing non-image result rather than failing.

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
- **THEN** the result SHALL carry the page it read alongside the extracted text and Artifact reference

#### Scenario: OCR rasterizes a single page
- **WHEN** an OCR call succeeds over a source it rasterized into exactly one page
- **THEN** the result SHALL carry that rendered page rather than the source it was rendered from
- **AND** the rendered page SHALL be retained as an Artifact linked to that source

#### Scenario: OCR rasterizes more than one page
- **WHEN** an OCR call succeeds over a source it rasterized into more than one page
- **THEN** the result SHALL carry the extracted text and Artifact reference without an image
- **AND** it SHALL NOT fail because no single page could be chosen

#### Scenario: OCR of a source that is not a reviewed image type
- **WHEN** an OCR call succeeds over a source whose type the image path does not review and which it did not rasterize
- **THEN** the result SHALL carry the extracted text and Artifact reference without an image
- **AND** it SHALL NOT fail because an image could not be attached

#### Scenario: Retaining a rendered page fails
- **WHEN** an OCR call cannot retain the page it rendered
- **THEN** the result SHALL carry the extracted text and Artifact reference without an image
- **AND** it SHALL NOT fail because the page could not be retained

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
