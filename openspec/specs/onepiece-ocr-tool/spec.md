# onepiece-ocr-tool Specification

## Purpose
Allows OnePiece to extract structured text from approved image and PDF Artifacts through a managed local OCR runtime with bounded, provenance-preserving results.
## Requirements
### Requirement: OCR accepts only admitted Artifact inputs
The OCR tool SHALL accept immutable Artifact ids for supported image or PDF media types and SHALL resolve them through the Artifact service. It SHALL NOT accept arbitrary host paths, provider-authored URLs, inline unbounded binary data, links, special files, or content whose stored hash no longer matches its Artifact metadata.

#### Scenario: Supported image Artifact
- **WHEN** OnePiece requests OCR for a supported, intact image Artifact within all limits
- **THEN** the system SHALL prepare a read-only local OCR input associated with that Artifact's provenance

#### Scenario: Arbitrary path is supplied
- **WHEN** a request supplies a host path instead of an Artifact id
- **THEN** the system SHALL reject it without reading the path

#### Scenario: Artifact integrity check fails
- **WHEN** stored bytes do not match the selected Artifact's content hash
- **THEN** OCR SHALL fail closed and SHALL not invoke the inference runtime

### Requirement: Managed local OCR readiness
The OCR tool SHALL be eligible only when the reviewed PaddleOCR framework is installed, enabled, healthy for inference, and compatible with the current OCR consumer protocol. Readiness checks SHALL remain separate from inference and SHALL NOT install, start, download, or contact a remote OCR service implicitly.

#### Scenario: OCR framework is ready
- **WHEN** a non-destructive readiness check confirms compatible local inference availability
- **THEN** OnePiece MAY receive the OCR tool according to its other eligibility rules

#### Scenario: Framework is absent or management-only
- **WHEN** PaddleOCR is missing, disabled, unhealthy, or exposes only the earlier management/self-test boundary
- **THEN** the OCR tool SHALL be unavailable with an actionable reason and SHALL not fall back to a hosted model

### Requirement: Bounded image and PDF processing
OCR SHALL enforce controller-owned limits for input bytes, dimensions, pixels, PDF pages, rendered page size, duration, memory, output blocks, output characters, and concurrent work. Caller inputs MAY select a smaller page range or lower limits but SHALL NOT exceed platform ceilings.

#### Scenario: PDF page selection is valid
- **WHEN** requested pages fall within the document and effective page limit
- **THEN** the system SHALL process only those pages and record their original page numbers

#### Scenario: Input exceeds a hard limit
- **WHEN** an image or PDF exceeds any admitted byte, pixel, page, rendering, time, or output limit
- **THEN** OCR SHALL stop safely and return an explicit limit outcome without claiming complete extraction

### Requirement: Structured OCR result and provenance
OCR SHALL return a versioned structured result containing source Artifact id and hash, engine identity/version, language configuration, page references, ordered text blocks, bounding geometry when available, confidence when reported by the engine, plain-text projection, truncation, duration, and safe warnings. Missing confidence SHALL remain unknown rather than being invented.

#### Scenario: OCR succeeds with layout data
- **WHEN** the engine returns admitted text and geometry
- **THEN** the system SHALL preserve their page/block order and source provenance in the result

#### Scenario: OCR emits no text
- **WHEN** inference succeeds but detects no text
- **THEN** the system SHALL return a successful empty result with provenance rather than fabricate text or classify it as a runtime failure

### Requirement: OCR outputs can become derived Artifacts
The user or OnePiece MAY publish admitted OCR text or structured JSON as an immutable derived Artifact. The derived Artifact SHALL link to the source Artifact and OCR operation, while the source bytes remain unchanged.

#### Scenario: Publish OCR text
- **WHEN** an OCR result is published
- **THEN** the Artifact service SHALL seal the bounded output with lineage to the source image/PDF hash and OCR operation id

### Requirement: OCR privacy and runtime honesty
OCR input bytes and extracted text SHALL remain local to the managed runtime and SHALL not be persisted in diagnostic logs. Web/mock SHALL not claim local OCR occurred and SHALL return deterministic fixtures or an explicit desktop/runtime-readiness result.

#### Scenario: OCR worker fails with input context
- **WHEN** the worker reports an error containing input text or a host path
- **THEN** the user-safe error and unified log SHALL retain only redacted category and safe operation metadata

#### Scenario: OCR requested in Web/mock mode
- **WHEN** no configured native OCR backend exists
- **THEN** Web/mock SHALL not process or upload the Artifact and SHALL report mock or unavailable status truthfully

### Requirement: OnePiece OCR SHALL reuse the shared local-media PaddleOCR runtime

OnePiece OCR SHALL delegate local OCR inference, engine profile/readiness, worker supervision, model loading, input admission/materialization, operation lifecycle, cancellation, error mapping, privacy, and cleanup to the published `local_media::api`. The repository SHALL have exactly one native owner for the PaddleOCR process/runtime.

#### Scenario: An implementation already exists before this change

* WHEN the repository already contains OnePiece-specific PaddleOCR process or model supervision
* THEN that implementation SHALL be migrated behind `local_media::api` or removed
* AND OnePiece SHALL retain its existing public tool behavior
* AND no duplicate PaddleOCR worker owner SHALL remain

#### Scenario: OnePiece OCR is specification-only before this change

* WHEN no prior runtime implementation exists
* THEN PaddleOCR SHALL be implemented once in `local_media`
* AND OnePiece SHALL add only its artifact-oriented adapter/controller integration

#### Scenario: Composer OCR and OnePiece OCR run concurrently

* WHEN both entry points submit OCR work
* THEN their requests SHALL be admitted through the same engine queue and profile revision policy
* AND the shared queue bound SHALL prevent unbounded process/model duplication

### Requirement: OnePiece OCR SHALL retain its managed-artifact trust boundary

Sharing the runtime SHALL NOT add an arbitrary host-path parameter to the OnePiece OCR tool. OnePiece inputs SHALL remain managed artifacts and SHALL be materialized/admitted into operation-owned storage before the shared worker receives them.

#### Scenario: The tool receives an admitted artifact

* WHEN the artifact satisfies the existing OnePiece OCR type/size/page/pixel policy
* THEN the OnePiece adapter SHALL request shared local-media admission and OCR
* AND the worker SHALL receive only the admitted canonical local path

#### Scenario: The tool receives an arbitrary path or URL

* WHEN a caller supplies a raw host path, URL, or unsupported reference outside the artifact schema
* THEN the request SHALL be rejected before worker execution
* AND sharing the composer runtime SHALL not weaken the tool schema

### Requirement: OnePiece OCR SHALL preserve structured output and provenance

The shared OCR result returned to OnePiece SHALL preserve the current capability's bounded structured page/line result, deterministic plain text, engine/model/profile provenance, warnings, and operation metadata.

#### Scenario: Shared OCR succeeds

* WHEN local-media returns a structured PaddleOCR result
* THEN the OnePiece adapter SHALL map it to the existing OnePiece OCR result contract
* AND it SHALL preserve page and reading order
* AND provenance SHALL identify the shared PaddleOCR engine and profile revision

#### Scenario: Shared OCR reports no text

* WHEN local-media returns `NO_TEXT_DETECTED`
* THEN OnePiece SHALL expose the existing no-text outcome rather than a generic worker failure

### Requirement: OnePiece OCR privacy and Web/mock behavior SHALL not regress

The integration SHALL preserve local-only execution, no implicit download/fallback, sensitive-content redaction, ephemeral cleanup, cancellation, and truthful Web/mock behavior required by the OnePiece OCR capability.

#### Scenario: The application runs in Web mode

* WHEN OnePiece OCR is requested without a native host
* THEN it SHALL report native/local OCR as unavailable according to the existing contract
* AND it SHALL not claim that PaddleOCR executed

#### Scenario: Shared worker fails

* WHEN PaddleOCR import/model/protocol/inference fails
* THEN OnePiece SHALL receive a stable mapped error with operation identity
* AND logs SHALL not include the artifact bytes or recognized text
