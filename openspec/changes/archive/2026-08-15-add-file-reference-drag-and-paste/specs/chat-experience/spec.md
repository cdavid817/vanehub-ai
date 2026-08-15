## ADDED Requirements

### Requirement: Attach file references by dragging or pasting a workspace path
The Files tab SHALL be able to hand a file path to the chat composer by drag-and-drop and through the clipboard, so that a file already visible in the workspace does not have to be retyped as a mention.

#### Scenario: Drag a file onto the composer
- **WHEN** a user drags a file row from the Files tab and drops it on the chat composer
- **THEN** a reference to that file SHALL be attached
- **AND** the reference SHALL carry no line range

#### Scenario: Directories are not draggable
- **WHEN** a user attempts to drag a directory row
- **THEN** no drag SHALL start and nothing SHALL be attachable from it

#### Scenario: The drop target is discoverable
- **WHEN** a draggable file is over the composer
- **THEN** the composer SHALL show that it will accept the drop
- **AND** the affordance SHALL disappear when the pointer leaves or the drop completes

#### Scenario: Copy a path from the Files tab
- **WHEN** a user invokes the copy-path action on a file row
- **THEN** the session-relative path SHALL be placed on the clipboard as plain text
- **AND** it SHALL additionally carry the application's file-path clipboard type

#### Scenario: Paste a copied path into the composer
- **WHEN** a user pastes clipboard content carrying the application's file-path clipboard type into the composer
- **THEN** a reference to that file SHALL be attached instead of the path being inserted as text
- **AND** the reference SHALL carry no line range

#### Scenario: Ordinary paste is unaffected
- **WHEN** a user pastes content that does not carry that clipboard type
- **THEN** the composer SHALL insert it as text exactly as it did before
- **AND** the composer SHALL NOT inspect pasted text to guess whether it names a file

#### Scenario: Pasting a copied path elsewhere
- **WHEN** a copied path is pasted into any target other than the composer
- **THEN** it SHALL arrive as the plain-text path

#### Scenario: Attached the same way as a typed mention
- **WHEN** a reference is attached by drop or paste
- **THEN** duplicate detection, the maximum number of references per message, and chip display SHALL behave exactly as for a reference attached by typing a mention

#### Scenario: Dropping or pasting while sending is not allowed
- **WHEN** the composer is disabled or a generation is streaming
- **THEN** a drop or a path paste SHALL NOT attach a reference

#### Scenario: External sources are not accepted
- **WHEN** content is dragged or pasted from outside the application
- **THEN** it SHALL NOT be treated as a file reference
- **AND** ordinary text handling SHALL apply
