# Reporting issues

## Check whether you should file one first

Run through these three steps before filing an issue — it saves most of the back-and-forth:

1. **Check [Troubleshooting](troubleshooting.md)** — high-frequency issues like "CLI shows not installed" or "a session is stuck at action required" already have answers there.
2. **Check the [FAQ](faq.md)** — if your question is really "is this feature supposed to work this way," the answer is most likely here.
3. **Search existing issues** — the [repository's issue list](https://github.com/cdavid817/vanehub-ai/issues). Searching is a required checkbox on the bug form; don't skip it.

VaneHub AI is a **community-supported** project with no commercial ticketing channel.

## Pick the right entry point

The repository has **blank issues disabled**; you must go in through one of these four entry points:

| What you're reporting | Where to go |
| --- | --- |
| **A reproducible defect** | [Bug report form](https://github.com/cdavid817/vanehub-ai/issues/new?template=bug.yml) |
| **A feature proposal** | [Feature request form](https://github.com/cdavid817/vanehub-ai/issues/new?template=feature.yml) |
| **A security vulnerability** | [Private security advisory](https://github.com/cdavid817/vanehub-ai/security/advisories/new) |
| **Not sure if it counts** | Read [SUPPORT.md](https://github.com/cdavid817/vanehub-ai/blob/main/SUPPORT.md) first |

> **Never open a public issue for a security vulnerability.** Use the private security advisory channel; the process is in the repository's `SECURITY.md`. A public issue is not an appropriate security-reporting channel — that's exactly why "This is not a security vulnerability" is a required checkbox on the bug form.

## Reporting a defect: what the form needs

The bug form has four required fields; missing any one of them makes the issue hard to work with.

### 1. Version or commit

Fill in the application version (like `v0.1.0`) or a commit SHA.

**Where to find it**: **Settings → About**, which has the version number, update check, and a changelog link.

### 2. Operating system

Choose one of three: **Windows**, **macOS**, **Linux**.

**Desktop behavior does not generalize across platforms.** The same symptom can have entirely different causes on the three systems (`PATH` resolution, credential storage, file locking), so this field is required. If an issue only shows up on one system, note which other systems you verified it on.

### 3. What happened

**Write both the actual and the expected behavior.** "It doesn't work" alone can't be diagnosed; "I expected X, but got Y" can.

### 4. Reproduction steps

Write them numbered, starting from which screen you opened:

```text
1. Open Settings → CLI management
2. Click "Upgrade" on Claude Code
3. Observe that the version number didn't change
```

**Shorter is better.** If a fresh empty project reproduces it, use that instead of your real repository.

### 5. Redacted diagnostics (optional, but strongly recommended)

Logs, screenshots, and the version numbers of the relevant Agent CLIs.

**Where to find them**:

- **Application logs** — by default under `logs/` in the data directory, main file `vanehub.log`. Per-platform paths are covered in [Troubleshooting](troubleshooting.md).
- **In-session logs and traces** — the workspace's **Logs** and **Traces** tabs; see [Observability](observability.md).
- **CLI versions** — `claude --version` / `codex --version` / `gemini --version` / `opencode --version` / `agy --version`.

## Redact before you submit — always

**This is a required checkbox, and it's also the step people most often get wrong.**

The logs VaneHub AI writes to disk are already sanitized — provider keys, Bearer tokens, private paths, and command arguments are replaced with redaction markers. **Screenshots are not**, and neither are snippets you paste in by hand.

Check each of these before submitting:

- **Credentials** — API keys, tokens, cookies, account passwords
- **Personal data** — emails, real names, organization names
- **Local paths** — `C:\Users\YourName\...`, `/home/your-username/...`
- **Conversation content** — what you and the Agent said to each other, source code the Agent read

> **Once submitted, it's public.** A GitHub issue is visible to everyone; editing or deleting it afterward cannot un-send notification emails, or pull it back from crawlers and search indexes that already picked it up. When in doubt, delete it before submitting, not after.

If sanitization has redacted enough that the logs no longer explain the problem, you can temporarily switch the capture policy to "redacted content" in **Settings → Execution observability** and reproduce again; see [Observability](observability.md).

## Proposing a feature

The feature form doesn't ask for a version or reproduction steps — it asks about four different things:

| Field | What matters |
| --- | --- |
| **Problem or opportunity** | What **user need** this solves, not "what feature I want" |
| **Proposed outcome** | Describe the desired **behavior**; **don't assume a specific implementation** |
| **Alternatives considered** | Optional, but filling it in noticeably improves the odds of it being adopted |
| **Additional context** | Optional |

Two required checkboxes worth knowing about upfront:

- **You searched existing issues and OpenSpec changes** — not just issues, but changes already in progress under the repository's spec workflow.
- **You understand that implementation requires an accepted OpenSpec proposal** — any new feature or architecture change in this project has to go through the proposal process first; an accepted issue doesn't mean work starts immediately.

## What gets handled fast

- **One issue reports one thing.** Bundling three problems together just gets all three stuck.
- **Write the title as a symptom, not a guess.** "CLI version doesn't refresh after upgrading" beats "CLI management module has a bug."
- **Separate "must fix" from "could be improved,"** the same way a review does.
- **State whether it reproduces reliably.** Intermittent problems are worth reporting too, but say so, and roughly how often it shows up.
- **Note what you've already ruled out.** "Restarted, refreshed detection, switched project directories, still happens" saves a whole round of back-and-forth questions.

## Related

- Something's broken, check here first → [Troubleshooting](troubleshooting.md)
- Is this feature supposed to work this way → [FAQ](faq.md)
- Logs, traces, and capture policy → [Observability](observability.md)
- Version numbers and update checks → [Application updates](app-updates.md)
