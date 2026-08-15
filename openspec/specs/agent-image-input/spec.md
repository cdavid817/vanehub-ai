# agent-image-input Specification

## Purpose
TBD - created by archiving change add-agent-image-input. Update Purpose after archive.
## Requirements
### Requirement: Image content blocks in provider requests
The system SHALL support carrying images to the provider as image content blocks within a user turn or a tool result, translated into the shape required by the session's `interface_format`. Only reviewed image media types SHALL be sent. An image that cannot be decoded, whose declared media type does not match its content, or whose type is not reviewed SHALL be refused rather than forwarded.

#### Scenario: Image reaches an Anthropic-format request
- **WHEN** a generation includes an image and the session's `interface_format` is `anthropic`
- **THEN** the request SHALL carry it as that format's image content block

#### Scenario: Image reaches an OpenAI-compatible request
- **WHEN** a generation includes an image and the session's `interface_format` is `openai-compatible`
- **THEN** the request SHALL carry it as that format's image content shape

#### Scenario: Declared type does not match content
- **WHEN** an image's declared media type disagrees with its decoded content
- **THEN** the system SHALL refuse it with an explicit error rather than forwarding it

#### Scenario: Unreviewed image type
- **WHEN** an image's media type is outside the reviewed set
- **THEN** the system SHALL refuse it without attempting to convert it

### Requirement: Model capability gating
The system SHALL send images only when the active Profile's model is known to accept image input. When it is not, the system SHALL report that clearly at the point of use and SHALL NOT send an image-bearing request that the provider would reject. Capability SHALL come from reviewed model metadata and SHALL NOT be inferred from an unknown model identifier.

#### Scenario: Model accepts images
- **WHEN** the active Profile's model is reviewed as accepting image input
- **THEN** image-bearing tool results and turns SHALL be sent as image blocks

#### Scenario: Model does not accept images
- **WHEN** the active Profile's model is not reviewed as accepting image input
- **THEN** the system SHALL report the limitation at the point of use
- **AND** it SHALL NOT send an image block to that provider

#### Scenario: Unknown model identifier
- **WHEN** the active Profile's model is absent from reviewed metadata
- **THEN** the system SHALL treat image input as unsupported rather than assuming support

### Requirement: Image bounds
The system SHALL enforce declared maximums for an image's pixel dimensions, its encoded byte size, and the number of images in one request. An image exceeding the dimension bound SHALL be downscaled before sending; an image exceeding the byte bound after downscaling SHALL be refused. A request exceeding the per-request image count SHALL be refused rather than silently dropping images.

#### Scenario: Oversized image is downscaled
- **WHEN** an image exceeds the declared pixel dimension bound
- **THEN** the system SHALL downscale it before sending
- **AND** the result SHALL state that downscaling occurred

#### Scenario: Image is still too large after downscaling
- **WHEN** a downscaled image still exceeds the encoded byte bound
- **THEN** the system SHALL refuse it with an explicit error

#### Scenario: Too many images in one request
- **WHEN** a request would carry more images than the declared per-request maximum
- **THEN** the system SHALL refuse it with an explicit error rather than dropping images silently

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

### Requirement: Image token accounting
Usage accounting SHALL attribute the provider-reported token cost of image-bearing requests through the existing invocation accounting contract, and SHALL NOT estimate image tokens from character counts.

#### Scenario: Provider reports usage for an image request
- **WHEN** an image-bearing generation completes with valid provider usage
- **THEN** the reported totals SHALL be attributed to that invocation unchanged

#### Scenario: Provider omits usage for an image request
- **WHEN** an image-bearing generation completes without valid provider usage
- **THEN** the runtime SHALL expose reduced reported coverage rather than estimating the image's cost from text length

### Requirement: Image sources are never caller-addressed
An image a tool returns SHALL come from content that tool itself produced in the same call: bytes it already holds, or a file inside its own sandbox workspace. No tool SHALL attach an image identified by a caller-supplied Artifact id, host path, or any other addressable reference. Where a tool resolves stored content to attach it, that resolution SHALL verify the stored content hash before returning bytes and SHALL NOT expose a host filesystem path.

#### Scenario: A produced image is attached
- **WHEN** a tool produces an image during a call and the active model accepts images
- **THEN** it SHALL attach that image from the content it produced

#### Scenario: Stored content contradicts its declared type
- **WHEN** content is sealed whose bytes do not match its declared media type
- **THEN** the store SHALL refuse to seal it rather than admitting content a later read would trust

#### Scenario: A caller-supplied reference is refused
- **WHEN** a tool call supplies an Artifact id, path, or other reference naming an image to attach
- **THEN** the system SHALL reject it rather than resolving and attaching it

#### Scenario: Integrity failure while resolving stored content
- **WHEN** a tool resolves stored content whose hash does not match its bytes
- **THEN** the resolution SHALL fail rather than returning content that failed verification

#### Scenario: Host path is never exposed
- **WHEN** a tool returns an image
- **THEN** the result SHALL NOT contain a host filesystem path for it

