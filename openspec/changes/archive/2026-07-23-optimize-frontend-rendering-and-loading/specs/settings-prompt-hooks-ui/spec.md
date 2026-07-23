## ADDED Requirements

### Requirement: Windowed large Prompt Hook inventories
The Prompt Hooks settings page SHALL use measured row windowing when the filtered inventory contains more than 500 hooks and SHALL preserve ordinary document-flow rendering at or below 500 hooks.

#### Scenario: Render a small or medium inventory
- **WHEN** the filtered Prompt Hook inventory contains 500 or fewer hooks
- **THEN** the page SHALL render the existing responsive card grid without virtualization

#### Scenario: Render a large inventory
- **WHEN** the filtered Prompt Hook inventory contains more than 500 hooks
- **THEN** the page SHALL mount only viewport-visible card rows plus no more than four overscan rows before and after the visible range
- **AND** it SHALL preserve the filtered order and stable hook ids

#### Scenario: Use responsive columns
- **WHEN** a windowed Prompt Hook inventory crosses between one-column and two-column layouts
- **THEN** the page SHALL regroup and remeasure virtual rows
- **AND** no hook SHALL be omitted, duplicated, clipped, or reordered

#### Scenario: Change filters or grouping
- **WHEN** the user changes a Prompt Hook filter, search term, sort, or grouping control
- **THEN** the virtualized collection SHALL update from the resulting ordered hooks
- **AND** the collection SHALL return to its start without retaining stale virtual indices

#### Scenario: Operate an offscreen hook
- **WHEN** the user scrolls a large inventory until a previously unmounted hook becomes visible
- **THEN** its card SHALL expose the same enablement, agent assignment, edit, delete, trace, and diagnostic operations as a non-virtual card

#### Scenario: Navigate a windowed inventory accessibly
- **WHEN** keyboard or assistive-technology users traverse a large Prompt Hook inventory
- **THEN** rendered cards SHALL expose their position and total collection size
- **AND** scrolling SHALL make subsequent hooks available without trapping focus
