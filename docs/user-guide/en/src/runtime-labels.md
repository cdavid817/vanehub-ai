# Runtime and feature labels

Every chapter in this guide opens with a status line. This page explains how to read them.

## Two kinds of label

**They answer two different questions**, which are easy to conflate:

| Question | Which label |
| --- | --- |
| Is this feature finished? | Feature state: Implemented / Preview / Planned |
| Does it actually work in the environment I am in right now? | Runtime: desktop only / Web/mock only |

A feature can be "Implemented" and "desktop only" — finished, just not effective in a browser.

## Feature state

| Label | Meaning |
| --- | --- |
| **Implemented** | A user-visible path is implemented, with verification evidence |
| **Preview** | The supporting contract exists, but the normal workflow is not complete |
| **Planned** | No supported workflow yet |

**"Preview" does not mean "nearly ready".** What it accurately means is that the underlying service or mock contract exists, but you cannot complete the flow in the interface. Whether a feature is usable is determined by whether there is a **user-visible path**, not by whether an implementation exists in the code.

## Runtime

| Label | Meaning |
| --- | --- |
| **Desktop only** | The Tauri runtime, using the local filesystem, CLIs, SQLite, or OS integration |
| **Web/mock only** | A deterministic browser simulation with no native side effects |

## Which runtime you are in

**If you installed the desktop application and opened it, you are on desktop**; if you reached it in a browser, you are in the Web preview.

The interface is exactly the same in both — deliberately, since one React codebase serves both runtimes. **So you cannot tell from appearance.**

## Capabilities that do not work in the browser preview

**All of these require desktop:**

| Capability | Desktop | Browser preview |
| --- | --- | --- |
| Starting a CLI process | Yes | No |
| Terminal (PTY) | Yes | No |
| Data persistence (SQLite) | Yes | No |
| Reading and writing files | Yes | No |
| System credential storage | Yes | No |
| SSH / IM network connections | Yes | No |
| Tray / notifications / launch at login | Yes | No |
| Permission interception | Yes | No |
| Execution trace collection | Yes | No |
| Interface and interaction logic | Yes | Yes |

**The last row is the point**: the interface and its interactions are real in both runtimes; nothing else is.

## "It succeeded" is not "it happened"

**A success message in the browser preview only proves the interface logic is correct.** It runs on deterministic mock data — the same action produces the same result every time, because the result is hardcoded rather than computed.

Concretely, in the browser preview:

- Creating a session succeeds, but **no process is started**
- The terminal produces output, but **it is simulated, not the real output of a command**
- Saving settings reports success, but **it is gone when you close the page**, because there is no database
- A CLI shows as "Installed", but **your machine was never inspected**

None of this is a bug. The browser preview exists to verify the interface, take documentation screenshots, and give a quick look at the product — not to replace the desktop application.

## The screenshots in this guide

**Every screenshot is taken in the browser preview**, in a fixed CI environment, so they are reproducible.

Which means: **a screenshot can show you what a surface looks like and where a control is, but it is not evidence that a process was started or a file was written.** For anything involving native side effects, the prose is authoritative, not the screenshot.

## Related

- The desktop app will not install, or a CLI is not detected → [Install and authenticate a CLI](getting-started.md)
- Something reports success in the browser but has no effect → [Troubleshooting](troubleshooting.md)
