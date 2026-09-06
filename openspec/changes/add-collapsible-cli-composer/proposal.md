## Why

A single-Agent CLI session already accepts keystrokes directly in the embedded terminal; the line composer beneath it is a convenience, not a requirement. It permanently takes three rows plus a button bar away from the terminal, which is the surface the user is actually working in. Users asked to fold it away.

## What Changes

- Add a collapse control to the Agent terminal composer. Collapsed, it shrinks to a single thin bar with an expand control and a hint that the terminal accepts input directly; the terminal reclaims the space and refits.
- Remember the collapsed state per viewer through browser storage, as a display convenience rather than an application setting.
- Localize the two control labels and the hint in every registered locale.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `agent-terminal-runtime`: Add the collapsible composer to the terminal presentation contract.

## Impact

- `src/session-workspace/agent-terminal-composer.tsx`, five locale files, a component test. No service, native, or persistence change.
