## ADDED Requirements

### Requirement: Polished workspace shell visuals
The workspace shell SHALL apply the shared visual design system consistently to the top bar, sidebar, main content panel, composer area, information panel, status bar, dialogs, and session cards.

#### Scenario: Workspace panel rhythm
- **WHEN** the workspace shell renders sidebar, main content, and information panel surfaces
- **THEN** panels SHALL use consistent border strength, panel backgrounds, radius, spacing, and shadow depth
- **AND** panel transitions and collapse controls SHALL remain visually aligned in both `futuristic` and `minimal` styles

#### Scenario: Session list visual hierarchy
- **WHEN** session cards, folder groups, activity groups, pinned areas, and archived areas render
- **THEN** they SHALL use consistent list-row density, icons, status markers, text hierarchy, hover states, and selected states
- **AND** long titles, folder paths, and agent labels SHALL not overlap adjacent controls

### Requirement: Workspace icon and toolbar polish
The workspace shell SHALL use consistent icons and compact toolbar controls for high-frequency workspace actions.

#### Scenario: Workspace action icons
- **WHEN** the top bar, sidebar utility row, session card context actions, create-session dialog, information panel tabs, or composer controls render actions
- **THEN** controls SHALL use consistent lucide or existing project icons where icons improve recognition
- **AND** icon-only controls SHALL have translated tooltips or accessible labels

#### Scenario: Compact grouped controls
- **WHEN** related workspace actions are displayed together
- **THEN** they SHALL use compact grouped-control styling with stable dimensions, consistent gaps, and clear active states
- **AND** hover or active styles SHALL not cause neighboring controls to shift

### Requirement: Workspace theme refinement
The workspace shell SHALL preserve functional layout behavior while improving visual quality in both registered styles.

#### Scenario: Futuristic workspace appearance
- **WHEN** `futuristic` style is active
- **THEN** the workspace SHALL present a dark, focused operational surface with subtle panel depth, readable transcript content, and clear primary/status accents

#### Scenario: Minimal workspace appearance
- **WHEN** `minimal` style is active
- **THEN** the workspace SHALL present a bright, crisp operational surface with low visual noise, clear separation between panels, and readable compact controls
