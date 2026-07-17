## Why

`settings-center-ui` has grown to 775 lines and 55 requirements, exceeding the repository's Spec Optimizer default budget. It combines shell behavior with CLI, basic settings, Skills, usage, extensions, IM, and floating-assistant UI requirements, making targeted review expensive.

## What Changes

- Produce a read-only compression analysis and source-to-target requirement mapping.
- Propose a capability split that preserves every requirement and scenario.
- Do not modify main specs until the mapping is approved in a follow-up change.

## Capabilities

### New Capabilities

- None.

### Modified Capabilities

- None; this change is analysis only.

## Impact

- Adds review artifacts under this change only. Desktop runtime, Web runtime, application code, and main specs remain unchanged.
