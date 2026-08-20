## Why

A desktop verification pass against a real built client found that three of the five managed CLI Agents could not complete a single chat turn, and a fourth could not be selected at all. Each cause traced to a rule the specs state more broadly than the runtime can honour: arguments must carry no control character (a composed prompt always has newlines), a missing managed SDK marks an Agent unavailable (nothing on the execution path loads that package), and an unrecognised provider event falls back to being shown as the Agent's words (the CLI emits nine such envelopes per turn).

This change corrects those three contracts so the specs describe what a working runtime actually does. It affects the desktop runtime only; the Web adapter answers from mock data and reaches none of these paths.

## What Changes

- Argument validation distinguishes text from identifiers. Tab, CR and LF are admitted in launch **arguments**; every other control character, and the whole control range in the executable, cwd and environment values, stays rejected. Arguments reach the OS as an array and never traverse a shell, while NUL still truncates a C string.
- Managed SDK status stops vetoing availability. An Agent whose executable resolves on PATH is available whatever its SDK status; SDK status decides only when no executable is declared, where it is the sole evidence. This aligns the registry with `agent-terminal-runtime`'s existing "Missing managed SDK does not block CLI terminal startup".
- **BREAKING (adapter contract)** gemini-cli delivers its prompt on stdin rather than in argv. On Windows it is an npm batch shim with no `.exe`, and `std::process::Command` refuses to pass a `.bat`/`.cmd` any argument containing CR or LF.
- An unrecognised structured provider event resolves to no output instead of being emitted verbatim as reply text. The raw-text fallback continues to cover output that is not structured at all.
- The launch refusal names the constraint it tripped, so `runner_invalid_launch` is actionable.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `agent-runner-runtime`: the control-character prohibition in command construction is narrowed to exclude tab, CR and LF in arguments, and a refusal must identify the constraint that produced it.
- `agent-tool-registry`: a missing managed SDK no longer marks an Agent unavailable when its executable is present; it decides availability only for an Agent that declares no executable.
- `agent-provider-runtime`: prompt delivery must suit the resolved executable rather than being fixed per Agent, and unrecognised structured output must not be published as Agent text.

## Impact

Desktop runtime only.

- `contexts/agent_runtime/application/runner.rs` — argument validation, executable measured against the path budget, `RunnerError` carries the refused constraint.
- `contexts/agent_runtime/domain/catalog.rs` — availability assessment order.
- `contexts/agent_runtime/infrastructure/providers/invocation.rs` and `output.rs` — gemini prompt delivery, unrecognised-event handling.
- `contexts/agent_runtime/infrastructure/{local_runner,process_adapter}.rs` — spawn failures carry their reason into the unified log.
- No frontend, service-boundary, or Web-adapter change: the affected surfaces are all behind existing Tauri commands whose signatures are unchanged.
- Provider invocation fixtures (`providers/fixtures/invocations.json`) encode prompt delivery per Agent and move with the gemini change.
