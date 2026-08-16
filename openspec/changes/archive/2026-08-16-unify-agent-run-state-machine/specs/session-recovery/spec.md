## ADDED Requirements

### Requirement: Session recovery reconciles canonical Runs
Startup Session recovery SHALL reconcile its active execution claim with the canonical Run and owner recovery policy, SHALL clear false running state for non-resumable processes, and SHALL never replay destructive work.

#### Scenario: Orphan generation has no live handle
- **WHEN** startup finds an active Session claim and canonical Run without resumable runtime evidence
- **THEN** the Session and Run record an explicit interrupted outcome and no provider or tool action is replayed
