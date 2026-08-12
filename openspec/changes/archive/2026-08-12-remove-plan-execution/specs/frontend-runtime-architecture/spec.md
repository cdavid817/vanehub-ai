## REMOVED Requirements

### Requirement: Plan frontend service boundary
**Reason**: The Plan Center and all Plan frontend consumers are removed.
**Migration**: None. Other frontend feature modules retain their existing service boundaries.

### Requirement: Plan adapter contract parity
**Reason**: Neither desktop nor Web/mock exposes Plan operations after this change.
**Migration**: Remove both adapters together; no runtime may advertise simulated or native Plan execution.

### Requirement: Bounded Plan UI projections
**Reason**: PlanRun lists, details, polling, and on-demand Attempt evidence no longer have a UI surface.
**Migration**: Existing Plan data remains local and inert; no replacement projection is provided.
