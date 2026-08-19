# repository-governance Specification

## Purpose
TBD - created by archiving change configure-github-repository. Update Purpose after archive.

## Requirements

### Requirement: Protected default branch
The GitHub repository SHALL protect the default branch from deletion, force pushes, and unvalidated direct changes, and SHALL require pull requests with resolved review conversations and successful required checks.

#### Scenario: Pull request targets main
- **WHEN** a contributor proposes a change to `main`
- **THEN** GitHub SHALL require the configured CI checks and all review conversations to resolve before merge

#### Scenario: Destructive branch update
- **WHEN** an actor attempts to delete `main` or update it with a non-fast-forward push
- **THEN** the repository ruleset SHALL reject the operation

### Requirement: Public contribution guidance
The repository SHALL publish ownership, contribution, conduct, support, vulnerability-reporting, issue, and pull-request guidance consistent with the project's OpenSpec and runtime-boundary rules.

#### Scenario: Contributor opens a pull request
- **WHEN** a contributor prepares a pull request
- **THEN** the template SHALL prompt for OpenSpec impact, adapter parity, logging constraints, required validation, and UI evidence where applicable

#### Scenario: Reporter discovers a vulnerability
- **WHEN** a reporter reads the repository security policy
- **THEN** the policy SHALL direct confidential reports to GitHub private vulnerability reporting instead of a public issue

### Requirement: Consistent merge and ownership settings
The repository SHALL use a documented ownership map, squash-oriented merge behavior, automatic feature-branch deletion, and labels that identify major project areas.

#### Scenario: Pull request is merged
- **WHEN** a feature pull request is merged
- **THEN** GitHub SHALL allow the configured squash workflow and delete the merged head branch automatically

#### Scenario: Repository area changes
- **WHEN** a pull request modifies paths covered by the label configuration
- **THEN** repository automation SHALL apply the corresponding project-area labels

### Requirement: Unified architecture fitness entry point
The repository SHALL provide one documented architecture fitness command that executes the registered frontend, native, and repository architecture rules without duplicating their underlying implementations.

#### Scenario: Developer runs the architecture gate
- **WHEN** a developer runs the repository architecture fitness command
- **THEN** all registered architecture rule groups SHALL execute and the command SHALL fail if any group reports a violation

#### Scenario: Architecture rule fails
- **WHEN** a registered rule detects a violation
- **THEN** its diagnostic SHALL include a stable rule id, affected file and line or module, and a concise repair direction

### Requirement: Prohibited production dependencies
Production frontend source SHALL use React built-in state and context and MUST NOT import Redux, Zustand, or MobX packages.

#### Scenario: Prohibited state library is imported
- **WHEN** production frontend source imports Redux, Zustand, or MobX directly or through their standard React bindings
- **THEN** architecture fitness SHALL fail with the dependency rule id and source location

#### Scenario: Historical package entry is unused
- **WHEN** a prohibited package remains declared but has no production use
- **THEN** the change SHALL either remove it safely or record a bounded removal task rather than add a permanent exemption

### Requirement: Architecture detector fixture coverage
Every architecture detector introduced by the repository SHALL have deterministic accepting and rejecting fixtures, including diagnostics assertions for rejected input.

#### Scenario: Detector fixtures run
- **WHEN** architecture detector unit tests execute
- **THEN** compliant fixtures SHALL pass and one fixture for every prohibited construct SHALL fail with the expected rule id and location

### Requirement: Existing source constraints remain enforced
The architecture gate SHALL preserve the repository's existing TypeScript, React, Rust, and file-size constraints and MUST NOT introduce a new blanket or permanent exemption. An existing oversized source path SHALL be governed by a recorded line budget rather than by disabling the file-size rule for that path. The prohibition on Rust panic shortcuts SHALL be enforced mechanically against non-test targets, and SHALL NOT be enforced against test targets, where the shortcuts are permitted.

#### Scenario: Production source violates an existing constraint
- **WHEN** production TypeScript uses explicit `any` or `@ts-ignore`, a new production TypeScript file exceeds 300 physical lines, or production Rust uses a prohibited panic shortcut
- **THEN** the configured repository checks SHALL reject the source

#### Scenario: Historical oversized path is exempted from the default limit
- **WHEN** a production source path is exempted from the default file-size limit because it predates the limit
- **THEN** the exemption SHALL take the form of a recorded line budget that bounds the path, and SHALL NOT take the form of disabling the file-size rule for that path

#### Scenario: Test code uses a panic shortcut
- **WHEN** Rust test code uses `unwrap()` or `expect()`
- **THEN** the panic-shortcut check SHALL NOT reject it, and no per-module exemption SHALL be required to keep it passing

#### Scenario: A production panic shortcut predates the check
- **WHEN** a production Rust file carried a panic shortcut before the check existed
- **THEN** its exemption SHALL be recorded at that file with the reason and the work expected to retire it, rather than by weakening the check for all files

### Requirement: Recorded line budgets for oversized source paths
Every production source path that is exempt from the repository's default file-size limit SHALL carry a recorded numeric line budget instead of a disabled check. A budget SHALL be expressed as a pair: a **path budget** bounding the physical lines of one file or glob, and a **subtree budget** bounding the aggregate physical lines of the directory that contains it. The repository checks SHALL fail when either bound is exceeded.

#### Scenario: Exempt file grows beyond its recorded budget
- **WHEN** a production source path registered with a line budget is changed so that its physical line count exceeds that budget
- **THEN** the repository checks SHALL fail and the diagnostic SHALL report the path, its measured line count, and its recorded budget

#### Scenario: Exempt file shrinks
- **WHEN** a registered path's physical line count falls below its recorded budget
- **THEN** the repository checks SHALL pass without requiring the budget to be updated in the same change

#### Scenario: Unregistered production file exceeds the default limit
- **WHEN** a production source file that carries no recorded budget exceeds the repository's default file-size limit
- **THEN** the repository checks SHALL fail under the default limit, and the presence of budgets for other paths SHALL NOT exempt it

### Requirement: Line budgets survive directory-module refactoring
A registered path budget SHALL be satisfied when the path no longer exists, because the subtree budget continues to bound whatever replaced it. This allows a single oversized file to be converted into a directory module without the gate reporting a failure for the refactor itself.

#### Scenario: Oversized file is split into a directory module
- **WHEN** a registered path is replaced by a directory of smaller modules whose aggregate physical lines stay within the subtree budget
- **THEN** the repository checks SHALL pass

#### Scenario: Split duplicates code instead of moving it
- **WHEN** a registered path is split such that the containing subtree's aggregate physical lines exceed its subtree budget
- **THEN** the repository checks SHALL fail and the diagnostic SHALL report the subtree, its measured aggregate, and its recorded budget

### Requirement: Raising a line budget is an explicit reviewed edit
Line budgets SHALL NOT be raised automatically, inferred from the working tree, or regenerated by tooling. Lowering a budget SHALL require no justification. Raising a budget SHALL require an explicit edit to the recorded budget accompanied by a stated reason.

#### Scenario: Change needs more lines than a budget allows
- **WHEN** a change requires a registered path or subtree to exceed its recorded budget
- **THEN** the change SHALL edit the recorded budget and state the reason, rather than disable, delete, or widen the check

#### Scenario: Budget check diagnostic guides repair
- **WHEN** a line budget check fails
- **THEN** the diagnostic SHALL name the decomposition work that owns the path so the author can choose between reducing the change and raising the budget
