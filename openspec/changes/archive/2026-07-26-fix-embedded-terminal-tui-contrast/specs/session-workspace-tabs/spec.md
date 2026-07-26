## ADDED Requirements

### Requirement: Readable embedded terminal rendering for agent and shell TUIs

The embedded terminals (the workspace agent terminal and the Shell terminal) SHALL render agent and full-screen TUI output on a self-consistent, opaque terminal canvas using a complete ANSI color palette, so that regions a TUI paints with background fills and the text on those regions remain readable in every registered visual style. The terminal SHALL NOT rely on per-color CSS neutralization of individual ANSI background classes to stay legible, and SHALL NOT leave background-filled TUI regions rendered as unreadable dark blocks or low-contrast text.

#### Scenario: Full-screen TUI paints background-filled regions

- **WHEN** an agent CLI that is a full-screen TUI (for example Codex CLI) paints its input composer, selected rows, or status bar using ANSI background colors
- **THEN** those regions SHALL render on the opaque terminal canvas with foreground text that keeps readable contrast against the region background, with no unreadable dark blocks and no low-contrast text

#### Scenario: Truecolor and 256-color backgrounds

- **WHEN** TUI output uses 256-color or 24-bit truecolor background sequences that no per-class CSS override can target
- **THEN** the terminal SHALL still present that output legibly by rendering it natively against the self-consistent opaque terminal background, without depending on CSS overrides of specific ANSI background classes

#### Scenario: Token-driven terminal palette across registered styles

- **WHEN** the `futuristic` or `minimal` visual style is active
- **THEN** the terminal canvas background, foreground, cursor, selection, and the full 16-color ANSI palette SHALL be provided by semantic terminal tokens rather than a component-hard-coded palette, and SHALL keep agent and TUI output readable in that style

#### Scenario: Consistent rendering for agent terminal and shell terminal

- **WHEN** the same rendering path is used by both the workspace agent terminal and the Shell terminal
- **THEN** both SHALL use the shared opaque terminal canvas and complete ANSI palette so that TUI output stays readable in either tab
