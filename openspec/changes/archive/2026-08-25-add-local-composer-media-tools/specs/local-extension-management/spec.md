## REMOVED Requirements

### Requirement: Managed PaddleOCR inference consumer

**Reason**: The PaddleOCR inference runtime moves to `local_media`, which becomes the single production owner of the inference process, model lifecycle, readiness, cancellation, timeouts, results, and resource limits. Keeping this requirement would mandate a second PaddleOCR inference owner in a context that no longer has one, and its implementation is deleted by this change.

**Migration**: OnePiece OCR reaches PaddleOCR through `local_media::api` (see the `onepiece-ocr-tool` delta in this change). No caller of the extension service loses a capability: the extension service never exposed OCR to the frontend, and its only native consumer was the OnePiece OCR adapter, which is re-pointed at `local_media`.

### Requirement: OCR inference remains local and bounded

**Reason**: This requirement constrains the inference boundary removed above. The same guarantees -- no remote transmission, no unrestricted environment inheritance, no raw inference content in durable logs, plus cancellation, duration, input, and output limits with descendant cleanup -- are restated against the owning context in the `local-media-runtime` capability, so removing it here narrows ownership rather than dropping the protection.

**Migration**: The guarantees are enforced by `local_media` and are covered by that capability's requirements and by the worker bridge's tests.

## ADDED Requirements

### Requirement: Local extension management SHALL own dependency installation and health only

The local-extension capability SHALL own the framework catalog, managed Python environment creation, package installation and version inventory, enablement state, and a loopback management sidecar used solely as a liveness probe. It SHALL NOT own an inference runtime, inference readiness, model lifecycle, or an inference worker for any framework it lists.

#### Scenario: A catalog entry describes an external dependency

* WHEN the catalog lists a framework such as PaddleOCR, faster-whisper, or sherpa-onnx
* THEN the entry SHALL describe the external packages, version range, disk estimate, and model requirement
* AND the entry SHALL NOT advertise an inference protocol, inference readiness, or an inference worker
* AND the presence of the entry SHALL NOT make the framework usable for inference

#### Scenario: A consumer needs local inference

* WHEN a native consumer needs OCR, transcription, or speech synthesis
* THEN it SHALL obtain them from `local_media`, which owns the engine profile, worker supervision, and readiness
* AND it SHALL NOT start, probe, or resolve an inference process through the local-extension service

#### Scenario: The management sidecar is running

* WHEN the loopback management sidecar reports healthy
* THEN the system SHALL treat that as installation liveness only
* AND it SHALL NOT report the framework as ready for inference on that basis

### Requirement: A framework entry SHALL NOT be implemented by a stand-in process

A catalog entry named after a real inference framework SHALL NOT present a stand-in process as that framework's implementation. A liveness probe MAY be a generic process, provided the system does not describe it as running the named framework or derive inference readiness from it.

#### Scenario: A liveness probe stands in for the framework

* WHEN the managed environment starts a generic process such as a static file server for health purposes
* THEN the system SHALL report only that the managed environment is installed and responsive
* AND it SHALL NOT claim the named framework is running, ready, or serving inference

#### Scenario: Ownership is reviewed in the repository

* WHEN architecture validation inspects production sources
* THEN PaddleOCR SHALL be imported and constructed only by the local-media worker
* AND the Agent runtime SHALL reach PaddleOCR only through `local_media::api`
* AND the local-extension context SHALL name no inference protocol or inference port
* AND no stand-in process SHALL appear on a code path that names an inference framework
