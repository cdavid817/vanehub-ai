## ADDED Requirements

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
