# Manage Skills

**Status: Delivered — desktop and Web/mock UI.** Desktop performs local persistence, CLI Skill mounting, and API Agent prompt binding. Web/mock only simulates the same UI and state transitions; it does not change local files or runtime configuration.

## Understand views and states

Enablement and Agent assignment are two independent dimensions.

| View or state | Meaning |
| --- | --- |
| All Skills | Every Skill in the global Skill catalog, regardless of enablement or assignment |
| Unassigned | Skills assigned to no CLI Agent or API Agent; enablement does not affect this classification |
| Agent page | Manages the selected Agent's Assigned and Available Skills |
| Enabled | A Skill-wide master switch that allows Agents already assigned that Skill to use it |
| Assigned | A relationship between the Skill and one specific Agent; each Agent is assigned independently |

“Enabled” does not mean “assigned to every CLI.” Enabling a Skill that has no assignments does not make any Agent use it and does not remove it from Unassigned.

Disabling a Skill pauses it for every assigned Agent without deleting those assignments. When the Skill is enabled again, only the previously assigned Agents resume using it; unassigned Agents remain unaffected.

## Typical outcomes

| Configuration | Result |
| --- | --- |
| Skill A is enabled but unassigned | No Agent uses it, and it remains in Unassigned |
| Skill B is enabled and assigned only to Codex | Codex can use it; other Agents cannot |
| Skill C is disabled and assigned to Codex and Claude | Both Agents pause it, but their assignments remain |
| Skill C is enabled again | Codex and Claude resume using it; other Agents do not receive it automatically |

## Enable and assign a Skill

1. Open Skills in Settings.
2. Find the Skill under All Skills and make sure Enabled is on.
3. Select a CLI Agent or API Agent in the left navigation.
4. Find the Skill in Available and choose Assign.
5. Confirm that the Skill moves to Assigned for the selected Agent.

If several Agents need the same Skill, open each Agent page and assign it separately. Turning off Enabled under All Skills pauses every assigned Agent; it is not a per-Agent switch.

## Runtime differences

- **Desktop:** Assigning a Skill to a CLI Agent may perform filesystem work in its Skill mount directory; assigning to an API Agent stores a prompt binding. If an operation fails, the error stays on the affected Skill row and the assignment does not move optimistically.
- **Web/mock:** You can verify filtering, assignment, removal, and responsive UI behavior, but every result is an in-memory simulation and is not evidence of changed local files or native configuration.
