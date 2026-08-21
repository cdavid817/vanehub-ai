## ADDED Requirements

### Requirement: Hot-path relationship reads are batched
Repository list operations MUST batch child relationship reads instead of issuing a fixed set of additional queries for every parent item.

#### Scenario: Load Agent registry
- **WHEN** the registry lists Agents with modes and capability tags
- **THEN** the repository SHALL load Agent rows and their relationships with a bounded number of queries independent of Agent count

#### Scenario: Load feedback for a message page
- **WHEN** a message page requests feedback summaries
- **THEN** the evidence repository SHALL use a bounded number of queries independent of message count

### Requirement: Shared registry locks exclude nested waits
An asynchronous registry lock SHALL be released before awaiting independently locked child state.

#### Scenario: Read connector health during replacement
- **WHEN** health is collected while a connector is being replaced or stopped
- **THEN** health collection SHALL NOT retain the connector registry read lock while awaiting connector state

