## ADDED Requirements

### Requirement: Auxiliary desktop surface visual consistency
Auxiliary VaneHub desktop surfaces SHALL use the same semantic visual system, icon conventions, accessibility treatment, and registered style meanings as workspace and settings surfaces.

#### Scenario: Render the futuristic floating surface
- **WHEN** the floating assistant renders with `futuristic` active
- **THEN** it SHALL use token-provided dark operational backgrounds, restrained translucent depth, blue primary accents, readable muted text, and visible status/focus contrast

#### Scenario: Render the minimal floating surface
- **WHEN** the floating assistant renders with `minimal` active
- **THEN** it SHALL use token-provided bright solid backgrounds, restrained borders, low shadow, clear primary accents, and readable compact-density text

#### Scenario: Avoid theme-specific component branches
- **WHEN** the floating assistant switches between registered styles
- **THEN** its components SHALL change appearance through semantic token values or shared classes and SHALL NOT branch on theme names or introduce a page-local hard-coded palette

#### Scenario: Use compact icon and status semantics
- **WHEN** the collapsed control, quick actions, session status, or mini-chat controls render
- **THEN** they SHALL use consistent Bot/Sparkles or action icons, translated accessible labels, and status text or accessible names in addition to color

#### Scenario: Verify auxiliary surface quality
- **WHEN** the floating assistant implementation is ready for review
- **THEN** visual QA SHALL cover collapsed, menu, chat, empty, streaming, and error states in both registered styles
- **AND** inspection SHALL confirm transparent edges, focus visibility, readable contrast, reduced-motion behavior, and absence of clipped or overlapping content at every native surface size
