## Why

The About settings page had become visually fragmented after removing runtime and local CLI environment content. Users need product details, update state, changelog, and positioning to scan as related information without a scattered stack of small cards.

## What Changes

- Rework the About settings page into a responsive horizontal layout aligned with the Basic Configuration page spacing and panel treatment.
- Merge product identity, software metadata, repository links, and update controls into one software information panel.
- Merge changelog and product positioning into one related information panel.
- Keep the previously removed runtime/agent and local CLI environment sections out of the About page.
- Preserve existing frontend service boundaries for update checks; no direct Tauri calls are introduced.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `settings-center-ui`: Clarify the About page information architecture and remove the obsolete requirement to display supported runtimes and supported AI coding agents.

## Impact

- Affects Web and Tauri desktop frontend rendering of the Settings About page.
- Updates `src/settings/pages/about-page.tsx` and its regression test.
- No backend, database, runtime adapter, dependency, or public API changes.
