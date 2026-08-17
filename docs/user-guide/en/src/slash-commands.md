# Slash commands: drive the interface from the input box

**Status: Implemented — the interface is identical on desktop and in Web/mock; commands are only active in built-in native Agent sessions.**

## Overview

Typing `/` in a session input box lets you switch tabs, change the execution mode, or check usage without moving your hands to the sidebar.

**These commands are executed by the frontend and are not messages sent to the model.** They act on existing session and interface capabilities — `/mode execute` flips the session's execution-mode switch; it does not ask the model to "enter execute mode".

## Available commands

Typing `/` brings up completion candidates, and `/help` lists every command available in the current session.

**Open a surface:**

| Command | What it does |
| --- | --- |
| `/todo` | Open the todo board |
| `/plans` | Open the plan center |
| `/plan` | Open this session's associated plan run |
| `/loops` | Open the loop center |
| `/files` | Open the files tab |
| `/changes` | Open the changes tab |
| `/documents` | Open the documents tab |
| `/report` | Open the report tab |
| `/logs` | Open the logs tab |
| `/traces` | Open the traces tab |
| `/shell` | Open the shell tab |
| `/terminal` | Open the terminal tab |

**Toggle session switches:**

| Command | What it does |
| --- | --- |
| `/mode <value>` | Set the execution mode |
| `/thinking <value>` | Toggle thinking |
| `/streaming <value>` | Toggle streaming |
| `/longcontext <value>` | Toggle long context |

**Inspect and export:**

| Command | What it does |
| --- | --- |
| `/help` | List available commands |
| `/status` | Show the current runtime switches |
| `/usage` | Show token usage for this session |
| `/export` | Export this session |

`/usage` reports total tokens, input tokens, output tokens, and the number of responses.

## What counts as a command

The test is strict; **every condition must hold**:

- After trimming, the input is a **single line**
- It begins with **exactly one** `/`
- The `/` is followed by a name that **starts with a letter** and contains only letters, digits, and hyphens
- The name is followed by whitespace or the end of the input

Command names are **case-insensitive**, and whatever follows the name is split on runs of whitespace into arguments.

The two counter-examples you are most likely to hit:

- **`/usr/bin/env` is not a command.** It is sent as an ordinary message. A `/` immediately after the name fails the "whitespace or end" condition.
- **Multi-line text is not a command**, even when the first line is exactly `/help`.

## Actually sending a sentence that starts with a slash

Start it with `//`, and the system **strips one `/` and sends it as an ordinary message**. Submitting `//help` sends the text `/help` and executes nothing.

**This escape is necessary because unknown commands are not forwarded to the model.** Without it, any prose genuinely beginning with `/` could not be sent at all.

## Which sessions have commands

**Only sessions belonging to the built-in native Agent enable slash commands**, decided by stable agent id rather than display name — renaming something does not change the behavior.

In a session where they are not enabled, command-shaped input is **sent as an ordinary message**. In other words, typing `/mode execute` in a CLI Agent session means the model receives that line of text and the execution mode does not change.

Beyond that, **each command has its own applicability condition**. `/help` lists the commands actually available in the current session; inapplicable ones do not appear.

## Commands never reach the model

Input taken over by the command layer is **not sent to the model, and the input box is cleared**. That decision is made synchronously at submission, so there is no intermediate state where a message appears sent and is then retracted.

The command's own execution may be asynchronous; its result is presented when it completes.

## When something goes wrong

| Case | Feedback | Side effects |
| --- | --- | --- |
| Unknown command | "Unknown command /xxx. Try /help." | **No message is sent** |
| Invalid argument | Lists that command's allowed values | **None** |
| Execution failed | "Command failed" | Reported through the service boundary |

**An unknown command is never silently forwarded to the model.** Silent forwarding would leave you believing the message arrived, when what the model actually received is a line of command text you never meant to send.

An invalid argument likewise **produces no side effect** — `/mode nonsense` reports an error listing the allowed values, and the execution mode is untouched.

## Where command output goes

**Command output is presented outside the chat message list**, and can be dismissed from its top right.

It is **not persisted as a message and does not appear in a session export**. The reason is practical: the message list is refetched from the backend after every send, and a locally injected entry does not survive one refresh. Kept outside the list, the output stays visible across an unrelated refetch.

## Completion

When the input box holds `/` or `/` plus part of a name, candidates are filtered by name prefix.

**Typing alone never executes a command.** Keying `/mode execute` character by character without submitting changes no execution mode.

## Notes and limits

- **Only built-in native Agent sessions are affected**; elsewhere, command-shaped input is treated as an ordinary message.
- **Command names stay in English.** Switching the interface language changes the descriptions, output, and error messages, but not the command names themselves — which is what keeps them typeable in every language.
- **Command output is not included in an export**; use `/export` to export the session itself when you need a record.
- **A path after `/` is not treated as a command**, but if the sentence you want to send happens to start with `/` followed by a letter, remember `//`.

## Related

- What execution mode, thinking, and streaming mean → [Native API Agent](native-agent.md)
- The board `/todo` opens → [Todo Board](todo-board.md)
- The loop center `/loops` opens → [Loop Engineering](loop-engineering.md)
- How the usage behind `/usage` is counted → [Scheduled and usage](automation.md)
