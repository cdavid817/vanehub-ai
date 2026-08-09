## 1. Analysis And Approval

- [x] 1.1 Identify every duplicate Requirement instance and its exact line range
- [x] 1.2 Map every duplicate Requirement and Scenario to a byte-identical retained target
- [x] 1.3 Verify 100% coverage for all removed `SHALL` and `MUST` statements
- [x] 1.4 Obtain review approval for `analysis/report.md`, `analysis/mapping.md`, `analysis/coverage.md`, and `analysis/diff.md`

## 2. Main Spec Repair

- [x] 2.1 Recompute duplicate hashes and stop if any mapped pair is no longer identical
- [x] 2.2 Remove the later duplicate `Bounded Skill prompt assembly` block from `agent-skill-injection`
- [x] 2.3 Remove the eight later duplicate Requirement blocks from `skill-management`

## 3. Verification

- [x] 3.1 Confirm every Requirement name is unique in both affected specifications
- [x] 3.2 Confirm retained hashes and normative coverage still match the approved mapping
- [x] 3.3 Run strict validation for both affected capabilities and `openspec validate --specs --strict`
- [x] 3.4 Run `openspec validate deduplicate-skill-spec-requirements --strict` and `git diff --check`
