## ADDED Requirements

### Requirement: Run status observation is bounded by Run activity
A rendered Run status SHALL observe a Run only while that observation can still change what it displays. Observation SHALL stop once the Run reaches a terminal state, and SHALL NOT continue for an owner that has no Run and no work that could produce one. The total observation cost of a surface SHALL be governed by how many Runs are actually active on it rather than by how many history items it renders.

#### Scenario: Run reaches a terminal state
- **WHEN** an observed Run reaches a terminal state
- **THEN** the status SHALL stop issuing further observations for that Run
- **AND** it SHALL keep displaying that Run's terminal state and frozen elapsed duration

#### Scenario: Owner has no Run and no active work
- **WHEN** a status is rendered for an owner that has no Run and whose work is not active
- **THEN** it SHALL resolve the absence once and SHALL NOT issue repeated observations

#### Scenario: Run has not been created yet
- **WHEN** a status is rendered for an owner whose work is active but whose Run does not exist yet
- **THEN** it SHALL keep observing until the Run appears, so the status is not permanently blank

#### Scenario: Long history is rendered
- **WHEN** a surface renders many history items whose Runs have all reached terminal states
- **THEN** the number of ongoing observations SHALL NOT grow with the number of rendered items
