## MODIFIED Requirements

### Requirement: Localized task-oriented user guides
The repository SHALL provide English and Simplified Chinese user guides organized around user goals, with equivalent navigation, commands, runtime applicability, prerequisites, results, and troubleshooting coverage. The Simplified Chinese guide SHALL be the authoritative complete set; the English guide SHALL be rebuilt to the same chapter topology.

During a declared transition period recorded in an OpenSpec change, the English guide MAY be partial, but every missing chapter SHALL be marked explicitly as a known gap in the guide's navigation, and the English guide SHALL NOT silently diverge from the Simplified Chinese guide in navigation structure, runtime labeling, or truthful feature-state labeling. Outside any declared transition period, the unconditional equivalence requirement applies in full.

#### Scenario: Follow a guide in either supported language
- **WHEN** a reader selects English or Simplified Chinese
- **THEN** the user guide SHALL expose equivalent task chapters and workflow outcomes in that language
- **AND** product names, stable Agent ids, commands, paths, and configuration keys SHALL remain technically accurate

#### Scenario: English guide is partial during a declared transition
- **WHEN** an OpenSpec change declares a transition period during which the English guide is not yet complete
- **THEN** every English chapter absent from the complete Simplified Chinese set SHALL be represented in the English navigation as an explicit known-gap marker
- **AND** the English guide SHALL NOT silently omit a chapter that exists in the Simplified Chinese guide
- **AND** chapters that ARE present SHALL match the Simplified Chinese guide in navigation order, runtime labels, and feature-state labels

#### Scenario: Guide step differs by runtime
- **WHEN** a task has different desktop-native and Web/mock behavior
- **THEN** the guide SHALL label each runtime path before the divergent steps
- **AND** Web/mock instructions SHALL not claim native process, SQLite, filesystem, or operating-system side effects

## ADDED Requirements

### Requirement: User-guide locale set is distinct from application UI locales
The user-guide locale set SHALL be exactly English and Simplified Chinese. Application UI resource locales that are not in the user-guide locale set (for example, Japanese, Traditional Chinese, Korean) SHALL NOT be advertised as having a user guide, and README localization claims SHALL distinguish between delivered application UI resources and delivered user guides.

#### Scenario: Application UI locale has no user guide
- **WHEN** an application UI locale is delivered (for example, Japanese UI resources) but is not in the user-guide locale set
- **THEN** the README and documentation entry points SHALL NOT claim a user guide exists for that locale
- **AND** the README SHALL state which locales have application UI resources and which have user guides, as separate facts

#### Scenario: Reader expects a guide in a UI-only locale
- **WHEN** a reader who uses the Japanese UI looks for a Japanese user guide
- **THEN** the documentation SHALL state that user guides are available in English and Simplified Chinese only
- **AND** the documentation SHALL NOT present the absence of a Japanese guide as a defect or a broken promise
