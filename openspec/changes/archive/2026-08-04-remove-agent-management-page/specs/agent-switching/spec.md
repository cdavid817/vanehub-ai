## REMOVED Requirements

### Requirement: Agent switching in settings center
**Reason**: Agent Management settings UI is removed, including runtime Agent selection, compatible-mode selection, and workflow launch. These controls are not moved into Agent Configuration.

**Migration**: Agent selection required for creating or using Sessions remains in the corresponding session workflows; no replacement settings-page workflow is provided.

### Requirement: Agent status visibility in settings center
**Reason**: Registered-Agent status, capability, workflow lifecycle, and Session-detail presentation belonged to the removed Agent Management UI.

**Migration**: No replacement settings surface is provided. Runtime and registry data remain available to non-settings consumers through the existing service boundary.
