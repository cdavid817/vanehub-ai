# Design

## D1. The Artifact read-back port is mostly unnecessary

This change was written assuming both producers store their image in the Artifact store and would
need to read it back by id, and that the agent runtime having no such port was the blocker. An
implementation survey shows that is wrong for the paths that matter.

**Screenshot already holds the bytes.** `browser_automation`'s native tool adapter receives
`bytes_base64` in the operation payload and seals it into an Artifact itself
(`native_tool_adapter.rs`, the `BrowserAction::Screenshot` branch). The image is in hand at
exactly the moment the result is built — nothing needs reading back.

**OCR's rendered pages are files it owns.** The OCR adapter renders into
`workspace.root/outputs/rendered` inside its own sandbox and seals text and JSON results. A
rendered page is a local file it can read directly. Only OCR's *source* image is referenced purely
by Artifact id (`result.source_artifact_id`), and returning the source is not what this change is
for — returning what OCR *rendered* is.

So the port shrinks to at most a narrow read-by-id used by one optional case, and should not be
built until something actually needs it. If it is built, note that `ArtifactService::read_text`
already shows the shape: `metadata(id)` then `blobs.read_verified(&content_hash)`, which verifies
integrity and never exposes a host path.

**A correction to this change's own spec:** the `Read-only Artifact bytes access` requirement says
access is restricted to "Artifacts owned by the calling session". The Artifact catalog has no
session field — `ArtifactDescriptor` carries `source_operation_id`, `creator`, and `visibility`,
not a session — so that is not enforceable as written. The real guarantee on these paths is
stronger and simpler: the id is never model-supplied. A tool attaches an image it just produced,
so there is no caller-controlled id to point somewhere else. That requirement should be rewritten
to state that before any port is implemented.

## D2. The actual blocker is the result envelope, and it is shared

`NativeToolResultEnvelope` has `output: Option<Value>` and `metadata`, and no image channel. Both
extended tools return through it, and `execute_registered_native_tool` converts it into a
`ToolExecutionOutcome` — text — which the tool loop pushes as
`(tool_use, output, is_error, None)`.

The tempting shortcut is to put base64 in `output`. That is the one thing this must not do: tool
output is persisted on the assistant message and rendered in the transcript, so a screenshot would
put megabytes of base64 into the conversation the parent has to carry — the exact cost
`add-agent-image-input` bounded everywhere else, reintroduced through the back door.

So the work is: give the envelope an image slot alongside `output`, surface it through
`execute_registered_native_tool` into the executed-call tuple that `build_reply_turns` already
accepts, and populate it in the two adapters. The tuple already carries `Option<AgentImage>` from
`add-agent-image-input`, so the loop end needs nothing new — only the extended-tool path does.

Both producers then route through `prepare_image`, inheriting the reviewed types, dimension and
byte bounds, downscaling, and the per-request budget rather than getting a second path.

## D3. Sequencing

D2 touches a type every native tool result flows through, so it wants to land as its own step with
the existing extended-tool tests green, before either adapter is changed. The adapters are then
independent of each other and can land separately: screenshot needs no new reads at all, and OCR
needs only to read a file inside its own sandbox.
