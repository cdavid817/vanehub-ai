## ADDED Requirements

### Requirement: Managed PaddleOCR inference consumer
The local-extension service SHALL expose a versioned, bounded PaddleOCR inference operation for authorized native consumers after the installed framework passes capability-specific inference readiness. This operation SHALL run through a backend-owned local worker boundary, SHALL accept only native-service-resolved inputs, and SHALL remain separate from the existing loopback management sidecar whose contract continues to provide lifecycle and health only.

#### Scenario: OnePiece OCR invokes a ready framework
- **WHEN** the OnePiece OCR service submits an admitted local input and PaddleOCR inference readiness passes
- **THEN** the extension service SHALL run the reviewed local inference plan and return a bounded structured engine result

#### Scenario: Management sidecar is healthy but inference is unavailable
- **WHEN** the existing health sidecar is running but the framework cannot satisfy the versioned inference protocol
- **THEN** inference readiness SHALL remain false and the service SHALL not claim OCR request availability

#### Scenario: Untrusted caller supplies a process plan
- **WHEN** a caller attempts to choose an executable, module, package, environment, endpoint, or arbitrary argument outside the backend-owned inference contract
- **THEN** the service SHALL reject the request without launching it

### Requirement: OCR inference remains local and bounded
PaddleOCR inference SHALL not send input or extracted text to a remote service, bind a non-loopback listener, inherit unrestricted environment variables, or persist raw inference content in unified logs. The worker SHALL enforce cancellation, duration, memory/process, input, and output limits and SHALL clean up owned descendants.

#### Scenario: Inference is cancelled
- **WHEN** the originating OCR tool call is cancelled
- **THEN** the extension service SHALL stop the owned inference work and return a cancelled outcome without continuing in the background

#### Scenario: Worker emits sensitive diagnostics
- **WHEN** PaddleOCR stderr includes input text, local paths, or environment values
- **THEN** durable logs SHALL retain only redacted safe diagnostics and stable error categories

