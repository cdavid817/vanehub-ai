## ADDED Requirements

### Requirement: The native context map SHALL register `local_media` as a peer bounded context

The canonical native bounded-context map and filesystem architecture SHALL include `local_media` with ownership of local OCR, microphone capture and whole-utterance transcription, speech synthesis/playback, local-media profiles/readiness, worker supervision, and ephemeral media lifecycle.

#### Scenario: Architecture checks enumerate contexts

* WHEN architecture validation compares `src-tauri/src/contexts` with the canonical context map
* THEN `local_media` SHALL appear in both
* AND no undocumented duplicate media runtime context SHALL be introduced

#### Scenario: Context ownership is reviewed

* WHEN a local-media responsibility is implemented
* THEN engine configuration, workers, audio coordination, admission, and cleanup SHALL belong to `local_media`
* AND generic operation records/cancellation routing SHALL remain owned by the operations context
* AND window/tray/shell lifecycle SHALL remain owned by desktop
* AND CLI/MCP/plugin/skill concerns SHALL remain owned by tooling

### Requirement: `local_media` SHALL publish a narrow API and preserve dependency direction

Other contexts SHALL depend only on `local_media::api` domain/application contracts. `local_media` MAY use published operation, artifact, persistence, and observability interfaces according to the existing dependency policy, but cross-context infrastructure imports SHALL be prohibited.

#### Scenario: OnePiece integrates shared OCR

* WHEN OnePiece submits an OCR artifact
* THEN it SHALL call a published local-media API method
* AND it SHALL not import the worker supervisor, Python process wrapper, profile repository, audio ports, or temp store

#### Scenario: Local media records an operation

* WHEN a probe or inference task starts
* THEN local-media application code SHALL call the published operation API
* AND it SHALL not write another context's database tables directly

### Requirement: Native local-media commands SHALL remain thin adapters

Tauri commands SHALL deserialize DTOs, call one local-media application use case, and map results/errors. Commands SHALL not contain inference, model, process, file admission, recording, playback, cleanup, or operation persistence policy.

#### Scenario: A recording command is invoked

* WHEN `start_microphone_recording` is called
* THEN the command SHALL delegate to the recording application service
* AND native device opening and sample handling SHALL occur behind context ports/infrastructure

#### Scenario: A command failure occurs

* WHEN the application service returns a domain error
* THEN the command SHALL map it to the stable frontend error DTO
* AND it SHALL not expose an internal traceback or full path

### Requirement: Long-running media operations SHALL follow the host task lifecycle

Local-media probes and inference SHALL produce stable task/operation IDs, observable phases, cancellation, terminal results, and redacted diagnostics consistent with the existing native runtime architecture.

#### Scenario: A local-media operation is accepted

* WHEN the application service admits a long-running task
* THEN a stable operation ID SHALL be returned immediately
* AND status SHALL remain queryable independently of the initiating command lifetime

#### Scenario: The application shuts down

* WHEN host shutdown begins
* THEN active local-media recording/playback SHALL stop
* AND workers SHALL receive bounded shutdown/termination
* AND operation-owned cleanup SHALL run according to host shutdown policy
