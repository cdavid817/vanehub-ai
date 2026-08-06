## ADDED Requirements

### Requirement: Deleting a memory revokes its retrieval index
The system SHALL revoke the retrieval index entry for a deleted memory, and SHALL NOT return deleted memories from retrieval even if an index entry survives.

#### Scenario: Memory deleted while index revocation fails
- **WHEN** a memory is deleted and its index revocation call fails
- **THEN** retrieval SHALL NOT return that memory, because results are resolved against the source table
- **AND** background reconciliation SHALL remove the orphaned index entry
