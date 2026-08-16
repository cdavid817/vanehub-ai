## ADDED Requirements

### Requirement: Code review ownership across existing contexts
Native code review behavior SHALL remain in the existing modular-monolith contexts: `sessions` owns review records and feedback coordination, `workspaces` owns Git/path/fingerprint/revert policy, `operations` owns observable action lifecycle, and `permissions` owns destructive approval; cross-context calls SHALL use published APIs assembled in bootstrap.

#### Scenario: Review command executes
- **WHEN** a declared Tauri review command is invoked
- **THEN** the handler SHALL validate/map transport data and call assembled application services without SQL, Git process construction, or business policy in the command module

#### Scenario: Architecture fixtures inspect review code
- **WHEN** native architecture fitness tests scan the implementation
- **THEN** no review module SHALL import another context's private domain, repository, or infrastructure implementation
