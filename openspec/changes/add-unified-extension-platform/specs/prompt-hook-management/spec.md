## ADDED Requirements

### Requirement: Prompt Hooks are projected into the generalized Hook catalog

Published Prompt Hooks and their CLI bindings SHALL appear as a distinct PromptTemplate source in the generalized Hook catalog with stable linkage to the authoritative Prompt Hooks record, category, lifecycle state, binding, version, and trace. Their existing non-executable template rendering semantics SHALL remain unchanged.

#### Scenario: User inspects a Prompt Hook from unified Hooks

* WHEN a Prompt Hook row is selected
* THEN the UI shows its generalized event/source projection and deep-links to the authoritative Prompt Hooks editor

#### Scenario: Prompt text contains command syntax

* WHEN a Prompt Hook body contains shell, Python, or other executable-looking text
* THEN it remains rendered prompt content and is not executed by the generalized Hook runtime

### Requirement: Prompt Hook and lifecycle Hook identities remain distinct

The system SHALL not silently convert Prompt Hook category/phase semantics into executable lifecycle handler semantics. Any explicit migration or duplication SHALL use preview, new identity, source provenance, and event-specific validation.

#### Scenario: User duplicates a Prompt Hook into a lifecycle transform

* WHEN the workflow is supported and confirmed
* THEN a new lifecycle Hook definition is created with explicit event, handler kind, limits, and provenance while the original Prompt Hook remains unchanged
