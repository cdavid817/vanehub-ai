## ADDED Requirements

### Requirement: Extension-contributed Skills use immutable virtual Registry-layer packages

Each valid Skill contribution from an enabled extension SHALL be projected as one immutable virtual Registry-layer package with source `extension:<extension-id>`, extension version, Skill subtree content hash, package/snapshot provenance, and eligibility tied to the extension generation. It SHALL participate in the existing `Project > User > Registry > System` order and SHALL not create a parallel Skill precedence layer.

#### Scenario: Project Skill shadows extension Skill

* WHEN a Project definition has the same Skill id as an eligible extension virtual Registry package
* THEN the Project definition remains effective according to current precedence

#### Scenario: Extension is rolled back

* WHEN the owning extension rolls back to an older snapshot
* THEN new effective resolutions use the corresponding immutable virtual Skill package while in-flight contexts may retain their pinned prior revision according to current policy

### Requirement: Extension provenance does not grant Skill authority

A verified/signed extension package SHALL NOT automatically trust executable Skill tools, grant allowed tools, provide secrets, change Skill configuration, modify Overlay, or authorize delegation. Those gates SHALL remain owned by the effective Skill, Skill Tool, configuration, delegation, and Permissions contracts.

#### Scenario: Extension bundles a WASM Skill tool

* WHEN the Skill references executable tool content
* THEN that content remains subject to current independent Skill Tool validation/trust/permission rules before eligibility

### Requirement: Extension lifecycle preserves user-owned Skill state

Disabling, reloading, rolling back, or uninstalling an extension SHALL update virtual base eligibility atomically but SHALL preserve user/project forks, Overlay history, configuration, usage/audit history, and unrelated Registry content.

#### Scenario: User customized an extension Skill through Overlay

* WHEN the extension is uninstalled
* THEN the uninstall preview identifies retained user-owned state and does not delete it
