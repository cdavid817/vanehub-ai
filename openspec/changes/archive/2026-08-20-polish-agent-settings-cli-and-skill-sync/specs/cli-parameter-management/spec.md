## ADDED Requirements

### Requirement: Officially audited comprehensive editable catalogs
The editable parameter catalog for each managed CLI SHALL cover the current useful non-secret launch options documented by that CLI's official command reference, except for VaneHub-owned arguments and approval, permission, or sandbox controls governed by Agent Policies. Each catalog SHALL record an official source URL and review date, and desktop and Web/mock runtimes SHALL expose identical definitions.

#### Scenario: Audit every managed CLI
- **WHEN** the catalog contract is verified
- **THEN** Claude Code, Codex CLI, Gemini CLI, OpenCode, and Antigravity CLI SHALL each have an official-source audit record
- **AND** every exposed flag SHALL match the documented spelling, value grammar, and applicable interactive or chat scope

#### Scenario: Official reference documents a useful safe launch option
- **WHEN** an official CLI reference exposes a non-secret option that can be represented by the supported typed controls and does not conflict with a reserved or policy-owned concern
- **THEN** the managed profile SHALL expose that option with a stable id, localized description, validation, default, risk, and deterministic argument rendering

#### Scenario: Official documentation is incomplete or disagrees with an installed build
- **WHEN** a candidate flag cannot be confirmed by an official reference or is not accepted by the supported provider invocation grammar
- **THEN** the system SHALL omit it from the editable catalog rather than infer or pass an unverified raw argument

### Requirement: Expanded catalog controls remain safe and usable
Expanded CLI parameter profiles SHALL preserve atomic validation, compact presentation, and safe preview behavior as the number of definitions grows.

#### Scenario: Browse an expanded profile
- **WHEN** a managed CLI contains multiple parameter groups
- **THEN** the page SHALL group related controls, keep flag descriptions scannable, and remain usable at supported narrow widths without horizontal page overflow

#### Scenario: Preview expanded selections
- **WHEN** the user changes or saves expanded parameter selections
- **THEN** the preview SHALL include only validated user-controlled tokens in provider-defined order
- **AND** it SHALL continue to omit credentials, prompts, session identifiers, output-protocol arguments, and policy-governed flags
