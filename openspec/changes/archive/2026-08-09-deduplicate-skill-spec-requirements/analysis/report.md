# Duplicate Requirement Analysis

## Summary

- `agent-skill-injection`: one duplicate Requirement pair.
- `skill-management`: eight duplicate Requirement pairs.
- All nine pairs have identical normalized SHA-256 hashes, descriptions, normative statements, and Scenarios.
- The duplicates trace to Requirements introduced by `2026-08-02-harden-skill-management-reliability`; retaining the first main-spec instance preserves that archived intent.
- No semantic merge is required. The proposed operation deletes only the later byte-identical copies.

## Budget Review

| Capability | Lines | Approximate bytes | Default limit |
|---|---:|---:|---:|
| `agent-skill-injection` | 142 | 9,486 | 500 lines / 8,000 tokens |
| `skill-management` | 348 | 21,217 | 500 lines / 8,000 tokens |

Neither specification exceeds the default line budget; neither is estimated to exceed the token budget.

## Recommendation

Approve the exact-instance deletion in `analysis/diff.md`. Do not use an OpenSpec `REMOVED Requirements` delta because requirement-name addressing cannot distinguish the retained instance from its duplicate.
