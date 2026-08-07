## ADDED Requirements

### Requirement: Contained external command termination

The native runtime SHALL spawn every externally-executed command on the bounded execution path into a platform process-containment primitive so the runtime can reach processes that command spawns. When the runtime terminates such a command because it exceeded its timeout or was cancelled, it SHALL terminate the entire contained process tree rather than only the process it launched directly.

The runtime SHALL continue to decide that a command has *completed* from the exit of the process it launched directly, and SHALL NOT wait for that process's descendants before returning a result. A command that exits successfully SHALL NOT have its surviving descendants terminated, so callers that deliberately launch a background process keep today's behavior.

#### Scenario: Timed-out command leaves a descendant running

- **WHEN** an external command on the bounded execution path exceeds its timeout and the process it launched has itself spawned another process
- **THEN** the native runtime SHALL terminate both the launched process and its descendants
- **AND** it SHALL report the timeout failure to the caller as it does today

#### Scenario: Cancelled command leaves a descendant running

- **WHEN** an external command on the bounded execution path is cancelled and the process it launched has itself spawned another process
- **THEN** the native runtime SHALL terminate both the launched process and its descendants
- **AND** it SHALL report the cancellation to the caller as it does today

#### Scenario: Successful command leaves a background process

- **WHEN** an external command on the bounded execution path exits successfully while a process it spawned is still running
- **THEN** the native runtime SHALL report the command as completed without waiting for that surviving process
- **AND** it SHALL NOT terminate that surviving process

#### Scenario: Containment is unavailable for a command

- **WHEN** the native runtime cannot establish process containment for a command it is about to launch
- **THEN** it SHALL NOT leave a started process unsupervised, and SHALL surface the failure as a launch failure rather than silently running the command without containment
