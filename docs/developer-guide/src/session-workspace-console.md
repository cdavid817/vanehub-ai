# The session workspace evidence console

The nine tabs beside a session — Workspace, Changes, Documents, Files, Terminal History, Shell, Logs, Traces, Report — are one capability rather than nine, and the thing they have in common is what they refuse to do. Every one of them answers a question about work that already happened, and every one of them can be asked a question it cannot fully answer. The design rule throughout is that **a short answer must never look like a complete one**.

This chapter covers the five things a developer working on these tabs needs and cannot infer from the code alone: what fidelity and coverage mean, how a Shell survives being navigated away from, what a remote workspace needs before it can answer anything, how logs are indexed, and how one panel hands a reader to another.

## Fidelity and coverage are two different admissions

They are easy to confuse and they answer different questions.

**Fidelity** is a property of one record: *how did we come to know this?* It is set by whoever produced the record and travels with it. See [execution observability](execution-observability.md) for the four tiers; what matters here is that the console never derives it, never upgrades it, and never displays a record without it.

**Coverage** is a property of an answer: *how much of what you asked about did this look at?* It travels on the page, not fetched separately, so a reader cannot end up looking at rows from one moment and a coverage claim from another.

| Coverage state | What it licenses a reader to conclude |
| --- | --- |
| `complete` | Everything in scope was examined. An empty result means there was nothing. |
| `indexing` | The rows shown are real; the set is not yet final. |
| `partial` | Something in scope was not examined. An empty result means nothing at all. |
| `unavailable` | The question could not be answered. |

Coverage only ever degrades, and each degradation adds a stable reason code — never prose, because a reader groups by codes and free text does not group. A page that dropped events, hit a source conflict, had rows trimmed by retention, or is reading a projection that is behind the journal will say so with a code for each.

**The distinction that matters most is between "this is everything" and "this is as much as was looked at."** They are separate facts with separate remedies, and a surface that renders them identically has thrown one of them away. This is why an empty list beside `partial` coverage never says "no results" — that is a conclusion the query is not in a position to offer.

### Absence is not zero

A figure that could not be derived is absent, not defaulted. `reportedTokens` missing means nobody reported tokens; `reportedTokens: 0` means somebody reported zero. Rust models use `Option` with `skip_serializing_if`, and the frontend renders a named absence — "not observed", "unavailable", "redacted" — rather than a dash, because those are three different things a reader would otherwise have to guess between and one of them means the work may not have happened the way the row implies.

### Historical activity is projected, never recorded

Sessions that predate the evidence journal still have `message.toolUse` blocks in their history, and the console shows them — as their own list, always `inferred`, always partial coverage, labeled as message-history.

They are **not** written into the journal. A `toolUse` block is what an assistant said it was doing; the journal holds what the runtime watched happen. Once the two are filed together nothing downstream can separate them again. Guards enforce this: no file may both append to the journal and read the chat corpus, and a migration test requires the journal to come out empty on a database that already holds such a message. Empty is the correct answer there, and it is the one that looks like a bug.

## Shell attach and detach

A Session Shell is a long-lived process. The tab is a *view* of it, and the two have independent lifetimes.

`SessionShellRegistry` is the only thing that opens or closes a shell — checked by construction, so there is no second place a shell can be created that the registry does not know about. Navigating away detaches the view; the process keeps running, keeps producing output, and is still there when the reader comes back. **A build survives a tab switch, a session switch, and a remount, and stops only when someone explicitly says so.**

What a frontend author needs to know:

- **Do not kill on unmount.** The retired one-view service did, and a background build died because somebody clicked another tab. Guards fail any production file that reconstructs it.
- **Reattaching replays.** The registry retains recent output, so a returning view is not blank and is not a second process.
- **Capabilities come from the runtime descriptor**, an internally tagged union the frontend narrows on `kind`. Each variant carries the capabilities that variant actually has — a simulated shell has no resize, a PTY has no reconnect. The string union this replaced let the UI ask for both.

## Remote workspaces need a helper, and say so before you ask

A remote workspace answers the same questions as a local one through a helper on the far side. It is not always there, and its absence is a normal state rather than an error.

The provider reports what it can do **capability by capability** rather than as one flag: a remote host with Git but no ripgrep is ordinary, and a single flag would either hide the search gap or disable the four things that work. Each capability carries its own state and, when unavailable, a stable reason code with a `workspace.capability.reason.*` translation — never a message.

Unsupported actions render **disabled with a reason** rather than hidden. A control that vanishes on a remote workspace makes a reader think they misremembered where it was; one that is visibly unavailable tells them the truth, which is that this workspace is on another machine. See [SSH connections](ssh-connections.md) for how the connection itself is established and verified.

## Log indexing

The Logs tab reads an index, and only an index. There is no fallback to scanning files: a fallback is a second query implementation with different filters, different bounds and different coverage semantics, reached exactly when a reader is least able to tell which one answered. When the index cannot answer, it says so.

The index is a **rebuildable projection** over the redacted JSONL files described in [unified logging](unified-logging.md). Nothing in it is a second source of truth — every row is derivable again from the files, which is what lets its schema be shaped for reading.

A repair job brings it up to date:

- **Started after the window exists, spawned rather than run**, and handed to the blocking pool rather than executed on an async worker. Synchronous SQLite on a runtime worker parks it for the length of the scan and queues every unrelated command behind it, with no symptom beyond "a few commands were slow once after launch."
- **Progress is a persisted checkpoint**, keyed by directory generation rather than by path. A rotated file reuses the path, so resuming at the old offset would silently skip the beginning of the new file.
- **Every batch is bounded**, and file reading happens with no transaction open. A transaction held across file IO holds the write lock for as long as the disk takes.
- **While it runs, queries answer with `indexing` coverage** and report the point the index has reached, so a stale page can explain itself.

Paging is keyset, and stays keyset. This result set grows underneath the reader; an offset renumbers with every insert, so offset paging skips exactly the rows that arrived while the reader was reading, and skips them silently.

Retention deletes index rows whose source file is no longer retained — and records a `log_source_expired` gap in the same transaction. It is the one deletion here that cannot be undone, because the file it would be re-read from is gone, so a query after a rotation reports partial coverage rather than a short answer that calls itself complete.

## Cross-panel navigation

The tabs are separate views of one session, and a reader following a thread should not have to reconstruct where they were. Navigation between them is a **scope**, not a route: a target tab plus the correlation that got the reader there — a run, an operation, a command, a file path, a span.

The rules a new link has to follow:

- **Carry the correlation, not the result.** Linking to "the logs for this command" passes the command id and lets the Logs tab ask its own question with its own bounds and its own coverage. Passing rows across would present one panel's bounds under another panel's name.
- **A link is offered only when the correlation is trusted.** Evidence exposes links to file mutations, review decisions, verification outcomes, and usage observations *when their canonical owners provide correlation* — never when it merely looks plausible.
- **The destination states its own coverage.** Arriving somewhere by link does not narrow what that panel admits about its answer.
- **Navigation is a service-boundary call**, like everything else in the frontend. Components never reach Tauri directly; see [runtime and service boundaries](runtime-boundaries.md).
