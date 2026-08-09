# Normative Coverage

## Result

**PASS** for the proposed exact-instance deletion.

| Capability | Removed `SHALL` | Retained identical `SHALL` | Removed `MUST` | Retained identical `MUST` | Scenario coverage |
|---|---:|---:|---:|---:|---:|
| `agent-skill-injection` | 3 | 3 | 0 | 0 | 2/2 |
| `skill-management` | 31 | 31 | 0 | 0 | 15/15 |
| **Total** | **34** | **34** | **0** | **0** | **17/17** |

The retained and removed block hashes match for every row in `mapping.md`. Therefore every removed normative statement and Scenario has an exact retained target.

## Guardrail

Coverage fails if implementation changes any retained block content, removes the earlier instance, or deletes content outside the listed later-instance ranges.
