# Cross-Platform App Icon Specification

## Purpose

Define the canonical VaneHub AI app icon identity, responsive source variants, deterministic cross-platform generation, platform integration, and verification requirements.

## Requirements

### Requirement: Canonical VaneHub app icon identity
The system SHALL provide a canonical VaneHub AI app icon that preserves the recognizable `V` mark and uses a restrained visual language for agent coordination, workspace convergence, and automation.

#### Scenario: Maintainer inspects the canonical source
- **WHEN** a maintainer opens the primary app icon source
- **THEN** the icon SHALL contain the VaneHub `V` as its dominant silhouette
- **AND** supporting orbit and hub elements SHALL remain subordinate to that silhouette

#### Scenario: Icon is rendered on light or dark surroundings
- **WHEN** a generated icon is displayed against a light or dark operating-system surface
- **THEN** its outer silhouette and central mark SHALL remain distinguishable with sufficient foreground-to-background contrast

### Requirement: Minimal visual treatment
The canonical app icon SHALL use a minimal flat visual language with one dark base color, one light foreground color, and one accent color, while avoiding nonessential material effects and decorative layers.

#### Scenario: Maintainer inspects the master source
- **WHEN** a maintainer reviews the master SVG
- **THEN** it SHALL NOT contain decorative gloss, ambient aura, blur or offset shadow, inner-rim decoration, complex texture, or an internal bridge element
- **AND** depth SHALL be expressed through geometric overlap and negative space

#### Scenario: Icon is viewed at medium size
- **WHEN** the master icon is rendered between 48px and 128px
- **THEN** the `V`, orbit, and Hub SHALL remain visually distinct without relying on shadow or highlight effects

### Requirement: Responsive optical icon variants
The icon system SHALL provide separate master and compact vector sources so required sizes from 16px through 512px retain a recognizable silhouette and appropriate detail density.

#### Scenario: Small icon assets are generated
- **WHEN** the generator produces 16px, 24px, or 32px assets
- **THEN** it SHALL use the compact optical source
- **AND** the result SHALL omit fine decorative treatments that would reduce small-size clarity

#### Scenario: Medium and large icon assets are generated
- **WHEN** the generator produces assets from 48px through 512px
- **THEN** it SHALL use the master source with the full brand detail hierarchy

### Requirement: Deterministic cross-platform icon generation
The repository SHALL provide one npm command backed by a cross-platform Node script that deterministically generates required Windows, macOS, Linux, iOS, Android, and Web icon assets from canonical vector sources.

#### Scenario: Maintainer generates all icon assets
- **WHEN** a maintainer runs `npm run icons:generate` after installing project dependencies
- **THEN** the command SHALL generate platform assets without requiring a platform-specific PowerShell workflow
- **AND** repeated generation from unchanged sources SHALL preserve the same dimensions, formats, and file layout

#### Scenario: A canonical source is missing
- **WHEN** the generator cannot find a required source or the local Tauri CLI
- **THEN** it SHALL stop with a non-zero exit status and identify the missing prerequisite

### Requirement: Desktop bundle icon integration
The Tauri desktop bundle SHALL reference existing platform-appropriate PNG, ICNS, and ICO assets produced by the canonical icon generator.

#### Scenario: Desktop bundle configuration is inspected
- **WHEN** Tauri resolves the configured bundle icon paths
- **THEN** every configured path SHALL exist under `src-tauri/icons/generated/`
- **AND** the configured set SHALL include PNG, ICNS, and ICO formats

#### Scenario: Windows icon container is generated
- **WHEN** the generator writes the Windows ICO asset
- **THEN** the ICO SHALL contain 16px, 24px, 32px, 48px, 64px, 128px, and 256px frames
- **AND** the 16px through 32px frames SHALL originate from the compact optical source

### Requirement: Web and installable-browser icon integration
The Web runtime SHALL expose SVG and PNG favicons, an Apple touch icon, theme metadata, and a Web App Manifest containing 192px and 512px application icons.

#### Scenario: Browser loads application metadata
- **WHEN** the Web application document is loaded
- **THEN** it SHALL reference the SVG favicon, 32px PNG favicon, Apple touch icon, Web App Manifest, and application theme color

#### Scenario: Browser reads the Web App Manifest
- **WHEN** an installable-browser surface reads `site.webmanifest`
- **THEN** the manifest SHALL identify VaneHub AI
- **AND** it SHALL reference valid 192px and 512px PNG icons with maskable support

### Requirement: Android adaptive and themed icon support
The icon system SHALL provide dedicated Android adaptive foreground and monochrome vector sources plus a controlled background color.

#### Scenario: Android adaptive assets are generated
- **WHEN** the Tauri icon generator processes the icon manifest
- **THEN** it SHALL generate density-specific standard, round, foreground, and monochrome Android icon assets
- **AND** the foreground mark SHALL remain inside the adaptive-icon safe area

### Requirement: Single authoritative icon workflow
The repository MUST keep canonical icon sources and outputs directly under `src-tauri/icons/` and MUST NOT retain obsolete candidate icons or legacy generators that recreate them.

#### Scenario: Maintainer inspects the icon directory
- **WHEN** the contents of `src-tauri/icons/` are reviewed
- **THEN** the top-level structure SHALL consist of documented canonical source and output groups without a versioned `vanehub-v2` wrapper

#### Scenario: Maintainer inspects generation scripts
- **WHEN** repository scripts are searched for icon generators
- **THEN** `scripts/generate-vanehub-icon.mjs` SHALL be the only project-owned app-icon generator
- **AND** legacy candidate-generating PowerShell scripts SHALL be absent

### Requirement: Icon asset verification
The implementation SHALL verify generated dimensions, container membership, regeneration success, application build compatibility, and repository quality checks before the icon change is considered complete.

#### Scenario: Icon change is prepared for review
- **WHEN** canonical icon sources or generation logic change
- **THEN** maintainers SHALL run the icon generator and confirm required PNG dimensions and ICO frames
- **AND** project build, lint, automated tests, Rust checks, and strict OpenSpec validation SHALL complete without new errors attributable to the icon change
