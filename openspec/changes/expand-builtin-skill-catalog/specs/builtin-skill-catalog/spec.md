## Purpose

Defines VaneHub AI's exact first-party Skill inventory and guarantees that every shipped package is correctly classified, safe, discoverable, dependency-honest, progressively disclosed, and upgrade-compatible.

## ADDED Requirements

### Requirement: Exact 28-package catalog
The System layer SHALL ship exactly the following 28 first-party canonical Skill ids, with no duplicate canonical ids: `developer`, `code-explorer`, `code-review`, `code-modification-mr`, `fix-vulnerability`, `mcp-builder`, `plugin-creator`, `plan`, `specification-architect`, `coach`, `general-assistant`, `project-initializer`, `vanehub-expert`, `skill-creator`, `md2word`, `pptx-craft`, `codewiki-api`, `deepresearch`, `image-analyzer`, `sdd-design-story`, `sdd-cloud-desktop-manager`, `specification-architect-skill-creator`, `version-bug-analysis`, `tdd-discipline`, `code-security-scan`, `api-doc-generation`, `unit-test-generation`, and `readme-generation`.

#### Scenario: System catalog enumerated
- **WHEN** the first-party System manifest is validated
- **THEN** it SHALL contain all and only the 28 canonical ids in this requirement

#### Scenario: Existing ids retained
- **WHEN** a user upgrades from the six-package catalog
- **THEN** all six existing canonical ids SHALL remain unchanged: `tdd-discipline`, `code-review`, `code-security-scan`, `api-doc-generation`, `unit-test-generation`, and `readme-generation`

#### Scenario: Stable kebab-case identities
- **WHEN** any first-party package is listed, loaded, assigned, used, overlaid, restored, or referenced in history
- **THEN** the system SHALL use its stable lowercase kebab-case canonical id rather than its localized display name

### Requirement: Explicit Role catalog classification
The following 13 packages SHALL declare `type: role`: `developer`, `mcp-builder`, `plugin-creator`, `specification-architect`, `coach`, `general-assistant`, `image-analyzer`, `tdd-discipline`, `code-review`, `code-security-scan`, `api-doc-generation`, `unit-test-generation`, and `readme-generation`. The seven newly introduced Role packages SHALL use on-demand delivery; the six existing packages SHALL retain eager delivery for compatibility.

#### Scenario: New Role loaded on demand
- **WHEN** one of the seven newly introduced Role packages is enabled and assigned to a native API Agent
- **THEN** it SHALL be discoverable and loadable on demand without automatic eager prompt injection

#### Scenario: Existing Role compatibility
- **WHEN** one of the six existing Role packages remains enabled and assigned after upgrade
- **THEN** it SHALL retain its previous eager delivery unless a higher-layer definition explicitly changes delivery

#### Scenario: Role is not delegated
- **WHEN** an Agent attempts to invoke one of these Role packages through Utility delegation
- **THEN** the runtime SHALL refuse it as the wrong Skill type

### Requirement: Explicit Utility catalog classification
The following 15 packages SHALL declare `type: utility` and on-demand delivery: `code-explorer`, `code-modification-mr`, `fix-vulnerability`, `plan`, `project-initializer`, `vanehub-expert`, `skill-creator`, `md2word`, `pptx-craft`, `codewiki-api`, `deepresearch`, `sdd-design-story`, `sdd-cloud-desktop-manager`, `specification-architect-skill-creator`, and `version-bug-analysis`.

#### Scenario: Utility catalog metadata
- **WHEN** any of these packages is listed
- **THEN** it SHALL be identified as delegated Utility content and SHALL NOT be eagerly injected as a Role

#### Scenario: Delegation runtime available
- **WHEN** an effective Utility has satisfied dependencies and is assigned to a supported native API Agent
- **THEN** it SHALL be eligible for bounded delegation according to its declared capability contract

#### Scenario: Delegation runtime unavailable
- **WHEN** Utility delegation support is not available in the running application
- **THEN** the package SHALL remain visible and previewable but SHALL expose a safe delegation-unavailable reason

### Requirement: Catalog purpose boundaries
Each first-party package SHALL have a distinct bounded purpose matching its canonical identity and SHALL NOT claim capabilities outside its declared dependencies and tool contract.

#### Scenario: Development purposes
- **WHEN** development Skills are inspected
- **THEN** `developer` SHALL guide implementation, `code-explorer` SHALL analyze repository structure, `code-review` SHALL review changes, `code-modification-mr` SHALL coordinate bounded change delivery, `fix-vulnerability` SHALL coordinate validated security remediation, `mcp-builder` SHALL guide MCP development, and `plugin-creator` SHALL guide VaneHub-compatible plugin work

#### Scenario: Architecture purposes
- **WHEN** architecture Skills are inspected
- **THEN** `plan` SHALL produce bounded analysis and execution plans, `specification-architect` SHALL coach specification decisions, `coach` SHALL clarify goals and working mode, and `general-assistant` SHALL coordinate general tasks without impersonating an unavailable Utility

#### Scenario: Project and configuration purposes
- **WHEN** project/configuration Skills are inspected
- **THEN** `project-initializer` SHALL initialize project knowledge, `vanehub-expert` SHALL guide VaneHub configuration and troubleshooting, and `skill-creator` SHALL create or improve valid Skill packages

#### Scenario: Document purposes
- **WHEN** document Skills are inspected
- **THEN** `md2word` SHALL coordinate Markdown-to-Word production, `pptx-craft` SHALL coordinate presentation production, and `codewiki-api` SHALL integrate configured code-documentation services without embedding credentials

#### Scenario: Research purposes
- **WHEN** research Skills are inspected
- **THEN** `deepresearch` SHALL perform bounded evidence-based research and `image-analyzer` SHALL analyze software-engineering images without claiming unsupported image access

#### Scenario: Specification workflow purposes
- **WHEN** specification workflow Skills are inspected
- **THEN** `sdd-design-story` SHALL coordinate local specification-driven artifacts and `sdd-cloud-desktop-manager` SHALL interact only with an explicitly configured compatible workspace service

#### Scenario: Specialized purposes
- **WHEN** specialized Skills are inspected
- **THEN** `specification-architect-skill-creator` SHALL generate team-specific specification-coach Skills and `version-bug-analysis` SHALL produce bounded issue-quality and root-cause analysis

#### Scenario: Existing package purposes retained
- **WHEN** the existing six packages are inspected after migration
- **THEN** their established TDD, review, security, API documentation, unit-test, and README purposes SHALL remain recognizable and backward compatible

### Requirement: Canonical aliases
Aliases SHALL be unique after canonical-id precedence. `code-reviewer` SHALL resolve to `code-review`; common concise aliases MAY be provided for other packages only when they do not collide with any canonical id or alias.

#### Scenario: Code reviewer alias
- **WHEN** discovery or loading receives `code-reviewer`
- **THEN** it SHALL resolve to the canonical `code-review` package and all persisted identity SHALL use `code-review`

#### Scenario: Alias collision in manifest
- **WHEN** first-party manifest validation finds an alias that collides ambiguously with another alias or canonical id
- **THEN** validation SHALL fail before the catalog is packaged

#### Scenario: Localized name is not an alias automatically
- **WHEN** a localized display name is rendered
- **THEN** it SHALL NOT become an identity alias unless explicitly declared and validated

### Requirement: Progressive package structure
Every first-party package SHALL contain a valid concise `SKILL.md` and MAY contain only required resources under `references`, `templates`, `assets`, and non-executing `scripts`. Detailed material SHALL be placed in indexed resources and referenced from `SKILL.md` instead of duplicating it in the instruction body.

#### Scenario: Concise primary instructions
- **WHEN** a first-party `SKILL.md` body is validated
- **THEN** it SHALL remain within the catalog's configured concise-body budget and SHALL link to required detailed resources using logical package-relative references

#### Scenario: Referenced resource exists
- **WHEN** `SKILL.md` or another validated first-party document references a package resource
- **THEN** the target SHALL exist inside the same package and use a safe canonical relative path

#### Scenario: Unreferenced resource
- **WHEN** a first-party package contains a resource not referenced by metadata, instructions, or another reachable resource
- **THEN** catalog validation SHALL report it as an orphan and fail unless the manifest explicitly identifies its runtime purpose

#### Scenario: Scripts remain inert
- **WHEN** a first-party package contains a file under `scripts`
- **THEN** this change SHALL treat it only as an indexed non-executing resource and SHALL NOT dynamically load or run it

### Requirement: Dependency-honest availability
Each first-party package SHALL declare the runtime capabilities, configured integrations, and input modalities required for its advertised behavior. Missing optional dependencies SHALL make only the affected behavior or package unavailable with an actionable reason and SHALL NOT trigger installation, account access, process launch, or network activity during availability checks.

#### Scenario: Core-only Skill available
- **WHEN** a package requires only capabilities present in the core runtime
- **THEN** availability checking SHALL report it available without starting an Agent or external process

#### Scenario: Document capability missing
- **WHEN** a document-production Utility requires an artifact capability that is not available
- **THEN** the package SHALL remain visible but SHALL report the missing capability and SHALL NOT claim it can produce that artifact

#### Scenario: External integration unconfigured
- **WHEN** `codewiki-api` or `sdd-cloud-desktop-manager` lacks its explicitly required configured integration
- **THEN** delegation SHALL be unavailable with setup guidance and SHALL NOT attempt a network request

#### Scenario: Image input unavailable
- **WHEN** `image-analyzer` is loaded in a runtime that cannot provide image input
- **THEN** the system SHALL expose the modality limitation rather than fabricating analysis

### Requirement: First-party content safety
First-party packages SHALL contain no credentials, private keys, user-specific absolute paths, hidden prompt-authority overrides, executable payloads, unsafe traversal, embedded account identifiers, or instructions to bypass VaneHub permissions, approvals, logging, service boundaries, or OpenSpec governance.

#### Scenario: Secret scan
- **WHEN** first-party package validation detects credential or private-key material
- **THEN** the build SHALL fail without publishing the package content

#### Scenario: Permission bypass instruction
- **WHEN** package content directs an Agent to bypass approval, sandbox, unified logging, or project governance
- **THEN** validation SHALL fail and identify the safe rule id and package

#### Scenario: Executable resource
- **WHEN** a first-party package contains a prohibited executable or script payload
- **THEN** validation SHALL fail even when the file is under an allowed resource directory

#### Scenario: Example placeholders
- **WHEN** a package needs to demonstrate configuration or credentials
- **THEN** it SHALL use unmistakable non-secret placeholders and direct the user to existing settings or service boundaries

### Requirement: First-party content quality validation
Every first-party package SHALL pass deterministic validation for required frontmatter, canonical identity, type, delivery, category, semantic version, aliases, delegation metadata, dependencies, body budget, resource links, UTF-8 text, duplicate purpose, unsafe content, and manifest hashes before application packaging.

#### Scenario: Invalid package blocks build
- **WHEN** any first-party package fails a required validation rule
- **THEN** catalog generation or verification SHALL fail and identify the package and safe rule id

#### Scenario: Duplicate purpose detected
- **WHEN** two first-party packages have materially duplicate identity, trigger, and purpose metadata without an explicit relationship
- **THEN** validation SHALL require merge, alias, or documented differentiation before packaging

#### Scenario: Manifest reproducibility
- **WHEN** the same validated package tree is processed twice
- **THEN** it SHALL produce the same deterministic package order, resource inventory, and content hashes

#### Scenario: Behavioral fixture
- **WHEN** catalog tests evaluate a package's representative positive and negative trigger fixtures
- **THEN** its expected type, purpose, dependency, and selection metadata SHALL match the fixture contract

### Requirement: Catalog localization
First-party package ids and instruction resources SHALL remain stable across locales, while user-facing package names, descriptions, categories, dependency reasons, and validation messages SHALL use the existing localization system with an English fallback.

#### Scenario: Chinese UI catalog
- **WHEN** the UI locale is Simplified Chinese
- **THEN** catalog names, descriptions, categories, and availability reasons SHALL use available Chinese translations without changing canonical ids

#### Scenario: Missing translation
- **WHEN** a catalog translation key is missing in the active locale
- **THEN** the UI SHALL use the English fallback and SHALL NOT display a raw undefined value

