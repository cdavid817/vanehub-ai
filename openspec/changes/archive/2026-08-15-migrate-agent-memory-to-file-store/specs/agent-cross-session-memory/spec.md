## ADDED Requirements

### Requirement: Memories are addressable files

The system SHALL store each memory as one Markdown file in a single host-level memory directory, rather than as a row in a table. The file SHALL carry frontmatter with a `name`, a one-line `description` used to judge relevance without reading the body, and a `type`; the body below the frontmatter SHALL be the memory content. Provenance — the producing agent id, the workspace folder when one was in scope, the save source, and the creation time — SHALL be recorded in the same frontmatter, and SHALL remain provenance only rather than a read filter, exactly as it was on the row store.

#### Scenario: A saved memory becomes a file

- **WHEN** a memory is saved by any path (the explicit tool, OnePiece's automatic extraction, or a CLI-wrapped agent's automatic extraction)
- **THEN** the system SHALL write one Markdown file into the host-level memory directory whose frontmatter carries a name, a description, a type, and the producing agent id
- **AND** that file SHALL be the sole source of truth for that memory's content

#### Scenario: A memory is addressed by name

- **WHEN** any read, update, or delete operation refers to a stored memory
- **THEN** it SHALL identify that memory by its directory-relative file path
- **AND** two memories SHALL NOT share one path

#### Scenario: Malformed file does not break enumeration

- **WHEN** a file in the memory directory has absent, unparseable, or incomplete frontmatter
- **THEN** the system SHALL skip that file and continue enumerating the rest
- **AND** it SHALL NOT fail the generation, the injection, or the management view that triggered the enumeration

### Requirement: Memory type taxonomy

The system SHALL constrain a memory's `type` to a closed set of four values — `user`, `feedback`, `project`, and `reference`. A file whose `type` is absent or unrecognized SHALL remain readable, injectable, and manageable, degrading to an untyped memory rather than being rejected, so that migrated files and hand-written files keep working.

#### Scenario: A recognized type is preserved

- **WHEN** a memory file declares one of the four recognized types
- **THEN** the system SHALL retain that type through enumeration, injection, and the management view

#### Scenario: An absent or unknown type degrades

- **WHEN** a memory file declares no `type`, or declares a value outside the four recognized ones
- **THEN** the system SHALL treat it as untyped
- **AND** it SHALL still be enumerated, injected, and listed

### Requirement: Memory index file

The system SHALL maintain a `MEMORY.md` index in the memory directory holding one line per memory: a pointer to that memory's file plus a short hook. The index SHALL NOT be a memory itself and SHALL NOT carry frontmatter, and memory content SHALL NOT be written into it. Every path that creates or deletes a memory SHALL keep the index consistent with the directory.

#### Scenario: Saving a memory updates the index

- **WHEN** a memory file is created
- **THEN** the system SHALL add exactly one corresponding line to `MEMORY.md`

#### Scenario: Deleting a memory updates the index

- **WHEN** a memory file is deleted
- **THEN** the system SHALL remove its line from `MEMORY.md`

#### Scenario: Index and directory disagree

- **WHEN** enumeration finds an index line whose target file does not exist, or a memory file with no index line
- **THEN** the system SHALL reconcile the index to match the directory
- **AND** the directory SHALL be authoritative

### Requirement: Model-side memory correction

The system SHALL expose the memory directory to OnePiece's generic file tools as an auto-approved read and write scope, so the model can read an existing memory, correct it in place, or delete one that turned out to be wrong. Paths outside the memory directory SHALL keep whatever approval they required before this change.

#### Scenario: Model corrects an existing memory

- **WHEN** the model reads a memory file and writes an updated version of it during a generation while the memory enablement toggle is on
- **THEN** the system SHALL apply the write without requiring user approval
- **AND** subsequent injections and listings SHALL reflect the updated content

#### Scenario: Model deletes a stale memory

- **WHEN** the model deletes a memory file
- **THEN** the system SHALL remove it from the index and revoke its retrieval index entry
- **AND** it SHALL NOT be injected into any agent's future generations

#### Scenario: Write outside the memory directory is unaffected

- **WHEN** the model writes to a path outside the memory directory
- **THEN** the approval behavior SHALL be exactly what it was before this change

#### Scenario: Memory-directory writes are inert when memory is disabled

- **WHEN** the model writes to the memory directory while the memory enablement toggle is off
- **THEN** the system SHALL reject the write, mirroring the explicit save tool's behavior under the same toggle

### Requirement: Migration from the row store

The system SHALL convert every previously stored memory row into a memory file without making a model call, preserving the row's content as the file body and its provenance as frontmatter. A migrated file SHALL receive a generated name and a description derived from its content, and SHALL be left untyped rather than assigned a guessed type. Migration SHALL be idempotent and SHALL NOT delete or rewrite files it has already produced.

#### Scenario: Existing rows migrate

- **WHEN** the memory directory is initialized on an installation that has stored memory rows
- **THEN** each row SHALL become one memory file preserving its content, producing agent id, workspace folder, save source, and creation time
- **AND** each migrated file SHALL appear in `MEMORY.md`

#### Scenario: Migration runs again

- **WHEN** migration runs a second time
- **THEN** it SHALL NOT produce duplicate files and SHALL NOT overwrite a file that the model or the user has since edited

#### Scenario: One row fails to migrate

- **WHEN** a single row cannot be converted
- **THEN** the system SHALL log the failure and continue migrating the remaining rows
- **AND** it SHALL NOT abort startup

## MODIFIED Requirements

### Requirement: Explicit memory saving

The system SHALL provide a tool the model can call to save a memory, auto-approved without requiring user confirmation, while the memory enablement toggle is on. The tool SHALL accept the memory's name, description, type, and content, and SHALL write the corresponding memory file and its index entry as one operation. Its name and its position in the tool catalog SHALL be unchanged by this change, so that the declared tool prefix stays byte-identical. This tool is exposed only to OnePiece's own API tool-calling loop; CLI-wrapped agents produce memories through the separate mechanism governed by the "Automatic memory extraction for CLI-wrapped agents" requirement, since VaneHub does not control a CLI-wrapped agent's own internal tool system.

#### Scenario: Model saves a memory via the tool

- **WHEN** the model calls the memory-saving tool with a name, description, type, and content during a generation and the memory enablement toggle is on
- **THEN** the system SHALL write the memory file and its index entry immediately without requiring approval
- **AND** it SHALL be available to future sessions for every agent, per the shared host-level pool

#### Scenario: Tool is inert when memory is disabled

- **WHEN** the model calls the memory-saving tool while the memory enablement toggle is off
- **THEN** the system SHALL reject the call without writing anything

#### Scenario: Saving over an existing name

- **WHEN** the model saves a memory whose name matches an existing memory file
- **THEN** the system SHALL replace that file's content rather than creating a second file for the same name
- **AND** `MEMORY.md` SHALL continue to hold exactly one line for it

#### Scenario: Tool catalog ordering is preserved

- **WHEN** the tool catalog is resolved for a generation
- **THEN** the memory-saving tool SHALL appear under its existing name at its existing position

### Requirement: Automatic memory extraction

The system SHALL attempt best-effort automatic extraction of memorable content from turns that context compaction is about to replace, without failing the generation if extraction fails, while the memory enablement toggle is on and, when the compacted turns include a tool call, only while the tool-assisted chat extraction toggle is also on. Extraction SHALL be a single model call with no tool access, and SHALL return a bounded list of create, update, and delete actions naming the memory files they apply to. The system SHALL validate every returned action — rejecting any that names a path outside the memory directory or omits required metadata — before applying the surviving actions. This requirement governs only OnePiece's compaction-triggered extraction; CLI-wrapped agents have no VaneHub-visible compaction signal and are governed instead by the separate "Automatic memory extraction for CLI-wrapped agents" requirement.

#### Scenario: Extraction runs when compaction triggers

- **WHEN** context compaction triggers during a generation and both applicable toggles allow it
- **THEN** the system SHALL make one additional call to extract memorable facts from the turns being compacted
- **AND** the surviving validated actions SHALL be applied to the same memory directory the explicit tool writes to

#### Scenario: Extraction updates an existing memory instead of duplicating it

- **WHEN** the extraction call is given the existing memories' names and descriptions and returns an update action naming one of them
- **THEN** the system SHALL replace that memory file's content rather than creating a second memory covering the same fact

#### Scenario: Extraction returns an invalid action

- **WHEN** a returned action names a path outside the memory directory, or omits a required metadata field
- **THEN** the system SHALL reject that action, apply the remaining valid ones, and log the rejection
- **AND** it SHALL NOT fail the generation

#### Scenario: Extraction finds nothing worth remembering

- **WHEN** the extraction call returns an empty action list
- **THEN** the system SHALL write no memories from that extraction, without treating this as a failure

#### Scenario: Extraction failure does not affect compaction

- **WHEN** the extraction call itself fails
- **THEN** the system SHALL log the failure and continue the generation and its compaction unaffected

#### Scenario: Extraction skipped when memory is disabled

- **WHEN** context compaction triggers while the memory enablement toggle is off
- **THEN** the system SHALL NOT make an extraction call

#### Scenario: Extraction skipped for a tool-assisted session when the sub-toggle is off

- **WHEN** context compaction triggers, the memory enablement toggle is on, the tool-assisted chat extraction toggle is off, and the compacted turns include a tool call
- **THEN** the system SHALL NOT make an extraction call for that compaction

### Requirement: Memory injection into the system prompt

The system SHALL inject the shared host-level memory pool into OnePiece's generation requests as part of the system prompt while the memory enablement toggle is on, bounded by a character budget, and SHALL never write memory content into the turns list context compaction manipulates. The pool SHALL be enumerated from the memory directory, and ordered by each memory's last modification time so that a memory the model has just corrected is treated as the most recent. The injected text for one memory SHALL be its body. This requirement governs only OnePiece's system-prompt injection; CLI-wrapped agents are governed instead by the separate "Memory injection into CLI prompts" requirement.

#### Scenario: Memories injected alongside Skill content

- **WHEN** a generation runs for an agent with both bound Skills and stored memories in scope, and the memory enablement toggle is on
- **THEN** the system prompt SHALL include both, as distinct sections

#### Scenario: Injected memories are bounded

- **WHEN** the memory directory's total content exceeds the injection character budget
- **THEN** the system SHALL include memories by recency up to the budget rather than including all of them unbounded

#### Scenario: A corrected memory becomes the most recent

- **WHEN** a memory that was written long ago is updated during a session
- **THEN** the next injection SHALL order it ahead of memories that have not been modified since

#### Scenario: Memory content survives compaction

- **WHEN** context compaction triggers during a generation with injected memory content
- **THEN** the injected memory content SHALL remain present, complete, and unchanged on every subsequent request of that generation

#### Scenario: No injection when memory is disabled

- **WHEN** the memory enablement toggle is off
- **THEN** the system SHALL NOT read the memory directory for injection and SHALL send the request without a memory section

### Requirement: Memory management

The system SHALL let a user list every stored memory in the shared host-level pool, delete one at a time, and delete every stored memory in a single action, regardless of which agent produced them. A listed memory SHALL carry its name, description, and type alongside its content, source, producing agent, and creation time, so that a user can tell entries apart without reading each body in full.

#### Scenario: List an agent's memories

- **WHEN** a user requests the stored memories
- **THEN** the system SHALL return every stored memory in the shared pool, each with its name, description, type, content, source, producing agent, and creation time

#### Scenario: Delete a memory

- **WHEN** a user deletes a stored memory
- **THEN** the system SHALL remove its file and its index line
- **AND** it SHALL no longer be injected into any agent's future generations

#### Scenario: Reset all of an agent's memories

- **WHEN** a user confirms resetting stored memories
- **THEN** the system SHALL remove every stored memory in the shared pool, regardless of which agent produced it
- **AND** none of them SHALL be injected into any agent's future generations
- **AND** the confirmation prompt SHALL make clear that this affects every agent's memories, not only the one currently being viewed

#### Scenario: Reset is scoped to one agent

- **WHEN** a user confirms resetting stored memories
- **THEN**, reversing the single-agent scoping this scenario previously guaranteed, stored memories belonging to every other agent SHALL be removed too — reset is no longer scoped to one agent, since there is no longer a per-agent scope to reset

#### Scenario: Memory edited outside the application

- **WHEN** a user edits or removes a memory file directly on disk and then opens the management view
- **THEN** the listing SHALL reflect the on-disk state without requiring a restart

### Requirement: Automatic memory extraction for CLI-wrapped agents

The system SHALL, after a CLI-wrapped agent's generation completes successfully and while the memory enablement toggle is on, attempt best-effort automatic extraction of memorable content from that turn's exchange by making an independent model call using OnePiece's currently configured provider credentials. Extracted memories SHALL be written to the same memory directory, attributed to the CLI-wrapped agent that produced them. The system SHALL NOT instruct a CLI-wrapped agent to write memory files itself, because several of them ship their own memory systems and a second set of persistence instructions in the same prompt would conflict with them. This extraction SHALL NOT block or delay delivery of the CLI generation's own result to the user, and SHALL NOT depend on or alter OnePiece's own compaction-triggered extraction mechanism.

#### Scenario: Extraction runs after a CLI generation completes

- **WHEN** a CLI-wrapped agent's generation completes successfully, the memory enablement toggle is on, and OnePiece has a usable configured credential
- **THEN** the system SHALL make one independent call to extract memorable facts from that turn's exchange
- **AND** any extracted facts SHALL be written as memory files in the shared host-level memory directory, attributed to the CLI-wrapped agent that produced them

#### Scenario: CLI prompt is not told about the memory directory

- **WHEN** a message is sent to a CLI-wrapped agent
- **THEN** the delivered text SHALL NOT contain the memory directory path or instructions to write memory files there

#### Scenario: Extraction finds nothing worth remembering

- **WHEN** the extraction call determines there is nothing worth remembering from that turn
- **THEN** the system SHALL write no memory from that extraction, without treating this as a failure

#### Scenario: Extraction is skipped without a usable OnePiece credential

- **WHEN** a CLI-wrapped agent's generation completes and OnePiece has no usable configured credential
- **THEN** the system SHALL log the condition and skip extraction for that turn
- **AND** it SHALL NOT affect the CLI generation, which has already completed and been delivered

#### Scenario: Extraction call failure does not affect the already-delivered CLI result

- **WHEN** the independent extraction call itself fails
- **THEN** the system SHALL log the failure and skip writing a memory for that turn
- **AND** the CLI generation's own result, already delivered to the user, SHALL be unaffected

#### Scenario: Extraction is skipped when memory is disabled

- **WHEN** the memory enablement toggle is off
- **THEN** the system SHALL NOT make an extraction call after a CLI-wrapped agent's generation completes

### Requirement: Deleting a memory revokes its retrieval index

The system SHALL revoke the retrieval index entry for a deleted memory, and SHALL NOT return deleted memories from retrieval even if an index entry survives. Because a memory is now a file that can be removed without the application's participation, retrieval results SHALL be resolved against the memory directory, and reconciliation SHALL treat a file's absence from the directory as a deletion.

#### Scenario: Memory deleted while index revocation fails

- **WHEN** a memory is deleted and its index revocation call fails
- **THEN** retrieval SHALL NOT return that memory, because results are resolved against the memory directory
- **AND** background reconciliation SHALL remove the orphaned index entry

#### Scenario: Memory file removed outside the application

- **WHEN** a memory file is deleted on disk while the application is not running, and the application is started again
- **THEN** retrieval SHALL NOT return that memory
- **AND** reconciliation SHALL remove its index entry and its `MEMORY.md` line
