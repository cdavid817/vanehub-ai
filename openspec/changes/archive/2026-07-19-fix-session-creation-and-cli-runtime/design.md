## Overview

The change keeps interaction modes as internal/runtime data while making session creation user-oriented. The UI should ask for agent, workspace location, and project folder; it should derive a useful default name and rely on the selected agent/runtime profile to determine implementation details.

## Decisions

- Treat `cli` and `native-desktop` as internal interaction mode values in the create-session flow; do not render them as primary user choices.
- Normalize display paths with the existing frontend/backend boundary instead of altering real filesystem paths passed to native APIs.
- Generate the default name in the service/UI layer from the selected workspace folder basename plus a stable timestamp format.
- Opening-method management will separate "configure default/order" from "launch opener"; changing a dropdown/select value only mutates persisted preference state.
- Codex CLI runtime will be verified against its declared invocation contract and stdin behavior; failures should surface in session chat instead of appearing as silence.
- Agent icon display should use the existing agent registry metadata so session details remain generic across Claude, OpenCode, Codex, and Gemini.

## Risks

- Some stored sessions may already contain `\\?\` prefixes; UI normalization must be non-destructive.
- Codex invocation differences across installed versions may require defensive parsing of both JSONL and plain error output.
- Removing direct mode choices must not prevent power users from using agents that only support one implementation mode.

## Verification

- Unit tests for path normalization, default session name generation, opener preference switching, and Codex runtime invocation mapping.
- Frontend tests/build for create-session and session detail rendering.
- Rust tests/check for runtime routing and persistence behavior.
