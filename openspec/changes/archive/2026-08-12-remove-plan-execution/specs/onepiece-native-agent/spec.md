## REMOVED Requirements

### Requirement: Bounded OnePiece planning requests
**Reason**: Structured OnePiece Plan generation is removed with the Plan execution workflow.
**Migration**: Use ordinary OnePiece API sessions for conversational planning.

### Requirement: Attempt execution profile
**Reason**: Plan SubTask attempts no longer dispatch bounded OnePiece worker generations.
**Migration**: Retained Agent and Loop execution paths continue using their own execution profiles.

### Requirement: OnePiece credential reference isolation
**Reason**: The requirement specifically governed Plan planner and worker records that are no longer produced.
**Migration**: Existing credential-storage and redaction requirements continue to protect all retained OnePiece operations.
