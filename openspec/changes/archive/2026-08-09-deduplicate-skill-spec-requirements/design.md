## Context

See `proposal.md` and `analysis/`. OpenSpec strict validation rejects duplicate Requirement names, while its normal delta language addresses Requirements by name and cannot select only one of two invalid same-name instances.

## Goals / Non-Goals

**Goals:**
- Restore unique Requirement names with a mechanically reviewable, coverage-preserving edit.
- Preserve the earlier instance of each exact duplicate and all unique surrounding content.
- Prove that no normative statement or Scenario is lost.

**Non-Goals:**
- Rewording, regrouping, splitting, or semantically consolidating Requirements.
- Editing archived change artifacts.
- Changing application code or runtime behavior.

## Decisions

1. **Use exact-instance deletion rather than a name-addressed OpenSpec delta.** The later blocks are invalid duplicates, and a standard `REMOVED Requirements` operation cannot distinguish them from the retained blocks. The implementation will delete only the line regions identified in `analysis/diff.md`.
2. **Retain the first instance.** It preserves document order and the earliest main-spec placement while matching the archived source intent byte-for-byte. Retaining the later instance would provide no semantic benefit and would move requirements away from related material.
3. **Gate application on hashes and coverage.** Before deletion, recompute the hashes in `analysis/mapping.md`. After deletion, verify unique names, retained normative counts, affected-spec strict validation, and full-corpus strict validation.
4. **Do not modify archive history.** The 2026-08-02 archive remains the immutable source explaining where the requirements originated.

## Risks / Trade-offs

- [Risk] Line numbers drift before application. -> Recompute hashes and locate blocks by name plus identical content; stop if either copy differs.
- [Risk] A broad text deletion removes the wrong instance. -> Apply two narrowly scoped patches and verify retained hashes immediately.
- [Risk] Full validation reveals unrelated errors. -> Report them separately; do not expand this change beyond the two analyzed capabilities.

## Migration Plan

1. Verify duplicate hashes still match the analysis.
2. Delete the later duplicate block in `agent-skill-injection`.
3. Delete the contiguous later duplicate region in `skill-management`.
4. Run affected and full strict validation plus `git diff --check`.
5. Roll back only these two deletions if coverage or validation fails.
