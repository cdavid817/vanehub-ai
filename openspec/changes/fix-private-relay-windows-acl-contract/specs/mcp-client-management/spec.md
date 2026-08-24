## MODIFIED Requirements

### Requirement: Protected transient MCP relay configuration
The desktop runtime MUST store invocation-scoped relay configuration in a private, uniquely named VaneHub-owned directory and MUST clean every owned artifact without changing the existing plaintext SQLite or export contract. On Windows, "private" MUST be defined and verified structurally — owner, principals, access masks, inheritance and DACL protection — rather than by the textual rendering of a security descriptor.

#### Scenario: Create relay artifacts
- **WHEN** the runtime prepares relay files containing MCP environment values, headers, database location, or execution context
- **THEN** it MUST create a unique per-invocation directory with current-user-only access before writing secret-bearing bytes
- **AND** it MUST use exclusive file creation with unpredictable names

#### Scenario: Windows relay artifacts grant only the current user
- **WHEN** the runtime creates a relay directory or file on Windows
- **THEN** the object's DACL MUST be present rather than NULL, because a NULL DACL grants full control to everyone
- **AND** it MUST be protected, so that later inheritance from the parent cannot widen it
- **AND** it MUST contain exactly one access-allowed entry, for the current user's SID, granting full access, with no inherited entries and no additional principal
- **AND** the owner MUST be the current user, because an owner may rewrite the DACL regardless of its contents

#### Scenario: The privacy check reports what it found
- **WHEN** the Windows relay access check fails
- **THEN** it MUST report the current user SID, the owner SID, whether the DACL is present or NULL, whether it is protected and whether it is auto-inherited, and the ordered ACE list
- **AND** each entry MUST report allow or deny, explicit or inherited, its SID, its access mask, and its inheritance flags
- **AND** it MUST report the expected contract in the same terms, so the failure states a difference rather than a boolean
- **AND** raw SDDL MAY be included as supplementary evidence but MUST NOT be the assertion

#### Scenario: The privacy check rejects a widened DACL
- **WHEN** a relay object's DACL carries an entry for Everyone, Users, or Authenticated Users, omits the current user, is unprotected, carries inherited entries, grants a mask other than full access, or orders its entries non-canonically
- **THEN** the check MUST fail
- **AND** principals MUST be compared by SID rather than by display name, which is locale-dependent and ambiguous across domains

#### Scenario: Relay preparation partially fails
- **WHEN** one server or provider configuration fails after earlier relay artifacts were created
- **THEN** the runtime MUST remove every artifact already owned by that preparation before returning the failure

#### Scenario: Relay invocation terminates
- **WHEN** an Agent invocation completes, fails, is cancelled, or times out
- **THEN** the owning relay guard MUST idempotently remove its provider and per-server configuration artifacts after verifying their canonical paths remain inside the dedicated relay root

#### Scenario: Relay helper consumes its configuration
- **WHEN** a relay helper successfully opens its configuration file
- **THEN** it SHALL unlink that file before connecting to the upstream MCP server

#### Scenario: Recover stale relay artifacts
- **WHEN** desktop startup finds a versioned VaneHub-owned relay invocation directory older than 24 hours
- **THEN** the runtime SHALL remove it only after canonical-root verification and SHALL log metadata-only cleanup counts
- **AND** it MUST NOT delete unrelated system temporary files
