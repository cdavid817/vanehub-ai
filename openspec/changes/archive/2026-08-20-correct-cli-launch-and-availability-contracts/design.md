## Context

See proposal.md — Why. Three details shape the approach rather than the motivation.

The four defects share a structure: a rule stated once, applied to values of different kinds. One control-character rule covered arguments, executables, paths and environment values. One length bound covered identifiers and resolved paths. One availability order treated the SDK and the executable as interchangeable evidence. One parser fallback covered structured events and plain text. In each case the rule is right for one kind of value and wrong for the other.

The host constrains two of them. Since the BatBadBut hardening, `std::process::Command` refuses to pass a `.bat`/`.cmd` any argument containing CR or LF, before `CreateProcess` runs; and `cmd.exe` truncates a command line at 8,191 characters, well inside the 16,384 an argument is permitted, with the truncation silent. Neither is negotiable from inside the product.

The verification harness reaches all of this only through the desktop client. The Web adapter answers these commands from memory, so none of the four is observable there.

## Goals / Non-Goals

**Goals:**

- Separate the value kinds that the four rules conflate, so each keeps the bound that suits it.
- Make a refused launch name its constraint, in the error and in the unified log.
- Keep the security properties that motivated the original rules: no shell traversal, no truncation at the process boundary, no unapproved secret forwarding.

**Non-Goals:**

- Streaming granularity for claude-code. Emitting tokens from `content_block_delta` would make it stream incrementally, but risks double-counting against the final `assistant` event; that is its own change.
- Resolving script wrappers to their underlying interpreter. It would keep argv delivery alive for gemini-cli, but requires the executable resolver to return prefix arguments as well as a path, and leaves the same failure one missing `.exe` away.
- Raising `MAX_RUNNER_ARGUMENT_CHARS`. With the prompt off the command line for script wrappers, the existing bound is no longer the binding constraint.

## Decisions

**Two validators rather than one relaxed validator.** Arguments validate as text; the executable, cwd and environment values keep the strict rule. The alternative — relaxing the single validator — would have admitted line breaks into an environment value and a working directory, where they signal a malformed value rather than content. Splitting also keeps the security intent legible: the reason a newline is safe in an argument is that arguments are passed as an array, and that reasoning does not transfer to the other fields.

**The executable measured against the path bound, not a new bound.** `cwd` is the same kind of value and already used `MAX_RUNNER_ARGUMENT_CHARS`; reusing it keeps one bound for paths instead of introducing a third number to keep in step.

**Availability inverted rather than special-cased per Agent.** The executable decides; the SDK decides only when no executable is declared. The alternative — exempting the two SDK-declaring Agents — would leave the same trap for the next Agent that declares one. `agent-terminal-runtime` already required the terminal path to ignore a missing SDK, so this makes the registry agree with a rule the product had already accepted elsewhere.

**gemini-cli moved to stdin rather than the executable resolved to `node`.** The CLI documents `-p` as appended to stdin input, and with no `-p` reads stdin as the prompt, so this is the same request through a channel with neither the line-break nor the length limit. Resolving to the interpreter would work today and break again whenever a shim's binary lookup fails — which `opencode` already falls back to.

**Unrecognised structured events resolve to empty rather than being enumerated.** Adding an arm per observed event type would fix the symptom for the types seen and reintroduce it for the next one the CLI adds. Treating "parsed as JSON but unmodelled" as "not Agent speech" is the invariant; the raw-text fallback keeps working for genuinely unstructured output.

**The refusal reason threaded through the error, not logged at the rejection site.** The application layer does not log; a `detail` on the error lets the infrastructure layer that already writes the lifecycle line include it, without inverting the dependency.

## Risks / Trade-offs

**Tab, CR and LF are now accepted where they previously were not.** The mitigation is that arguments never traverse a shell — they are handed to the OS as an array — and NUL, the character that can actually truncate a value at the process boundary, stays rejected. A reviewer should confirm no consumer of `RunnerLaunchSpec` re-serialises arguments into a single string; the remote command contract encodes them separately and rejects what it cannot encode.

**An Agent can now be reported available and still fail at launch.** Availability answers "is the binary here", which is what the user can act on; whether the provider then accepts the account is a different question with a different remedy. The verification pass shows both shapes: codex-cli became available and works, gemini-cli became launchable and is refused by its provider for account reasons.

**gemini-cli's prompt no longer appears in argv**, so anything that inspected the command line to recover it — a diagnostic, a redaction test — sees less. The unified log already redacted prompt content, so this narrows what is available to diagnose rather than changing what is exposed.

**Suppressing unmodelled events can hide a provider contract change.** If a CLI moves its reply into an event type the parser does not model, the turn now produces nothing instead of producing noise. Nothing-instead-of-noise is the better failure, but it is quieter; the desktop session specs assert a real reply per Agent, which is what would catch it.
