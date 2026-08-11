# Manage Skills

**Status: Delivered — desktop and Web/mock UI.** Desktop performs local persistence, CLI Skill mounting, and API Agent prompt binding. Web/mock only simulates the same UI and state transitions; it does not change local files or runtime configuration.

## Understand views and states

Enablement and Agent assignment are two independent dimensions.

| View or state | Meaning |
| --- | --- |
| All Skills | One effective row per canonical Skill id in the current management context, regardless of enablement or assignment |
| Unassigned | Skills assigned to no CLI Agent or API Agent; enablement does not affect this classification |
| Agent page | Manages the selected Agent's Assigned and Available Skills |
| Enabled | A Skill-wide master switch that allows Agents already assigned that Skill to use it |
| Assigned | A relationship between the Skill and one specific Agent; each Agent is assigned independently |

“Enabled” does not mean “assigned to every CLI.” Enabling a Skill that has no assignments does not make any Agent use it and does not remove it from Unassigned.

Disabling a Skill pauses it for every assigned Agent without deleting those assignments. When the Skill is enabled again, only the previously assigned Agents resume using it; unassigned Agents remain unaffected.

## Understand effective definitions

Global and Workspace are management scopes for enablement, assignment, drift, and deletion intent. The content used at runtime comes from one effective layer selected in this order: Project, User, Registry, then System. Lower-priority definitions with the same canonical id appear under Runtime details as shadowed definitions; they are not separate active Skills.

Each row identifies its Role or Utility type, eager or on-demand delivery, effective layer, origin, version, availability, usage summary, and compatibility state. Existing Skills without explicit type or delivery remain compatible as eager Role Skills.

System packages are read-only. You can preview, enable, disable, and assign them, but you cannot directly edit or delete their content. An Overlay can customize effective content without rewriting the base package; it does not change the base layer or mutability. Utility delegation, bundled-script execution, and remote package installation are not available in this runtime version.

For native API Agents, on-demand Role Skills can be discovered through fixed read-only Skill tools. Resource references use bounded `skill://` identifiers rather than local filesystem paths. Utility Skills remain visible but unavailable until delegated execution is implemented.

## Customize a Skill with an Overlay

Open Overlay in Skill details to compare base and final effective content and inspect scopes, trust, revisions, conflicts, resource shadowing, and history. An Overlay is a customization record above the base package, not another active Skill row.

Overlay scopes and Skill content layers are separate concepts. System, User, and Project Overlays replay in that order, while the base package remains the one effective definition selected from Project, User, Registry, then System. A Project Overlay belongs only to its canonical workspace and is omitted without an active workspace. A higher-scope resource can shadow a lower resource at the same logical path without deleting the original file.

The common workflow is:

1. Choose an exact patch, learned guidance, or supporting file. Supporting files must be under `references`, `templates`, or `assets` and cannot be scripts or executables.
2. Run Preview and review the match count, scan result, complete diff, and revision witnesses.
3. Confirm the commit. If the base package or Overlay revision changed, the dialog retains your input and requires a reload and new preview.
4. Use Disable to exclude a mutation temporarily, or Revert to create a new revision that undoes it. Audit history is retained.

A locally created Overlay is eligible for replay after validation. An imported package starts in untrusted quarantine and cannot affect an Agent until you review and promote its exact revision, content hash, scan result, and diff. Private keys, credential structures, prompt-authority overrides, script markup, and disguised executable content are hard refusals with no force-trust option.

When the base package changes, the Overlay reports that reconciliation is required. Until you approve the new complete diff, Agents use the last healthy lower-scope result or base content. A conflicted scope never applies partially, and dependent higher scopes remain blocked. You can edit a conflicting mutation or ignore and disable it.

Pinning a Skill preserves healthy Overlays that were already effective, but creation, import, promotion, disable, revert, and reconciliation become read-only. Explicitly unpin the Skill before changing it.

Default limits include 1 MiB per supporting file, an 8 MiB import package, 32 MiB expanded content, 512 archive entries, and 256 mutations. Resource paths are limited to 240 characters and eight components; history rolls into linked 4 MiB segments. Desktop commits manifests, resources, history, and counters in one recoverable transaction, so interruption restores either the complete old revision or the complete new revision. Do not edit Overlay storage or history files manually.

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
