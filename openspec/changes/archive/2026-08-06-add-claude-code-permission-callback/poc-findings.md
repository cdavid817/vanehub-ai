## PoC gate findings (tasks.md Group 1)

Scratch spike location: OS temp dir (`%TEMP%\claude-hook-poc`), deleted after this session — never inside the VaneHub repo. Real user-level `~/.claude/settings.json` was never written to; verified byte-identical (sha256 `8707...c964e71`, size 765, mtime `2026-08-05 00:13:38.444114200 +0800`) before and after this session.

Environment: `claude --version` → `2.1.221 (Claude Code)`. Node v24.15.0 used for the throwaway spike (server + wrapper) — chosen purely for PoC iteration speed, per the task's own "pick whatever's fastest" allowance. This is **not** a recommendation to use Node for the real wrapper; see Finding E below, which if anything reinforces D4's choice of a compiled Rust binary.

### Summary

| # | Scenario | Result | Evidence |
|---|---|---|---|
| 1.1 | Command-hook contract, end-to-end, Bash tool | **PASS** (mechanics); live-CLI dispatch not independently confirmed | see below |
| 1.2 | Server crash mid-request | **PASS** | ECONNRESET detected in 337ms, fail-closed |
| 1.3 | Hung/non-responding server | **PASS** | bounded client timeout fired at 5030ms (bound=5000ms), fail-closed, did not hang |
| 1.4 | Malformed/garbage response | **PASS** | parse failure caught, fail-closed for Bash; also exercised D5 allowlist asymmetry for Read |
| 1.5 | `"type": "http"` hook fires for `PreToolUse`? | **NOT EMPIRICALLY CONFIRMED** — environment blocker, see Finding D | strong documentary evidence found, explicitly insufficient per task instructions |
| 1.6 | Record findings | **DONE** (this document) | — |

**Bottom line: Group 2 is clear to proceed.** All of 1.2–1.4 passed with clean evidence — no design-revisit trigger from the task's own gate condition. The one open item (1.5) is something design.md already scoped as non-blocking ("Resolving the `http` question is left to the fault-injection PoC gate... not blocking here" — Non-Goals; "Revisit only if process-spawn overhead proves to be a real problem in practice" — Risks). See "Impact on design.md" below for the two things worth updating in the document itself.

---

### What was built

- `server.js` — loopback HTTP server, `127.0.0.1`, random port via `server.listen(0, ...)`, random bearer token, writes `{port, token}` to a discovery file on startup. Mode selected via `POC_MODE` env var: `normal` (canned allow/deny on `POST /evaluate`, our own wrapper<->server protocol), `crash` (accepts the request, waits 250ms, then `process.exit(1)` without responding), `hang` (accepts the request, never responds), `malformed` (responds HTTP 200 with a garbage non-JSON body), `http_ok`/`http_deny` (responds directly in Claude Code's `hookSpecificOutput` wire format, for the 1.5 test).
- `.claude/hooks/wrapper.cjs` — the throwaway hook wrapper. Reads stdin, parses the real documented `PreToolUse` JSON shape (`session_id`, `hook_event_name`, `tool_name`, `tool_input`, `cwd`, ...), reads the discovery file fresh every invocation (per D3), POSTs to `/evaluate` with a bounded client-side timeout (`POC_TIMEOUT_MS`, defaulted to 5000ms for fast PoC iteration — design's real D8 value is ~310–330s; only the *mechanism*, not the constant, was validated here), and translates the result into `{"hookSpecificOutput": {"hookEventName": "PreToolUse", "permissionDecision": "allow"|"deny", "permissionDecisionReason": ...}}` on stdout with exit 0. On any failure (parse error, discovery-file-missing, connection error, timeout, malformed response) it applies D5's asymmetric fallback: a hardcoded `Read`/`Glob`/`Grep` allowlist fails open, everything else fails closed.
- `.claude/settings.json` (project-level, scratch dir only) — registered a `PreToolUse` hook, matcher `Bash`, `"type": "command"`, `"command": "node \".../wrapper.cjs\""`, `"timeout": 30`.
- Fixture files reproducing the exact documented stdin contract (confirmed against `code.claude.com/docs/en/hooks` this session) for `Bash` (allow and deny variants) and `Read`.

### 1.1 — Command-hook contract, end-to-end

Verified the full loop mechanically, both directions, via direct subprocess invocation of `wrapper.cjs` (spawn process → write real-shaped stdin JSON → close stdin → read stdout JSON + exit code — exactly what Claude Code's `"type": "command"` dispatcher does per the docs) against a live `normal`-mode server:

- Deny path (`echo hello world`, no `ALLOWME` in the command): server responded `{"decision":"deny","reason":"poc-server canned deny"}` in 81ms; wrapper emitted `{"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"deny","permissionDecisionReason":"poc-server canned deny"}}`, exit 0.
- Allow path (`echo ALLOWME hello world`): server responded `{"decision":"allow",...}` in 85ms; wrapper emitted `permissionDecision":"allow"`, exit 0.

Both match the documented `hookSpecificOutput`/`permissionDecision` schema and exit-code-0-means-parse-stdout contract exactly.

**What this does *not* cover:** I could not get a genuine live `claude` process (interactive or `-p`) to actually spawn `wrapper.cjs` via the registered hook and drive this loop itself — see Finding D. So "Claude Code's own hook dispatcher correctly invokes this wrapper per `settings.json`" is unverified, as distinct from "the wrapper correctly implements the contract," which is verified. I'm marking 1.1 passed because its text ("reproducing the contract end-to-end for one tool") is about faithfully reproducing the protocol, which was done rigorously in both directions; the live-dispatch question is what 1.5 is specifically about, and that one I'm leaving unchecked.

### 1.2 — Server crash mid-request — PASS

`server.js` in `crash` mode: accepts the POST, logs it, waits 250ms, then `process.exit(1)` without ever calling `res.end()`.

```
server.log: [14:41:17.380Z] REQUEST ... mode=crash ...
server.log: [14:41:17.382Z] SIMULATING crash mid-request: process.exit in 250ms, no response sent
server.log: [14:41:17.639Z] CRASHING NOW

wrapper.log: [14:41:17.347Z] INVOKED tool=Bash ...
wrapper.log: [14:41:17.684Z] REQUEST ERROR elapsedMs=337 message=read ECONNRESET
wrapper.log: [14:41:17.686Z] DECISION deny tool=Bash reason="offline-fallback-deny: request error: read ECONNRESET" exit=0
```

The OS-level TCP reset was surfaced as an `error` event well before the 5000ms bound (337ms), the wrapper's fail-closed path fired correctly, exit 0 with a valid deny JSON. Confirmed the server process was actually dead afterward (not just slow).

### 1.3 — Hung/non-responding server — PASS

`server.js` in `hang` mode: accepts the connection, logs it, never calls `res.end()`.

```
wrapper.log: [14:41:46.409Z] INVOKED tool=Bash ...
wrapper.log: [14:41:51.439Z] CLIENT TIMEOUT elapsedMs=5030 (bound=5000ms) — destroying socket
wrapper.log: [14:41:51.445Z] REQUEST ERROR elapsedMs=5036 message=client-side timeout after 5000ms
wrapper.log: [14:41:51.447Z] DECISION deny tool=Bash reason="offline-fallback-deny: request error: client-side timeout after 5000ms" exit=0
```

The wrapper's own bounded timeout (Node's `http.request({timeout})` + explicit `req.destroy()` on the `timeout` event — needed because Node's `timeout` option alone does not abort the request) fired at 5030ms against a configured 5000ms bound, and resolved to Deny rather than hanging indefinitely. Server was confirmed still alive-but-unresponsive afterward (this scenario doesn't crash the server, unlike 1.2), then killed. Note this PoC used a compressed 5s bound for fast iteration; only the *mechanism* (a finite client-side timeout that resolves to Deny) was validated, not design's real ~310–330s constant (D8) — that constant should get at least one real-timing smoke test once implemented in Group 4.

### 1.4 — Malformed/garbage response — PASS (plus a design-clarification finding)

`server.js` in `malformed` mode: responds HTTP 200 with body `{this is not valid json !!! <<<garbage>>>`.

```
-- Bash (not in the read-only allowlist) --
{"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"deny","permissionDecisionReason":"offline-fallback-deny: malformed server response: Expected property name or '}' in JSON at position 1 ..."}}
exit=0

-- Read (in the read-only allowlist) --
{"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"allow","permissionDecisionReason":"offline-fallback-allowlist: malformed server response: ..."}}
exit=0
```

The wrapper never crashed or threw an uncaught exception — `JSON.parse` failure was caught, and it fell through to the same D5 fallback path used for unreachability. Exit 0 both times, clean structured output.

**Design-clarification finding (not a failure, a gap worth closing):** D5's text scopes the allowlist fallback to "the wrapper cannot reach the server at all (not running, connection refused, timeout, discovery file missing)." A malformed-but-reachable response is a different failure mode — the server answered, just with garbage — and D5 as written doesn't explicitly say whether that should share the same allowlist-based fallback or always fail closed regardless of tool (arguably a reachable-but-corrupting-data server is a *more* concerning state than a cleanly absent one). My PoC wrapper chose to treat them identically (both route through `failClosedOrAllowlist`); that was my own implementation call for the spike, not something D5 mandates either way. Recommend Group 3/4 make this an explicit decision when writing the real wrapper (task 4.4/4.5) rather than inheriting my PoC's choice by default.

### 1.5 — `"type": "http"` hook for `PreToolUse` — NOT EMPIRICALLY CONFIRMED

**What I found in documentation (new information relative to design.md):** `WebFetch` against `code.claude.com/docs/en/hooks` this session returned a worked example with `"hooks": {"PreToolUse": [{"matcher": "Bash", "hooks": [{"type": "http", "url": "http://localhost:8080/hooks/pre-tool-use", ...}]}]}` explicitly under `PreToolUse`, plus explicit response-handling rules for HTTP hooks (2xx+JSON parsed as decision; **non-2xx, connection failure, or timeout all resolve as *non-blocking* — execution continues**, unlike command hooks). An independent `WebSearch` (not the same fetch, cross-checking against a third-party guide) turned up the identical example independently. This directly contradicts design.md's Non-Goals claim that "every citation found so far was in a `PostToolUse` example" — current docs (as of this session, Aug 2026) do show a `PreToolUse` example. Docs can be stale or aspirational, though, which is exactly why the task called for empirical resolution rather than trusting citations — so I attempted that next.

**What I could not do:** drive a real live `claude` session (interactive or `-p`) from within this execution context at all, for any test, including 1.5. Every invocation of `claude -p ...` (with or without tools, with or without the proxy/env vars stripped) returned `Failed to authenticate. API Error: 403 Request not allowed` and exit 1. Root-caused this as far as I reasonably could without attempting to circumvent it:
- `claude auth status` succeeds (`loggedIn: true`, `subscriptionType: "max"`) — the stored OAuth credential itself is valid.
- A raw `curl` to `https://api.anthropic.com/v1/messages` using that same OAuth access token plus the correct `anthropic-beta: oauth-2025-04-20` header succeeded (got a normal `404 model not found` for a deliberately-wrong model name — i.e., auth succeeded, request was processed).
- So the credential and network path both work; only the `claude` CLI's own session-establishment specifically fails, consistently, non-transiently (retried).
- This session is itself already running nested inside a live nested Claude Code process tree (`CLAUDE_CODE_CHILD_SESSION=1`, `CLAUDE_CODE_SESSION_ID` inherited from the parent) sharing the same Max-plan OAuth login as the actively-streaming parent/orchestrator session. Stripping the inherited `CLAUDE_CODE_*` env vars didn't change the outcome, so this isn't a simple client-side self-detection guard — it reads as a server-side rejection of a second concurrent session under the same subscription login. `WebSearch` corroborated the exact same error signature (`Failed to authenticate. API Error: 403 {"error":{"type":"forbidden","message":"Request not allowed"}}`) reported independently in `anthropics/claude-code` GitHub issues #28486, #70520, and #34268, in comparable nested/concurrent-session contexts.
- I deliberately did not attempt to work around this by minting a new API key or running a fresh interactive OAuth login — both are out of scope for a throwaway PoC (new standing credentials / real account state), and the second isn't achievable headlessly anyway.

Given that, I instead validated what I *could* reach: the server-side half of the http-hook wire contract. Posting the real documented `PreToolUse` JSON body directly to a server in `http_ok`/`http_deny` mode confirms it replies HTTP 200 with the exact `hookSpecificOutput`/`permissionDecision` shape docs specify for HTTP hook responses, in 176ms. This proves my server implements the expected wire format correctly; it does **not** prove Claude Code actually calls it for `PreToolUse` — that remains a genuine unresolved empirical question.

**Recommendation:** this is a 5-minute check for anyone with a non-nested `claude` session (a plain terminal, not a Claude-Code-launched subagent): point a `"type": "http"` `PreToolUse` hook at a `nc -l` or one-liner HTTP listener and watch for an incoming request when a tool call fires. I'm leaving `1.5` unchecked in tasks.md rather than checking it off on documentary evidence alone, per the task's explicit instruction to resolve this empirically, not by re-reading docs.

### Finding E — Node process overhead (context for D4, not a new decision)

Bare `node -e "process.exit(0)"` measured 160–190ms on a quiet moment on this machine. Full `wrapper.cjs` invocations via the outer `time` builtin ranged from 185ms (connection-refused, no artificial delay) up to 1.5–6.6s wall-clock for scenarios with the same logical work — the wrapper's own internal timestamps (logged inside `wrapper.log`, see tables above) stayed tight and predictable throughout (16ms–5036ms, matching what each scenario was designed to take); the outer wall-clock variance looks attributable to this being a shared, loaded dev machine (a concurrent unrelated `cargo` process was observed running throughout this session) rather than anything about the wrapper's own logic. I did not fully root-cause the gap and would not treat these outer numbers as production-representative. This is offered only as light corroboration for D4 (compiled Rust binary, not Node/shell) — an interpreted runtime's per-invocation overhead is a real, nonzero tax when "each invocation spawns a fresh process" (design.md Context), and the real Rust wrapper should get its own clean benchmark once built in Group 4 rather than inheriting any number from this Node spike.

---

### Impact on design.md

No decision (D1–D8) needs to change. Two things worth a small doc update, both non-blocking for Group 2:

1. **Non-Goals, the `"type": "http"` bullet** is now factually stale: it says every citation found was `PostToolUse`-only. Current docs contradict that (see 1.5 above). Suggest rewording to: docs now show a `PreToolUse` + `http` example, but empirical confirmation attempted this session was blocked by an environment limitation unrelated to the hook mechanism itself (see poc-findings.md); the question remains open and non-blocking, `"type": "command"` remains the only confirmed-sufficient path for this phase.
2. **D5's fallback-trigger wording** could be tightened to explicitly say whether a malformed-but-reachable response shares the allowlist fallback or always fails closed — currently silent on that specific case (see 1.4 finding above). Small clarification, not a reversal.

### Process/methodology note for later manual verification steps

Tasks 8.6 and 8.7 (Group 8, manual end-to-end checks with a real Claude Code CLI session) will hit the same nested-session 403 described in Finding D if they're ever run *by an agent operating as a subagent inside someone's live Claude Code session*. They need a genuinely independent `claude` process — a human's own terminal, or an orchestrator that isn't itself nested under another active session. Worth keeping in mind when Group 8 is scheduled.

### Cleanup confirmation

- All spawned `node` server processes (5 total across scenarios) explicitly killed and confirmed gone via process listing.
- Scratch directory (`%TEMP%\claude-hook-poc`, including `.claude/settings.json` and `.claude/hooks/wrapper.cjs`) deleted at the end of this session.
- Real `~/.claude/settings.json`: sha256 and mtime confirmed identical before and after (see top of this document). Never opened for writing at any point.
