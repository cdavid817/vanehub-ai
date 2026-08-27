## ADDED Requirements

### Requirement: Native screenshot regions SHALL enter OCR through the admitted-source boundary
The local-media runtime SHALL accept a successful native region capture as an operation-owned PNG source for the existing OCR workflow. It SHALL apply the same media type, byte, dimension, page, path, profile-revision, cancellation, and worker-supervision bounds as an image selected from disk, without exposing an arbitrary host path to the frontend.

#### Scenario: Captured region is recognized
- **WHEN** a valid region capture is handed to a ready OCR engine for the initiating composer scope
- **THEN** the runtime SHALL recognize the captured PNG and return the result through the existing editable OCR review flow

#### Scenario: Captured region exceeds OCR bounds
- **WHEN** the encoded region exceeds the configured image byte or pixel limit
- **THEN** OCR SHALL fail with the existing bounded-media error and the captured file SHALL be deleted

### Requirement: Screenshot OCR SHALL preserve review-before-append behavior
A screenshot OCR result SHALL NOT be appended or sent automatically. The user SHALL be able to edit, confirm, or cancel the recognized text under the same draft-length and active-scope rules as file-based OCR.

#### Scenario: User confirms screenshot text
- **WHEN** the user reviews and confirms recognized screenshot text
- **THEN** the text SHALL append to the latest draft for the same composer scope without sending it

#### Scenario: User cancels screenshot review
- **WHEN** the user cancels the OCR review
- **THEN** neither the recognized text nor screenshot pixels SHALL remain in the composer or temporary storage

