## ADDED Requirements

### Requirement: Code intelligence performance evidence uses versioned repositories
Code intelligence benchmarks SHALL cover Tree-sitter incremental updates, workspace indexing, indexed search, and bounded LSP definition and reference queries using versioned small, medium, and large synthetic repository datasets.

#### Scenario: Code intelligence datasets are measured
- **WHEN** supported Tree-sitter, index, search, and LSP workloads run
- **THEN** results SHALL include bounded files, bytes, symbols, locations, queue work, and response items
- **AND** dedicated results SHALL include P50/P95 latency with commit, platform, profile, and dataset provenance

#### Scenario: LSP is unavailable or returns too many locations
- **WHEN** the server is unavailable, warming, timed out, or returns more than the supported cap
- **THEN** the benchmark SHALL preserve existing degradation semantics and deterministic response bounds without converting unavailable state into a false latency success

