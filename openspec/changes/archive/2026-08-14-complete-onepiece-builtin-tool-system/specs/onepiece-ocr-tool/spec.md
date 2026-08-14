## Purpose

Allows OnePiece to extract structured text from approved image and PDF Artifacts through a managed local OCR runtime with bounded, provenance-preserving results.

## ADDED Requirements

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

