## ADDED Requirements

### Requirement: Prompt delivery suits the resolved executable
A provider adapter SHALL choose prompt delivery so the prompt survives the host's process-creation rules for the executable that will actually run, rather than fixing one channel per Agent.

Where a managed CLI resolves to a platform script wrapper rather than a native binary, the prompt SHALL NOT be delivered as a command-line argument. Two host limits make that channel unsound for such a wrapper: process creation refuses arguments containing line breaks, and the command line is bounded well below the size a composed prompt can reach — past that bound the launch succeeds while the wrapper receives nothing, losing the prompt with no error.

An adapter that moves a prompt off the command line SHALL keep the remaining invocation arguments, resume mapping, output parsing, cancellation, usage accounting and terminal behaviour unchanged.

#### Scenario: Managed CLI resolves to a script wrapper
- **WHEN** a managed CLI Agent's executable resolves to a platform script wrapper with no native binary beside it
- **AND** the composed prompt spans more than one line
- **THEN** the adapter SHALL deliver the prompt on standard input
- **AND** the turn SHALL reach the provider rather than failing at process creation

#### Scenario: Prompt exceeds the command-line bound
- **WHEN** a composed prompt is longer than the host's command-line bound for the resolved executable
- **THEN** the adapter SHALL NOT place it in the command line
- **AND** the prompt SHALL be delivered whole

### Requirement: Unrecognised structured output is not Agent speech
A provider parser SHALL NOT publish an event it does not model as the Agent's own words. Where a line is structured output whose type the parser has no handling for, it SHALL resolve to no output.

The fallback that treats a line as literal Agent text SHALL apply only to output that is not structured at all, which is the case it exists for. A single turn can carry many envelope events around the one that holds the reply, so publishing unmodelled envelopes verbatim replaces the answer with protocol noise.

#### Scenario: Provider emits an envelope the parser does not model
- **WHEN** a provider emits a structured event whose type the parser has no handling for
- **THEN** the parser SHALL produce no output for that line
- **AND** the reply the turn does carry SHALL reach the user unchanged

#### Scenario: Provider emits unstructured text
- **WHEN** a provider emits a line that is not structured output
- **THEN** the parser SHALL surface it as Agent text
