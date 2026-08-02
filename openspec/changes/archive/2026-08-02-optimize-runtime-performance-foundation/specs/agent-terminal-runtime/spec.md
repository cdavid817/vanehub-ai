## ADDED Requirements

### Requirement: Bounded incremental terminal transcript retention
The native retained terminal and frontend replay cache MUST store transcript output incrementally with a maximum retained size of 1 MiB per session.

#### Scenario: Append terminal output below the limit
- **WHEN** a terminal emits a new output chunk and the retained transcript remains below 1 MiB
- **THEN** the runtime SHALL append the new chunk without rebuilding the previously retained transcript

#### Scenario: Append terminal output above the limit
- **WHEN** a terminal output append would exceed 1 MiB of retained content
- **THEN** the runtime SHALL evict or trim only the oldest chunks needed to restore the bound
- **AND** the newest output SHALL remain available for replay

#### Scenario: Reattach to a retained terminal
- **WHEN** a terminal view requests cached or native retained output
- **THEN** the applicable chunk buffer SHALL create a replay snapshot at that boundary
- **AND** the existing duplicate-replay protection SHALL remain effective
