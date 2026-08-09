# Proposed Diff

No main spec has been edited by this analysis.

## Exact Deletions

1. In `openspec/specs/agent-skill-injection/spec.md`, delete the later `Bounded Skill prompt assembly` block currently at lines 121-131.
2. In `openspec/specs/skill-management/spec.md`, delete the contiguous later duplicate region currently at lines 231-315, ending immediately before `Safe CLI Skill mount roots`.

## Expected Result

| Capability | Requirements before | Requirements after | Lines before | Lines after |
|---|---:|---:|---:|---:|
| `agent-skill-injection` | 9 | 8 | 142 | 131 |
| `skill-management` | 27 | 19 | 348 | 263 |

All retained text remains byte-for-byte unchanged. After application, run strict validation for both capabilities and the complete main-spec corpus.
