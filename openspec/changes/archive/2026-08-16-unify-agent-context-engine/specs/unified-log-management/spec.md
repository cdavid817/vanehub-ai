## ADDED Requirements

### Requirement: Context Engine diagnostics use an allowlisted metadata schema
Unified logging SHALL accept Context Engine lifecycle events only through the existing logging service after redaction, with allowlisted policy versions, correlations, source kinds, counts, safe fingerprints, score and latency buckets, token or character estimates, reason codes, and terminal outcomes.

#### Scenario: Source collection includes private content
- **WHEN** an evidence source reads source code, prompts, memory, tool output, credentials, headers, or environment values
- **THEN** persisted Context Engine diagnostics SHALL exclude those values and raw payloads
- **AND** the page-visible evidence manifest SHALL remain available through its service-owned projection
