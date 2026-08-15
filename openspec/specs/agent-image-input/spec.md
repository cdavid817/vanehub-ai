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
Durable logs SHALL contain only an image's hash, media type, dimensions, and byte count; they SHALL NOT contain image bytes or any encoding of them. The persisted transcript SHALL NOT embed image bytes or any encoding of them in message text.

#### Scenario: Image-bearing request is logged
- **WHEN** the system logs a generation that carried an image
- **THEN** the durable log SHALL contain the image's hash, media type, dimensions, and byte count only

#### Scenario: Transcript persistence
- **WHEN** a message carrying an image is persisted
- **THEN** the stored message SHALL describe the image rather than embedding its bytes

### Requirement: Tools that may return an image
The file tool's read operation SHALL return a reviewed image type as an image rather than refusing it as binary content, subject to the same workspace, hidden-path, and file-size rules it already applies. No other tool SHALL return an image. When the active model does not accept images, the file tool SHALL return its existing non-image result rather than failing.

#### Scenario: File read of a reviewed image type
- **WHEN** the native agent reads a file whose type is a reviewed image type
- **THEN** the system SHALL return it as an image instead of refusing it as binary content
- **AND** the existing workspace, hidden-path, and size rules SHALL still apply

#### Scenario: File read of a non-image binary
- **WHEN** the native agent reads a binary file that is not a reviewed image type
- **THEN** the system SHALL continue to refuse it with an explicit reason

#### Scenario: File read on a text-only model
- **WHEN** the active model does not accept images and the native agent reads a reviewed image type
- **THEN** the file tool SHALL return its existing non-image result
- **AND** it SHALL NOT fail because an image could not be attached

### Requirement: Image token accounting
Usage accounting SHALL attribute the provider-reported token cost of image-bearing requests through the existing invocation accounting contract, and SHALL NOT estimate image tokens from character counts.

#### Scenario: Provider reports usage for an image request
- **WHEN** an image-bearing generation completes with valid provider usage
- **THEN** the reported totals SHALL be attributed to that invocation unchanged

#### Scenario: Provider omits usage for an image request
- **WHEN** an image-bearing generation completes without valid provider usage
- **THEN** the runtime SHALL expose reduced reported coverage rather than estimating the image's cost from text length

