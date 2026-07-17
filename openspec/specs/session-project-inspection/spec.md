# session-project-inspection Specification

## Purpose
Defines confined, bounded, read-only inspection of session project files, documents, Git status, and structured diffs.

## Requirements
### Requirement: Session-root access confinement
Project inspection operations SHALL resolve their root from the registered session and MUST reject access outside the canonical session root.

#### Scenario: Read a valid relative path
- **WHEN** a request contains a relative path whose canonical target remains under the session root
- **THEN** the native runtime SHALL evaluate the bounded inspection operation for that target

#### Scenario: Reject parent traversal
- **WHEN** a relative path attempts parent traversal outside the session root
- **THEN** the native runtime SHALL reject the request with a concise validation error

#### Scenario: Reject symlink escape
- **WHEN** a path resolves through a symbolic link to a target outside the session root
- **THEN** the native runtime SHALL reject the request and SHALL NOT return target metadata or content

#### Scenario: Resolve missing session root
- **WHEN** the selected session has no available project or working folder
- **THEN** the service SHALL return a typed unavailable result rather than inspecting an arbitrary process directory

### Requirement: Lazy project file tree
The Files tab SHALL load immediate directory children on demand with deterministic sorting and bounded results.

#### Scenario: Expand a directory
- **WHEN** the user expands a directory node
- **THEN** the service SHALL return its non-hidden immediate children with directories before files and names sorted deterministically

#### Scenario: Reach directory entry limit
- **WHEN** a directory contains more entries than the configured bound
- **THEN** the response SHALL mark the result as truncated and the UI SHALL display a localized partial-result state

#### Scenario: Collapse and reopen a directory
- **WHEN** the user collapses and reopens a previously loaded directory in the same mounted tab
- **THEN** the tree SHALL retain its expanded-node and selection state unless its query was invalidated

### Requirement: Bounded read-only file preview
The Files and Documents tabs SHALL preview supported text files without exposing binary or oversized content.

#### Scenario: Read a supported text file
- **WHEN** the selected file is text and no larger than 1 MiB
- **THEN** the service SHALL return decoded content and metadata for read-only display

#### Scenario: Reject oversized content
- **WHEN** the selected file exceeds 1 MiB
- **THEN** the service SHALL return an oversized marker without loading the full content into the frontend

#### Scenario: Detect binary content
- **WHEN** the selected file is detected as binary
- **THEN** the service SHALL return a binary marker and SHALL NOT return its raw bytes as text

#### Scenario: File changes during read
- **WHEN** a file disappears or becomes inaccessible before content is read
- **THEN** the UI SHALL show a concise localized error and remain usable for another selection

### Requirement: Structured Git status
The Changes tab SHALL expose structured index and worktree status for the selected session root when it is a Git repository.

#### Scenario: Show changed paths
- **WHEN** Git reports modified, added, deleted, renamed, conflicted, or untracked paths
- **THEN** the service SHALL preserve index and worktree status separately for each path

#### Scenario: Display changed-path status
- **WHEN** Changes renders a structured Git status entry
- **THEN** the UI SHALL show conventional index/worktree codes and localized status labels that distinguish unmodified, modified, added, deleted, renamed, copied, conflicted, and untracked states

#### Scenario: Non-Git session
- **WHEN** the selected session root is not a Git repository
- **THEN** Changes SHALL show a localized non-Git empty state rather than a raw command failure

#### Scenario: Git command fails
- **WHEN** Git inspection fails for another reason
- **THEN** the native runtime SHALL persist redacted diagnostics through unified logging and return a concise error through the service boundary

### Requirement: Unified and split Git diff views
The Changes tab SHALL render working-tree and staged diffs from one structured file/hunk/line model in unified or split view.

#### Scenario: Select diff source
- **WHEN** a path has staged and working-tree changes
- **THEN** the user SHALL be able to inspect each source without combining their hunks ambiguously

#### Scenario: Switch diff view
- **WHEN** the user switches between unified and split view
- **THEN** the viewer SHALL reuse the same structured diff and preserve the selected file and scroll context where practical

#### Scenario: View untracked text file
- **WHEN** an untracked text file is within the content bound
- **THEN** the diff SHALL represent its lines as additions against an empty file

#### Scenario: View binary or oversized diff
- **WHEN** a changed file is binary or exceeds the configured diff bound
- **THEN** the viewer SHALL show status metadata without attempting to render textual hunks

#### Scenario: Parse renamed file
- **WHEN** Git reports a rename
- **THEN** the structured diff SHALL preserve old and new paths for display
