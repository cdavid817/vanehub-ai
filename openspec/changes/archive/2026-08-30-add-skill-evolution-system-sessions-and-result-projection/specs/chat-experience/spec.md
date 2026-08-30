## ADDED Requirements

### Requirement: Read-only system activity presentation
When a system activity session is selected, the chat workspace SHALL replace interactive conversation controls with a localized read-only activity timeline and system identity. It SHALL hide composer, send, stop, enhance, mentions, file references, model/reasoning/permission controls, terminal, and Agent availability actions.

#### Scenario: Open system activity session
- **WHEN** the selected view has system-activity kind
- **THEN** the UI shows timeline filters, unread state, safe export, and projection health without a message composer

#### Scenario: Keyboard send shortcut is used
- **WHEN** focus is inside a system activity view and the user presses a normal send shortcut
- **THEN** no message, Agent invocation, or mutation command is issued

### Requirement: Localized structured activity rendering
The activity presentation SHALL render locale-neutral event codes and safe parameters as localized timeline items and supported read-only Rich Blocks while preserving stable ids, timestamps, severity, status, and accessible labels.

#### Scenario: Unsupported activity payload appears
- **WHEN** an item has an unknown payload schema
- **THEN** the UI renders a bounded safe fallback with event code and preserves the rest of the timeline

### Requirement: System activity navigation is non-mutating
Links and buttons inside system activity items SHALL only navigate, copy safe ids, adjust local filters/read state, or export. They MUST NOT approve, apply, retry source work, cancel runs, close breakers, revert Overlays, or send chat messages.

#### Scenario: Attention item links to breaker
- **WHEN** the user follows the link
- **THEN** breaker detail opens without acknowledging or closing it

