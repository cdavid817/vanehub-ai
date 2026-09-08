# ACP CLI Agents and the legacy iFlow entry

Besides the original five CLIs, VaneHub AI drives six more coding CLIs through the **Agent Client Protocol (ACP)**: Qwen Code, Kimi Code CLI, Qoder CLI, CodeBuddy Code, GitHub Copilot CLI, and Cursor Agent CLI. A seventh entry, iFlow CLI, is kept only as a legacy native-terminal option. This chapter covers what differs from the original five; everything else (creating a session, the workspace tabs, permission templates) works the same way.

## What ACP changes for you

With the original five, VaneHub starts a fresh process for each turn and reads its output. With an ACP CLI, **the program stays running for the whole session**, and VaneHub talks to it over a structured protocol. In practice:

- **Permission requests come from the CLI itself.** When the agent wants to run a command or write a file, an approval card appears in the conversation, same as with Claude Code. Choosing *Allow once* grants exactly that one request; nothing you click is silently widened to "always".
- **Questions and plans are cards, too.** Cursor Agent CLI can ask you a question or propose a plan before continuing; answer or refuse it in the conversation. Leaving the card unanswered for 30 minutes, closing it, or cancelling the turn counts as a refusal.
- **Cancel is cooperative first.** Cancelling asks the agent to stop; if it does not answer within a couple of seconds, VaneHub terminates only the process it started for this session.
- **Token usage shows as "unavailable".** These CLIs do not report usage through ACP, so VaneHub shows unavailable rather than a misleading zero.
- **If the connection drops mid-turn**, the message ends as *interrupted, effects unknown*. VaneHub never resends your prompt on its own; review what the agent changed and send it again yourself if needed.

## Install and sign in

Installation and sign-in follow the same rules as the original five: VaneHub installs only from the audited source listed below, **sign-in always happens in your terminal**, and detection on Settings → CLI Management never starts an ACP session or a login. Because none of these CLIs documents a reliable read-only sign-in check, their authentication status is shown as **Unknown**; unknown does not mean signed in. Run the CLI once in a terminal and confirm it accepts a prompt before creating a session.

| Agent | Command | In-app source | Notes |
| --- | --- | --- | --- |
| Qwen Code | `qwen` | npm `@qwen-code/qwen-code` | The vendor's standalone installer may fall back to npm internally; VaneHub uses the npm package directly |
| Kimi Code CLI | `kimi` | npm `@moonshot-ai/kimi-code` | An older Python/uv installation is detected and reported as a separate installation; VaneHub never migrates it. Sessions stay bound to the distribution they started on |
| Qoder CLI | `qoder` | npm `@qoder-ai/qodercli` | Windows on arm64 is unsupported upstream and shown as such; the old `qodercli` name is accepted as an alias |
| CodeBuddy Code | `codebuddy` | npm `@tencent-ai/codebuddy-code` | Choose the **account environment** (International, China, or iOA) for the session; it is independent of the interface language |
| GitHub Copilot CLI | `copilot` | npm `@github/copilot`; WinGet on Windows | The standalone Copilot CLI, not the older `gh copilot` extension. Tool and reasoning options are fixed when the process starts, so each session gets its own process |
| Cursor Agent CLI | `agent` | Vendor installer, latest only | `agent` is a common program name; VaneHub accepts a copy only when its install path or version banner names Cursor. Anything else is reported as a mismatch and never launched |

Every ACP CLI also has a **native terminal** option, which opens the CLI's own interactive interface in the session terminal.

Two explicit actions live on each card in Settings → CLI Management:

- **Check connection** (installed ACP CLIs only) starts the program for a single protocol handshake and then releases it. It shows the protocol version, the program's reported name and version, whether it offers session resume, and which sign-in methods it advertises. It creates no session, sends no prompt, and does not prove you are signed in.
- **Sign-in guide** opens the vendor's own documentation in your browser. Only HTTPS documentation links are ever opened, and only on your click; sign-in itself still happens in your terminal.

## Third-party endpoints

Qwen Code and iFlow can run against any OpenAI-compatible endpoint. Rather than editing `~/.qwen/.env` or `~/.iflow/settings.json` by hand, create a profile for either Agent under **Settings → Agent configurations** and apply it; see [Agent configuration](agent-configuration.md). The other five CLIs offer no such setting in their programs, so VaneHub does not pretend to configure one.

## Resuming a session

An ACP session is resumed only when the CLI advertises session loading and VaneHub still holds the exact session identity it recorded, on the same installation. Otherwise a new external session starts and the local history is kept. VaneHub never guesses "the last session" from the CLI's own history.

## Unattended runs

Scheduled tasks and multi-Agent seats can use the ACP CLIs, but an unattended run has nobody to answer an approval card. Such runs follow the task's unattended policy: a permission request is rejected and the run is flagged for intervention rather than approved automatically.

## iFlow CLI (legacy)

iFlow's official service and API shut down on 2026-04-17. VaneHub keeps a **legacy** entry for people who still run the installed CLI against their own custom API configuration:

- It is hidden under the *Legacy* group in the session dialog until you opt in, and it is marked *legacy* wherever it appears.
- Only the **native terminal** is available: no managed conversation, no multi-Agent seat, no scheduled task.
- VaneHub does not install it, does not offer a login, does not import or migrate its configuration, and does not claim any official service.

## Troubleshooting

- **The card says the agent stopped before the end of the turn** (`max_tokens`, `refusal`, `max_turn_requests`): the CLI stopped early on its own. This is not a verified success; check the result before continuing.
- **"… is not signed in"** when starting a conversation: the CLI answered that no account is signed in. Sign in with the CLI in a terminal and try again; VaneHub never signs in for you.
- **"Policy cannot be enforced"** when opening a native terminal: the selected permission template has no launch flag on that CLI (today only Qoder CLI with the Read-only template, which has no plan mode). Pick a template the CLI can honor; VaneHub never falls back to a looser mode by itself.
- **Cursor shows "identity mismatch"**: the `agent` found on `PATH` is not Cursor's. Install Cursor's CLI or fix `PATH` order; see [Installation conflicts](getting-started.md#installation-conflicts).
- **CLI Parameters page**: the ACP CLIs and iFlow have their own entries (model, reasoning or thinking mode, agent profile, and a few startup switches, each read from the installed program's `--help`). Permission flags are not editable there: they follow your policy template, and under ACP only the read-only posture is passed as a flag.
