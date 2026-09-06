## MODIFIED Requirements

### Requirement: Retrieval searches the shared host-level memory pool
The system SHALL answer Agent recall from the same host-level memory pool that memory injection draws from, filtered by the same eligibility rules injection applies — lifecycle, read policy, scope, and audience — evaluated by the trusted runtime for the calling turn's principal (agent, workspace, session mode). The recall tool's input schema SHALL still expose exactly `query` and `limit`; the principal SHALL be injected by the runtime and SHALL NOT be model-suppliable. Both retrieval paths SHALL run over the eligibility-filtered id set, and every hit SHALL be revalidated against the authoritative record's status and revision before it is returned. The user's owner-level management search SHALL remain full-pool and separate from Agent recall.

#### Scenario: Memory saved under a different agent is recallable
- **WHEN** the model invokes the recall tool from one agent's session and a memory saved under a different agent is global-scoped with an audience that admits the caller
- **THEN** the system SHALL consider that memory a recall candidate
- **AND** recall SHALL NOT return a strict subset of what memory injection already placed in the system prompt

#### Scenario: A workspace-scoped memory is recallable inside its workspace
- **WHEN** the model invokes recall from a session whose workspace matches a workspace-scoped memory the caller's audience admits
- **THEN** the system SHALL consider that memory a recall candidate
- **AND** the same memory SHALL NOT be a candidate from a session in another workspace

#### Scenario: An audience-excluded memory is not recallable by the excluded Agent
- **WHEN** a memory's audience names selected Agents and the calling Agent is not among them
- **THEN** recall for that Agent SHALL NOT return the memory
- **AND** recall for a named Agent under the same conditions SHALL be able to return it

#### Scenario: Recall tool exposes no scope parameter
- **WHEN** the recall tool definition is resolved
- **THEN** its input schema SHALL expose exactly `query` and `limit`
- **AND** the principal used for eligibility SHALL come from the trusted runtime, never from tool input

#### Scenario: Hits are revalidated against the authoritative store
- **WHEN** an index row matches the query but its authoritative record is archived, deleted, or at a different revision than the index knew
- **THEN** the system SHALL drop or refresh the hit from the authoritative record before returning
- **AND** SHALL NOT return content that only survives in a derived index
