# ACP runtime for managed CLI conversations

Six of the twelve catalog CLIs run their managed conversation over the **Agent Client Protocol (ACP)** instead of a per-turn headless command: Qwen Code, Kimi Code CLI, Qoder CLI, CodeBuddy Code, GitHub Copilot CLI, and Cursor Agent CLI. The seventh addition, iFlow CLI, is a legacy entry with a native terminal only. This chapter explains the transport, how it plugs into the existing provider runtime, and where each safety boundary is enforced. The normative requirements live in the OpenSpec change `extend-cli-providers-with-acp`.

## Three transports, one gateway

`ProviderTransport` has three values: `Terminal` (a PTY the user types into), `Headless` (one process per turn whose stdout is parsed), and `AcpStdio` (a long-lived agent process speaking JSON-RPC). A provider declares which of them it supports in its definition, and the same fact is mirrored in the tooling catalog as `CliManagedTransport` so the CLI Management page can say it without reaching into the runtime. A consistency test asserts the two catalogs agree.

`CompositeProcessGateway` routes a generation request by the resolved provider's transport: the original five keep the headless adapter, the six ACP providers reach `AcpAgentProcessAdapter`, and a provider that supports neither is refused before any process starts. Nothing in the application layer branches on a provider id.

## Wire layer

The ACP module is in-house Rust (`contexts/agent_runtime/infrastructure/providers/acp/`); the official `agent-client-protocol` crate was evaluated and not adopted because the runtime's ports are thread-based `Send + Sync` traits and the budgets below need to be enforced structurally. Layers, inside to out:

| Module | Responsibility |
| --- | --- |
| `framing` | UTF-8 newline-delimited JSON. Each frame is bounded to 1 MiB and 64 levels of nesting; a partial trailing line is kept across reads so arbitrary chunking is safe |
| `jsonrpc` | Classifies a document as request, notification, response, or error; anything else (a banner, a bare string) is a protocol failure, never ignored |
| `connection` | Owns one child process, an independent reader thread, and a single writer. Requests are correlated by id; a response for an unknown or already-timed-out id is dropped. The inbound queue holds 1,024 frames, at most 128 requests may be outstanding, and stderr is captured to a bounded, redacted 64 KiB tail |
| `session` | `initialize` (protocol version 1, client capabilities), `session/new`, `session/load` when the peer advertises `loadSession`, and the prompt-turn driver |
| `handlers` | Decides agent-originated requests: `session/request_permission`, `fs/*`, `terminal/*`, and Cursor's `cursor/ask_question` / `cursor/create_plan` |
| `interactions` | The pending-interaction store: everything waiting on a person |
| `binding` | The persisted execution binding (migration `cli-execution-bindings`) |
| `adapter` | The `AgentProcessGateway` implementation and per-binding connection pool |

Application logging goes through the unified logging port and never to the child's stdout, which is the protocol stream.

## A prompt turn

A turn is one `session/prompt` request. While its response is outstanding the driver keeps draining the inbound queue, so a permission request the agent sends mid-turn is decided or deferred to a person and answered without either side waiting on the other. That is the recursion the protocol requires and the deadlock the tests guard against.

The turn ends on the prompt response's `stopReason`, never on process exit: `end_turn` completes normally, `cancelled` after our own `session/cancel` is a cancellation, and `max_tokens`, `max_turn_requests`, or `refusal` complete with a visible card that says the agent stopped early and that this is not a verified task success. A healthy connection is returned to its binding for the next turn; one binding has at most one active turn.

Usage and cost are **unavailable** on this transport by declaration. The runtime reports `None`, and the UI shows "unavailable", never zero.

## Cancellation

Cancel sends `session/cancel` and waits up to two seconds for the prompt response with `stopReason: cancelled`. An agent that ignores it has its **own** process tree terminated; nothing outside the tree is touched, and proxy terminals it created are reaped with it. Application shutdown closes every pooled connection under a five-second deadline.

## Permissions and proxies

An agent's `session/request_permission` is evaluated against the session's effective permission policy first. `Deny` answers with the rejecting option and the tool never runs; `Allow` answers with the narrowest allowing option; `Ask` becomes a pending interaction shown as an approval card. Every pending interaction is scoped to the session, turn, connection epoch, JSON-RPC id, tool call, and policy revision, and is consumed exactly once. Dismissing the card, a timeout (30 minutes), a disconnect, or a cancel **rejects**; nothing approves silently. Choosing "allow once" answers with the once option the agent offered and never widens to "always".

`fs/read_text_file` and `fs/write_text_file` are advertised only because they are fully implemented: paths are canonicalized and must resolve inside the session's authorized roots after following symlinks, reads are capped at 8 MiB, and every write is a permission-checked tool call. `terminal/create|output|wait_for_exit|kill|release` run through the same governed process executor, keep at most eight terminals per session, bound captured output to 256 KiB, and refuse a terminal id that belongs to another session.

Policy flags are transport-aware. On a native terminal each template is projected through the flag the installed program's own `--help` documents (Qwen `--approval-mode`, Kimi `--plan`/`--yolo`/`--auto`, Qoder and CodeBuddy `--permission-mode`, Copilot `--mode plan`/`--allow-all-tools`, Cursor `--mode plan`/`--force`, iFlow `--plan`/`--default`/`--autoEdit`/`--yolo`); a template with no flag on that CLI (Qoder's read-only) is refused before spawn with a reason code rather than launched under a looser mode. Under ACP only the read-only posture is passed as a flag: the agent asks the host per call through `session/request_permission`, and a permissive launch flag would let it skip that question.

## Vendor extensions

Cursor's `cursor/ask_question` and `cursor/create_plan` are blocking requests: they surface as question and plan cards and are answered, refused, or cancelled with a schema-valid reply. Its `cursor/update_todos`, `cursor/task`, and `cursor/generate_image` are notifications, projected into the transcript and never answered. Any other unknown request receives a JSON-RPC method-not-found error; an unknown notification is ignored. CodeBuddy's account environment (international, China, iOA) is a process-level environment variable selected by profile, not a credential. Copilot's `--acp --stdio` server fixes tool and reasoning options at process start, so each binding gets its own process.

## Session binding and resume

The first turn persists an execution binding: provider, distribution, installation identity, transport, account profile, workspace, and the exact `sessionId` the agent returned. Resume runs only when the peer advertised `loadSession` and a binding exists for this session; the replay the agent sends during `session/load` is counted and discarded, so nothing is stored, billed, or approved twice. A missing binding, a different installation identity, or a peer without load support starts a fresh external session explicitly. An agent that answers `session/new` or `session/load` with JSON-RPC `-32000` (ACP's `auth_required`) is reported as *not signed in* with the instruction to sign in through the CLI in a terminal; the runtime never starts a sign-in itself. A connection that drops mid-turn ends the turn as **interrupted with unknown effects**; the prompt is never resent automatically.

Sessions created before this change carry no binding and keep their previous transport path. Deleting a session releases its pooled connections and its bindings and touches no CLI global data.

## What the settings page may do

Detection on Settings → CLI Management runs only the bounded `--version` probe. It never starts an ACP session, opens a login, or calls a model. The one program start the page may trigger is the explicit **Check connection** action (`check_cli_connection` command, `ManagedConnectionControlPort::check_connection`): it launches the resolved executable with the provider's ACP flag, sends `initialize` only, and terminates the process, returning a redacted negotiation summary. A **Sign-in guide** button opens the catalog's reviewed `login_docs_url` through the HTTPS-only external link service on a click. Authentication state for the six ACP CLIs is reported as `unknown` because none of them documents a stable read-only authentication probe; "unknown" is not "signed in".

## Where the design lives

- OpenSpec change: `openspec/changes/extend-cli-providers-with-acp/` (proposal, design, provider matrix, verification results)
- Runtime: `src-tauri/src/contexts/agent_runtime/infrastructure/providers/acp/`
- Provider definitions: `src-tauri/src/contexts/agent_runtime/infrastructure/providers/definitions.rs`
- Catalog: `src-tauri/src/contexts/tooling/cli/domain/registry.rs`
- Related chapters: [Runtime boundaries](runtime-boundaries.md), [CLI lifecycle](cli-lifecycle.md), [Permission model](permission-model.md)
