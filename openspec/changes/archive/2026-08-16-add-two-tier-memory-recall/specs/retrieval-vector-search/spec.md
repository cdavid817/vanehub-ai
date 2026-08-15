## MODIFIED Requirements

### Requirement: Retrieval tool is registered only when configured

The system SHALL offer the recall tool to the model only when an embedding source is configured. Memory injection SHALL NOT depend on that configuration: with no embedding source configured, the memory index SHALL still be injected and relevance selection SHALL still run, so that an installation without retrieval keeps a working memory feature.

#### Scenario: No embedding configured

- **WHEN** no embedding source is configured
- **THEN** the recall tool SHALL NOT appear in the tool catalog
- **AND**, replacing this scenario's previous guarantee that recency-based memory injection continues unchanged, index injection and relevance-selected body injection SHALL both continue to operate

#### Scenario: Embedding configured

- **WHEN** an embedding source is configured
- **THEN** the recall tool SHALL appear in the tool catalog
- **AND** it SHALL remain the content-driven search path, complementary to the description-driven relevance selection rather than replaced by it
