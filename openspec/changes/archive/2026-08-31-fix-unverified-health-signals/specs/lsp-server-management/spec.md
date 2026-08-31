## ADDED Requirements

### Requirement: Server test cleanup is budgeted rather than left over
The isolated server test's cleanup phase SHALL receive a minimum budget of its own, applied when the
caller's deadline is already spent. Cleanup SHALL report success only when the child process was
observed to have ended, and SHALL report the forced disposition when termination was forced.

The deadline a caller supplies bounds the work it asked for. Cleanup is not that work — it is what
this code owes the machine afterwards, and giving it the remainder means a slow spawn leaves it
nothing. What follows is not a slow cleanup but a false one: the kill is issued, the wait is skipped
because there is no time to wait in, and the phase reports failure for a child that did in fact die.
A caller told cleanup failed has no way to distinguish that from a process tree still running.

The floor is a bound, not an extension: a cleanup that completes inside the caller's remaining
budget SHALL NOT be made to wait for the floor.

#### Scenario: The caller's deadline is spent before cleanup begins
- **WHEN** discovery, spawn and initialize consume the whole timeout the caller supplied
- **THEN** cleanup SHALL still receive its minimum budget
- **AND** it SHALL report a succeeded phase with the forced-termination reason once the child has ended

#### Scenario: Cleanup finishes inside the caller's budget
- **WHEN** the child ends while the caller's deadline still has time remaining
- **THEN** cleanup SHALL return at that moment
- **AND** it SHALL NOT wait out the minimum budget

#### Scenario: The child does not end within the floor
- **WHEN** a child has not ended by the end of the cleanup floor
- **THEN** cleanup SHALL report a failed phase
- **AND** the result SHALL NOT claim the process tree was cleaned up
