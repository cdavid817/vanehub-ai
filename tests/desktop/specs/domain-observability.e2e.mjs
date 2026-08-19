import assert from "node:assert/strict";
import { execFile } from "node:child_process";
import { mkdir, mkdtemp, readFile, writeFile } from "node:fs/promises";
import { createServer } from "node:http";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import process from "node:process";
import { promisify } from "node:util";

const run = promisify(execFile);
const invoke = (fn, ...args) => globalThis.browser.tauri.execute(fn, ...args);
const blocked = [];

// The run root is the parent of the isolated data directory (scripts/desktop/run-context.mjs).
// `disposeRunContext` removes it once the app has exited, which is the only moment a directory a
// native child rooted itself in is actually unlocked -- deleting it from inside a test fails with
// EBUSY on Windows.
const fixtureRoot = process.env.VANEHUB_APP_DATA_DIR
  ? join(dirname(process.env.VANEHUB_APP_DATA_DIR), "fixtures")
  : tmpdir();

// Deliberately not 11434: smoke.e2e.mjs binds that port for local-endpoint discovery, and two
// specs in one run must not race for it.
const LOCAL_MODEL_PORT = 11533;
const LOCAL_MODEL_BASE_URL = `http://127.0.0.1:${LOCAL_MODEL_PORT}/v1`;
const LOCAL_MODEL_ID = "domain-sweep-local-model";
const LOCAL_PROFILE_NAME = "Domain sweep local model";

// The connector whose credential slot this spec is willing to borrow. Telegram takes exactly one
// secret (`botToken`, src-tauri/src/contexts/communications/domain/connector.rs:43-47), which
// keeps the save payload minimal, and it is saved disabled so no transport ever dials out.
const BORROWED_CONNECTOR = "telegram";

let repository;

/**
 * Runs a command and reports the rejection instead of throwing.
 *
 * `@wdio/tauri-service` collapses a rejected `core.invoke` into `e.message || String(e)`
 * (node_modules/@wdio/tauri-service/dist/esm/index.js:3141), so a structured error reaches the
 * spec as a bare string and its machine-readable code is lost. Communications commands reject
 * with their safe code as a plain string (src-tauri/src/commands/error.rs:89-96 and 151-161),
 * while execution observability rejects with `{ code, message, field }`
 * (src-tauri/src/commands/execution_observability/dto.rs:162-168) -- catching in the page keeps
 * both intact.
 */
async function attempt(command, args) {
  return invoke(({ core }, request) => core.invoke(request.command, request.args).then(
    (value) => ({ ok: true, value }),
    (error) => ({ ok: false, error }),
  ), { command, args: args ?? {} });
}

async function settle(operationId, message) {
  return globalThis.browser.waitUntil(async () => {
    // src-tauri/src/commands/operations/get_operation_status.rs:6-9 -- `operation_id`.
    const status = await invoke(
      ({ core }, id) => core.invoke("get_operation_status", { operationId: id }),
      operationId,
    );
    return ["succeeded", "failed", "cancelled"].includes(status.status) ? status : false;
  }, { timeout: 120_000, interval: 1_000, timeoutMsg: message });
}

async function createRepository(label, seed) {
  await mkdir(fixtureRoot, { recursive: true });
  const root = await mkdtemp(join(fixtureRoot, `${label}-`));
  await run("git", ["init"], { cwd: root });
  await run("git", ["config", "user.email", "desktop-e2e@example.invalid"], { cwd: root });
  await run("git", ["config", "user.name", "Desktop E2E"], { cwd: root });
  await writeFile(join(root, "seed.txt"), seed, "utf8");
  await run("git", ["add", "seed.txt"], { cwd: root });
  await run("git", ["commit", "-m", "fixture"], { cwd: root });
  return root;
}

/** src-tauri/src/commands/agent_runtime/list_agents.rs:7-9 -- `capability_tag`. */
async function listAgents() {
  return invoke(({ core }) => core.invoke("list_agents", { capabilityTag: null }));
}

async function availableCliAgent() {
  const agents = await listAgents();
  return agents.find((agent) => agent.availabilityState === "available"
    && agent.supportedInteractionModes.includes("cli")) ?? null;
}

/**
 * Any Agent a session can actually be created with. `ensure_selectable`
 * (contexts/agent_runtime/domain/catalog.rs:289-311) refuses an unavailable Agent, so a case that
 * only needs "a session that exists" still has to pick one this host can start. CLI is preferred:
 * session creation runs automatic code-index discovery for `onepiece` sessions only
 * (src-tauri/src/commands/sessions/background.rs:20-30), and this file also owns a code-index
 * case that must not inherit a workspace it did not register.
 */
async function anySessionAgent() {
  const cli = await availableCliAgent();
  if (cli) {
    return { id: cli.id, mode: "cli" };
  }
  const agents = await listAgents();
  const agent = agents.find((entry) => entry.availabilityState === "available"
    && entry.supportedInteractionModes.length > 0);
  return agent ? { id: agent.id, mode: agent.supportedInteractionModes[0] } : null;
}

/**
 * src-tauri/src/commands/sessions/create_session.rs:9-15 -- `input`, whose fields come from
 * `CreateSessionInput` (src-tauri/src/commands/sessions/dto.rs:171-183, camelCase).
 * Creation is asynchronous: the command hands back an `OperationTask` and the record only exists
 * once that operation reaches `succeeded`.
 */
async function createSession(agentId, interactionMode, title, projectPath) {
  const operation = await invoke(({ core }, input) => core.invoke("create_session", { input }), {
    agentId,
    interactionMode,
    title,
    folder: projectPath,
    projectPath,
    remoteWorkspace: null,
    worktree: null,
  });
  const settled = await settle(operation.id, `Creating the ${agentId} session never settled.`);
  assert.equal(settled.status, "succeeded", settled.error ?? `${agentId} session creation failed`);
  return globalThis.browser.waitUntil(async () => {
    const sessions = await invoke(({ core }) => core.invoke("list_sessions"));
    return sessions.find((item) => item.title === title) ?? false;
  }, { timeout: 30_000, timeoutMsg: `The "${title}" session was not created.` });
}

/** src-tauri/src/commands/sessions/delete_session.rs:6-11 -- `session_id`. */
async function deleteSession(sessionId) {
  await invoke(({ core }, id) => core.invoke("delete_session", { sessionId: id }), sessionId)
    .catch(() => {});
}

globalThis.describe("VaneHub AI desktop domain observability", () => {
  globalThis.before(async () => {
    const root = await globalThis.$("#root");
    await root.waitForExist({ timeout: 120_000 });
    await globalThis.browser.waitUntil(
      async () => (await root.getAttribute("data-vanehub-bootstrap")) === "ready",
      { timeout: 120_000, timeoutMsg: "React bootstrap did not become ready." },
    );
    repository = await createRepository("domain", "seed\n");
  });

  globalThis.it("reports the IM connector catalogue and refuses every unconfigured action", async () => {
    // src-tauri/src/commands/communications/list_im_connectors.rs:8 -- no arguments.
    // `ConnectorView` is src-tauri/src/commands/communications/dto.rs:8-15.
    const connectors = await invoke(({ core }) => core.invoke("list_im_connectors"));
    // The wire ids are not the Rust variant names: `ConnectorKind` is kebab-case with two explicit
    // renames, and WeChat ships as `weixin` (connector.rs:5-16).
    assert.deepEqual(
      connectors.map((entry) => entry.descriptor.kind).sort(),
      ["dingtalk", "feishu", "telegram", "wecom", "weixin"],
      "the built-in connector catalogue changed shape",
    );

    const wechat = connectors.find((entry) => entry.descriptor.kind === "weixin");
    // builtin_descriptors() (connector.rs:147-161) makes WeChat the only QR-authorized connector.
    assert.equal(wechat.descriptor.supportsQrAuthorization, true);
    assert.equal(wechat.descriptor.experimental, true);
    const telegram = connectors.find((entry) => entry.descriptor.kind === "telegram");
    assert.equal(telegram.descriptor.maxOutboundChars, 4_096);
    for (const entry of connectors) {
      // ConnectorHealth / ConnectorLifecycle: domain/status.rs:31-39 and 5-15 (kebab-case).
      assert.ok(
        ["unconfigured", "disabled", "connecting", "connected", "reconnecting",
          "authorization-expired", "error"].includes(entry.health.lifecycle),
        `${entry.descriptor.kind} reported an unknown lifecycle: ${entry.health.lifecycle}`,
      );
      assert.equal(typeof entry.hasCredentials, "boolean");
      // `credentialRef` is a keyring account name, never the secret itself
      // (contexts/communications/infrastructure/credential_adapter.rs:60-66), so it has to be
      // present as a nullable field rather than omitted.
      assert.ok(
        Object.prototype.hasOwnProperty.call(entry.config, "credentialRef"),
        `${entry.descriptor.kind} omitted its credential reference`,
      );
    }

    if (telegram.hasCredentials) {
      // Both of these dial the real service once a credential exists, so they are only safe while
      // the slot is empty. A populated slot is a host fact, recorded rather than worked around.
      blocked.push("IM connector lifecycle refusal: the Telegram credential slot is already populated on this host");
      return;
    }
    // src-tauri/src/commands/communications/test_im_connector.rs:7-8 and
    // restart_im_connector.rs:7-8 -- both take `kind`. With no stored credential
    // `load_runtime_definition` fails at `credentials_required()`
    // (contexts/communications/application/service.rs:996-998) before any socket is opened.
    for (const command of ["test_im_connector", "restart_im_connector"]) {
      const refused = await attempt(command, { kind: BORROWED_CONNECTOR });
      assert.equal(refused.ok, false, `${command} ran against an unconfigured connector`);
      assert.equal(refused.error, "connector-credentials-required", `${command}: ${refused.error}`);
    }
  });

  globalThis.it("saves and clears an IM connector credential without contacting the service", async function borrowSlot() {
    const before = await invoke(({ core }) => core.invoke("list_im_connectors"));
    const slot = before.find((entry) => entry.descriptor.kind === BORROWED_CONNECTOR);
    if (slot.hasCredentials) {
      // The credential store is the machine keyring (service `io.vanehub.ai.im`,
      // contexts/communications/infrastructure/credential_adapter.rs:11), which -- unlike the
      // SQLite database -- is not scoped to VANEHUB_APP_DATA_DIR. Overwriting a developer's real
      // token and then deleting it is not an acceptable side effect.
      blocked.push(`IM connector save/clear: ${BORROWED_CONNECTOR} already holds a credential in the machine keyring`);
      this.skip();
    }

    try {
      // src-tauri/src/commands/communications/save_im_connector.rs:8-11 -- `input`, shaped by
      // `SaveConnectorInput` (dto.rs:17-25). `enabled: false` is load-bearing: the enabled branch
      // of `save_connector` calls `transports.replace_and_start`
      // (application/service.rs:252-261), which would start a real Telegram long poll.
      const outcome = await attempt("save_im_connector", {
        input: {
          kind: BORROWED_CONNECTOR,
          enabled: false,
          displayName: "Desktop domain sweep",
          publicConfig: {},
          credentials: { botToken: "0000000000:desktop-domain-sweep-not-a-real-token" },
        },
      });
      if (!outcome.ok && outcome.error === "communications-credential-write-failed") {
        // The store is the OS keyring (credential_adapter.rs:178-180), which a headless host
        // without a secret service simply does not have. That is a host fact, not a defect.
        blocked.push("IM connector save/clear: the OS credential store rejected a write on this host");
        this.skip();
      }
      assert.equal(outcome.ok, true, `saving a disabled connector failed: ${outcome.error}`);
      const saved = outcome.value;
      assert.equal(saved.kind, BORROWED_CONNECTOR);
      assert.equal(saved.enabled, false, "a disabled save must not enable the connector");
      assert.equal(saved.displayName, "Desktop domain sweep");
      assert.ok(saved.credentialRef, "the credential was not stored behind a reference");
      // reject_sensitive_public_config (connector.rs:179-204) keeps secrets out of the public
      // half; the round trip has to prove the token is not echoed back anywhere.
      assert.equal(
        JSON.stringify(saved).includes("desktop-domain-sweep-not-a-real-token"),
        false,
        "the saved connector echoed its secret back",
      );

      const withCredential = (await invoke(({ core }) => core.invoke("list_im_connectors")))
        .find((entry) => entry.descriptor.kind === BORROWED_CONNECTOR);
      assert.equal(withCredential.hasCredentials, true, "the credential did not persist");
      assert.equal(withCredential.config.enabled, false);
    } finally {
      // src-tauri/src/commands/communications/clear_im_connector.rs:7-8 -- `kind`. This is the
      // only undo: it stops the transport, deletes the keyring entry and clears `credential_ref`
      // (application/service.rs:333-384).
      // Swallowed so a cleanup failure cannot mask the assertion that got here; the two
      // assertions below fail loudly if the credential really did survive.
      await invoke(({ core }, kind) => core.invoke("clear_im_connector", { kind }), BORROWED_CONNECTOR)
        .catch(() => {});
    }

    const cleared = (await invoke(({ core }) => core.invoke("list_im_connectors")))
      .find((entry) => entry.descriptor.kind === BORROWED_CONNECTOR);
    assert.equal(cleared.hasCredentials, false, "clearing left the credential in the keyring");
    assert.equal(cleared.config.credentialRef, null, "clearing left a dangling credential reference");
  });

  globalThis.it("binds IM routing to an installed CLI Agent and refuses anything else", async function bindRouting() {
    // src-tauri/src/commands/communications/get_im_routing.rs:7 -- no arguments; the payload is
    // `RoutingSettings` (contexts/communications/domain/routing.rs:43-48) or null.
    const original = await invoke(({ core }) => core.invoke("get_im_routing"));

    // src-tauri/src/commands/communications/save_im_routing.rs:7-9 -- `routing`. The adapter
    // resolves the Agent through the registry and rejects an unknown or unavailable one with
    // `default-agent-unavailable` (infrastructure/application_adapters.rs:46-56 and 161-166).
    const refused = await attempt("save_im_routing", {
      routing: { agentId: "not-an-agent", projectPath: repository },
    });
    assert.equal(refused.ok, false, "routing accepted an Agent that does not exist");
    assert.equal(refused.error, "default-agent-unavailable", `unexpected refusal: ${refused.error}`);

    const cli = await availableCliAgent();
    if (!cli) {
      blocked.push("IM routing: no installed CLI Agent on this host, and routing only accepts one");
      this.skip();
    }

    try {
      const saved = await invoke(({ core }, routing) => core.invoke("save_im_routing", { routing }), {
        agentId: cli.id,
        projectPath: repository,
      });
      assert.equal(saved.agentId, cli.id);
      // The stored path is the inspected project root, not the string that was sent
      // (application_adapters.rs:66-70), so this asserts on the identity of the directory.
      assert.ok(
        saved.projectPath.replaceAll("\\", "/").toLowerCase()
          .endsWith(repository.replaceAll("\\", "/").toLowerCase().split("/").pop()),
        `routing stored an unrelated project path: ${saved.projectPath}`,
      );
      assert.deepEqual(
        await invoke(({ core }) => core.invoke("get_im_routing")),
        saved,
        "the saved routing did not survive a read",
      );
    } finally {
      // Spec files in one run share a data directory, so a routing target left here is what the
      // next spec finds. There is no "unset" command, so only a pre-existing value is restorable.
      if (original) {
        await invoke(({ core }, routing) => core.invoke("save_im_routing", { routing }), original)
          .catch(() => {});
      }
    }
  });

  globalThis.it("guards the IM session binding surface when nothing is paired", async function bindingGuards() {
    // `begin_pairing` starts with `sessions.exists` (application/service.rs:536-540), so the
    // session-scoped half of this contract needs a real session id.
    const agent = await anySessionAgent();
    if (!agent) {
      blocked.push("IM binding guards: no Agent on this host is available enough to open a session");
      this.skip();
    }

    const session = await createSession(agent.id, agent.mode, "Domain sweep binding", repository);
    try {
      // src-tauri/src/commands/communications/get_im_session_binding.rs:7-9 -- `session_id`.
      // `SessionBindingView` is dto.rs:46-51.
      const empty = await invoke(({ core }, id) => core.invoke("get_im_session_binding", {
        sessionId: id,
      }), session.id);
      assert.deepEqual(empty, { binding: null, pendingConnector: null },
        "a session with no pairing reported a binding");

      // src-tauri/src/commands/communications/begin_im_pairing.rs:7-11 -- `session_id`,
      // `connector`, `replace_existing`. An unknown session is rejected before anything else.
      const unknownSession = await attempt("begin_im_pairing", {
        sessionId: "00000000-0000-0000-0000-000000000000",
        connector: BORROWED_CONNECTOR,
        replaceExisting: false,
      });
      assert.equal(unknownSession.ok, false, "pairing accepted a session that does not exist");
      assert.equal(unknownSession.error, "im-session-not-found", `unexpected: ${unknownSession.error}`);

      // A configured, enabled and *connected* connector is the second gate
      // (application/service.rs:541-564). Nothing here is connected, so this must be refused
      // rather than reaching out for a pairing code.
      const notReady = await attempt("begin_im_pairing", {
        sessionId: session.id,
        connector: BORROWED_CONNECTOR,
        replaceExisting: false,
      });
      assert.equal(notReady.ok, false, "pairing started against an unconfigured connector");
      assert.equal(notReady.error, "im-connector-not-ready", `unexpected: ${notReady.error}`);

      // src-tauri/src/commands/communications/cancel_im_pairing.rs:6-9 -- `session_id`,
      // `connector`. Cancelling a pairing that was never started is a no-op, not an error.
      // The connector travels as an argument, not through the closure: this arrow is serialised
      // and evaluated in the page, where a module constant from the spec does not exist.
      const cancelled = await invoke(({ core }, request) => core.invoke("cancel_im_pairing", {
        sessionId: request.sessionId,
        connector: request.connector,
      }), { sessionId: session.id, connector: BORROWED_CONNECTOR });
      assert.equal(cancelled, false, "cancelling a pairing that does not exist reported a change");

      // set_im_binding_paused.rs:6-9 and set_im_completion_notifications.rs:6-9 both update a row
      // that has to already exist; `changed == 0` becomes `im-binding-not-found`
      // (infrastructure/sqlite_repository.rs:137-142 and 157-162).
      const paused = await attempt("set_im_binding_paused", { sessionId: session.id, paused: true });
      assert.equal(paused.ok, false, "pausing a binding that does not exist succeeded");
      assert.equal(paused.error, "im-binding-not-found", `unexpected: ${paused.error}`);
      const notifications = await attempt("set_im_completion_notifications", {
        sessionId: session.id,
        enabled: true,
      });
      assert.equal(notifications.ok, false, "toggling notifications on a missing binding succeeded");
      assert.equal(notifications.error, "im-binding-not-found", `unexpected: ${notifications.error}`);

      // remove_im_session_binding.rs:6-8 -- `session_id`. Deleting nothing reports `false`
      // (sqlite_repository.rs:167-178) instead of failing, which is what makes it idempotent.
      const removed = await invoke(({ core }, id) => core.invoke("remove_im_session_binding", {
        sessionId: id,
      }), session.id);
      assert.equal(removed, false, "removing a binding that does not exist reported a deletion");

      // reset_im_bindings.rs:7-9 -- `kind` is optional; scoped to one connector on purpose so it
      // cannot clear bindings this spec did not create.
      const reset = await attempt("reset_im_bindings", { kind: BORROWED_CONNECTOR });
      assert.equal(reset.ok, true, `resetting one connector's bindings failed: ${reset.error}`);
    } finally {
      await deleteSession(session.id);
    }
  });

  globalThis.it("starts and cancels a pairing intent on a connected IM connector", async function pairingIntent() {
    const connectors = await invoke(({ core }) => core.invoke("list_im_connectors"));
    const connected = connectors.find((entry) => entry.health.lifecycle === "connected");
    if (!connected) {
      // Reaching `Connected` needs a real bot token for a real IM tenant, which this suite must
      // not carry. Reported as skipped rather than passed: the pairing code path never ran.
      blocked.push("IM pairing intent: no connector is connected -- needs a real IM bot credential on this host");
      this.skip();
    }
    const agent = await anySessionAgent();
    if (!agent) {
      blocked.push("IM pairing intent: no Agent on this host is available enough to open a session");
      this.skip();
    }

    const session = await createSession(agent.id, agent.mode, "Domain sweep pairing", repository);
    try {
      const started = await invoke(({ core }, request) => core.invoke("begin_im_pairing", request), {
        sessionId: session.id,
        connector: connected.descriptor.kind,
        replaceExisting: false,
      });
      // `PairingStartView` -- src-tauri/src/commands/communications/dto.rs:36-44.
      assert.equal(started.connector, connected.descriptor.kind);
      assert.equal(started.sessionId, session.id);
      assert.equal(started.replaceExisting, false);
      assert.ok(started.code.length > 0, "no pairing code was issued");
      assert.ok(Date.parse(started.expiresAt) > Date.now(), "the pairing code was born expired");

      const pending = await invoke(({ core }, id) => core.invoke("get_im_session_binding", {
        sessionId: id,
      }), session.id);
      assert.equal(pending.pendingConnector, connected.descriptor.kind,
        "the started intent is not visible on the session");
      assert.equal(pending.binding, null, "an unconfirmed pairing must not create a binding");

      // Stop here deliberately: completing the pairing needs `/bind <code>` to arrive from the IM
      // side, which means posting into a real chat.
      const cancelled = await invoke(({ core }, request) => core.invoke("cancel_im_pairing", request), {
        sessionId: session.id,
        connector: connected.descriptor.kind,
      });
      assert.equal(cancelled, true, "the started pairing intent could not be cancelled");
      const cleared = await invoke(({ core }, id) => core.invoke("get_im_session_binding", {
        sessionId: id,
      }), session.id);
      assert.equal(cleared.pendingConnector, null, "the cancelled intent is still pending");
    } finally {
      await deleteSession(session.id);
    }
  });

  globalThis.it("round-trips execution observability settings and reports observation capabilities", async () => {
    // src-tauri/src/commands/execution_observability/get_observability_settings.rs:6-8 -- no
    // arguments. `ObservabilitySettingsDto` is dto.rs:17-31; the enums on it are snake_case
    // (`CapturePolicyDto` dto.rs:4-9, `OtlpProtocolDto` dto.rs:11-15), not kebab or camel.
    const original = await invoke(({ core }) => core.invoke("get_observability_settings"));
    assert.equal(typeof original.localTimelineEnabled, "boolean");
    assert.equal(original.otlpProtocol, "http_protobuf");
    assert.ok(["metadata_only", "redacted_content"].includes(original.capturePolicy));
    assert.equal(
      Object.prototype.hasOwnProperty.call(original, "otlpAuthToken"),
      false,
      "the write-only OTLP token was serialized back to the client",
    );

    try {
      // update_observability_settings.rs:6-9 -- `settings`. `otlpAuthToken: null` maps to `None`,
      // which preserves whatever is in the machine keyring; `Some("")` would delete it and
      // `Some(secret)` would write one (contexts/execution_observability/api.rs:51-63).
      const updated = await invoke(({ core }, settings) => core.invoke("update_observability_settings", {
        settings,
      }), {
        ...original,
        samplingRatio: 0.5,
        retentionDays: 7,
        capturePolicy: "redacted_content",
        otlpAuthToken: null,
      });
      assert.equal(updated.samplingRatio, 0.5);
      assert.equal(updated.retentionDays, 7);
      assert.equal(updated.capturePolicy, "redacted_content");
      assert.deepEqual(
        await invoke(({ core }) => core.invoke("get_observability_settings")),
        updated,
        "the updated settings did not survive a read",
      );

      // Every rejection below is a domain guard in
      // contexts/execution_observability/domain/settings.rs:54-72, surfaced as
      // `{ code: "invalid_settings", field }` by mapper.rs:190-192.
      const rejections = [
        [{ ...original, retentionDays: 0 }, "retention_days"],
        [{ ...original, samplingRatio: 2 }, "sampling_ratio"],
        [{ ...original, otlpEnabled: true, otlpEndpoint: null }, "otlp_endpoint"],
        [{ ...original, otlpEndpoint: "https://user:secret@collector.example.com" }, "otlp_endpoint"],
      ];
      for (const [settings, field] of rejections) {
        const refused = await attempt("update_observability_settings", { settings });
        assert.equal(refused.ok, false, `${field} accepted an invalid value`);
        assert.equal(refused.error.code, "invalid_settings", `${field}: ${refused.error.code}`);
        assert.equal(refused.error.field, field, `unexpected field: ${refused.error.field}`);
      }
    } finally {
      await invoke(({ core }, settings) => core.invoke("update_observability_settings", { settings }), {
        ...original,
        otlpAuthToken: null,
      }).catch(() => {});
    }

    // get_execution_observation_capabilities.rs:6-8 -- no arguments, and the matrix is a fixed
    // four Agents crossed with two transports (contexts/execution_observability/api.rs:80-111).
    const capabilities = await invoke(({ core }) => (
      core.invoke("get_execution_observation_capabilities")
    ));
    assert.equal(capabilities.length, 8, "the observation capability matrix changed shape");
    assert.deepEqual(
      [...new Set(capabilities.map((entry) => entry.agentId))].sort(),
      ["claude-code", "codex-cli", "gemini-cli", "opencode"],
    );
    assert.deepEqual([...new Set(capabilities.map((entry) => entry.transport))].sort(), ["http", "stdio"]);
    for (const entry of capabilities) {
      assert.equal(entry.toolFidelity, "inferred", `${entry.agentId} claims non-inferred tool fidelity`);
      // A relay-less Agent must report `opaque` rather than pretending to observe MCP traffic.
      assert.equal(entry.mcpFidelity, entry.relaySupported ? "proxied" : "opaque");
      assert.ok(entry.detail.length > 0);
    }
    assert.deepEqual(
      capabilities.filter((entry) => entry.relaySupported).map((entry) => entry.agentId).sort(),
      ["claude-code", "claude-code", "codex-cli", "codex-cli"],
    );

    // list_execution_runs.rs:6-10 -- `request` (`PageRequestDto`, dto.rs:120-125) and an optional
    // `session_id`. `PageRequest::new` rejects a zero or >100 limit
    // (contexts/execution_observability/domain/pagination.rs:17-21).
    const page = await invoke(({ core }) => core.invoke("list_execution_runs", {
      request: { limit: 20, pageToken: null },
      sessionId: null,
    }));
    assert.ok(Array.isArray(page.items), "the run page is not readable");
    for (const [limit, label] of [[0, "zero"], [101, "over the ceiling"]]) {
      const refused = await attempt("list_execution_runs", {
        request: { limit, pageToken: null },
        sessionId: null,
      });
      assert.equal(refused.ok, false, `a ${label} page limit was accepted`);
      // Note the code: an invalid *size* is reported as `invalid_page_token`, because
      // list_execution_runs.rs:11-12 funnels every PageRequest error through `invalid_page`.
      assert.equal(refused.error.code, "invalid_page_token", `unexpected: ${refused.error.code}`);
    }

    // get_execution_run.rs:6-9 / get_execution_timeline.rs:6-9 -- `run_id`, parsed as a
    // 36-character hyphenated hex id (domain/identity.rs:6-23) before storage is touched.
    for (const command of ["get_execution_run", "get_execution_timeline"]) {
      const refused = await attempt(command, { runId: "not-a-run-id" });
      assert.equal(refused.ok, false, `${command} accepted a malformed run id`);
      assert.equal(refused.error.code, "run_not_found", `${command}: ${refused.error.code}`);
    }
  });

  globalThis.it("records a real execution trace for a deterministic local-model turn", async function localTrace() {
    // A trace only exists after a turn actually runs (the runtime calls `telemetry.start_run`
    // from contexts/agent_runtime/application/service.rs:2319). A live CLI Agent would make this
    // slow and host-dependent, so the turn is driven against a loopback server that answers the
    // OpenAI chat-completions shape -- real IPC, real runtime, no network.
    const settings = await invoke(({ core }) => core.invoke("get_observability_settings"));
    if (!settings.localTimelineEnabled) {
      // With the local timeline off, `CompositeExecutionTelemetry` has nothing to write to and
      // `list_execution_runs` is empty by design -- asserting on it would test the wrong thing.
      blocked.push("execution trace: localTimelineEnabled is off on this host, so no run is ever persisted");
      this.skip();
    }

    const requests = [];
    const server = createServer((request, response) => {
      requests.push(request.url);
      if (request.url === "/v1/models") {
        response.writeHead(200, { "content-type": "application/json" });
        response.end(JSON.stringify({ data: [{ id: LOCAL_MODEL_ID }] }));
        return;
      }
      if (request.url === "/v1/chat/completions") {
        response.writeHead(200, { "content-type": "text/event-stream" });
        response.write('data: {"choices":[{"index":0,"delta":{"content":"TRACED"},"finish_reason":null}]}\n\n');
        response.write('data: {"choices":[{"index":0,"delta":{},"finish_reason":"stop"}],"usage":{"prompt_tokens":4,"completion_tokens":2,"total_tokens":6}}\n\n');
        response.end("data: [DONE]\n\n");
        return;
      }
      response.writeHead(404);
      response.end();
    });
    await new Promise((resolve, reject) => {
      server.once("error", reject);
      server.listen(LOCAL_MODEL_PORT, "127.0.0.1", resolve);
    });

    let profileId = null;
    let sessionId = null;
    try {
      // save_custom_onepiece_provider_profile.rs:6-9 -- `input`, shaped by
      // `SaveCustomOnePieceProviderProfileInput` (commands/agent_runtime/dto.rs:374-393).
      // `runtimeKind: "local"` is required: the command refuses any runtime that is not local or
      // private, which is exactly what keeps this turn off the network.
      const profiles = await invoke(({ core }, input) => (
        core.invoke("save_custom_onepiece_provider_profile", { input })
      ), {
        id: null,
        name: LOCAL_PROFILE_NAME,
        baseUrl: LOCAL_MODEL_BASE_URL,
        modelId: LOCAL_MODEL_ID,
        runtimeKind: "local",
        authenticationMode: "none",
        apiKey: null,
        timeoutMs: 10_000,
        privacyClassification: "local",
        toolCallingCapability: "unsupported",
        imageInputCapability: "unsupported",
        structuredOutputCapability: "unknown",
        reasoningFieldCapability: "unsupported",
        contextWindowTokens: 8_192,
        reservedOutputTokens: 1_024,
      });
      const created = profiles.profiles.find((profile) => profile.name === LOCAL_PROFILE_NAME);
      assert.equal(created?.active, true, "the local profile did not become active");
      profileId = created.id;

      const session = await createSession("onepiece", "api", "Domain sweep trace", repository);
      sessionId = session.id;
      // send_message.rs:6-13 -- `session_id`, `content`, `config` (`ChatConfig`,
      // commands/agent_runtime/dto.rs:544-558), `file_references`, `runner`.
      await invoke(({ core }, request) => core.invoke("send_message", request), {
        sessionId: session.id,
        content: "Answer with the single word TRACED.",
        config: {
          agentId: "onepiece",
          interactionMode: "api",
          executionMode: "inherit",
          providerId: null,
          modelId: null,
          reasoningDepth: null,
          streaming: true,
          thinking: false,
          longContext: false,
        },
        fileReferences: null,
        runner: null,
      });

      // list_messages.rs:7-13 -- `session_id`, `limit`, `before_id`.
      const answered = await globalThis.browser.waitUntil(async () => {
        const messages = await invoke(({ core }, id) => core.invoke("list_messages", {
          sessionId: id,
          limit: null,
          beforeId: null,
        }), session.id);
        return messages.some((message) => message.role === "assistant"
          && (message.content ?? "").includes("TRACED"));
      }, { timeout: 60_000, interval: 1_000, timeoutMsg: "no reply" }).then(() => true, () => false);
      if (!answered) {
        // The one host condition that breaks a loopback turn: an inherited `all_proxy=socks5://…`
        // is honoured by the Rust HTTP client but not by NO_PROXY, so 127.0.0.1 never resolves to
        // this server. Reported as skipped rather than passed -- the trace assertions never ran.
        blocked.push(`execution trace: the loopback turn never answered (server saw ${requests.join(", ") || "no requests"}) -- check for an inherited socks5 all_proxy`);
        this.skip();
      }
      assert.ok(requests.includes("/v1/chat/completions"), "the turn never reached the local server");

      const runs = await globalThis.browser.waitUntil(async () => {
        const page = await invoke(({ core }, id) => core.invoke("list_execution_runs", {
          request: { limit: 20, pageToken: null },
          sessionId: id,
        }), session.id);
        return page.items.length > 0 ? page : false;
      }, { timeout: 30_000, interval: 1_000, timeoutMsg: "The completed turn produced no execution run." });
      assert.equal(runs.nextPageToken, null, "a single run should not need a second page");

      // `ExecutionRunSummaryDto` -- commands/execution_observability/dto.rs:70-85. The trace and
      // span ids are fixed-width lowercase hex (domain/identity.rs:30-57).
      const summary = runs.items[0];
      assert.equal(summary.sessionId, session.id);
      assert.equal(summary.agentId, "onepiece");
      assert.equal(summary.source, "desktop", "a desktop turn was attributed to another source");
      assert.equal(summary.sourceId, null);
      assert.match(summary.runId, /^[0-9a-f-]{36}$/);
      assert.match(summary.traceId, /^[0-9a-f]{32}$/);
      assert.match(summary.rootSpanId, /^[0-9a-f]{16}$/);
      assert.ok(
        ["accepted", "running", "succeeded", "failed", "cancelled", "incomplete"].includes(summary.status),
        `unexpected run status: ${summary.status}`,
      );

      const fetched = await invoke(({ core }, id) => core.invoke("get_execution_run", { runId: id }), summary.runId);
      assert.equal(fetched.runId, summary.runId);
      assert.equal(fetched.traceId, summary.traceId);

      // `ExecutionTimelineDto` -- dto.rs:112-118.
      const timeline = await invoke(({ core }, id) => (
        core.invoke("get_execution_timeline", { runId: id })
      ), summary.runId);
      assert.equal(timeline.run.runId, summary.runId);
      assert.ok(timeline.spans.length > 0, "the run recorded no spans");
      const root = timeline.spans.find((span) => span.spanId === summary.rootSpanId);
      assert.ok(root, "the timeline is missing the run's root span");
      assert.equal(root.parentSpanId, null, "the root span reported a parent");
      assert.equal(root.name, "vanehub.task.execute", `unexpected root span: ${root.name}`);
      for (const span of timeline.spans) {
        assert.ok(
          ["native", "proxied", "inferred", "opaque"].includes(span.fidelity),
          `${span.name} reported an unknown fidelity: ${span.fidelity}`,
        );
        assert.equal(typeof span.attributes, "object");
      }
      for (const event of timeline.events) {
        assert.equal(typeof event.sequence, "number");
        assert.ok(timeline.spans.some((span) => span.spanId === event.spanId),
          `event ${event.name} is anchored to a span that is not in the timeline`);
      }
    } finally {
      if (sessionId) {
        await deleteSession(sessionId);
      }
      if (profileId) {
        // An active provider profile left behind decides which profile the next spec finds
        // active. Retried because a turn is still flushing usage rows when this runs and a
        // `database is locked` here would silently hand the next spec the wrong one.
        await globalThis.browser.waitUntil(async () => {
          const removed = await attempt("delete_onepiece_provider_profile", { profileId });
          return removed.ok;
        }, { timeout: 30_000, interval: 1_000, timeoutMsg: "The sweep's provider profile could not be deleted." })
          .catch(() => {});
      }
      await new Promise((resolve) => server.close(() => resolve()));
    }
  });

  globalThis.it("extends a code review with comments, findings, a decision and a real revert", async function codeReview() {
    // smoke.e2e.mjs already covers open + load + a revert that is refused for a stale snapshot.
    // Everything here is the part it does not reach: the comment, decision, action and feedback
    // commands, and a revert that actually succeeds and puts the file back.
    const cli = await availableCliAgent();
    if (!cli) {
      blocked.push("code review: no installed CLI Agent, and session creation rejects an unavailable one");
      this.skip();
    }

    const reviewRepository = await createRepository("review", "before\n");
    await writeFile(join(reviewRepository, "seed.txt"), "after\n", "utf8");
    const session = await createSession(cli.id, "cli", "Domain sweep review", reviewRepository);
    let operationId = null;
    try {
      // open_code_review.rs:6-9 -- `session_id`; `ReviewSessionDto` is
      // commands/sessions/review_dto.rs:77-93.
      const review = await invoke(({ core }, id) => core.invoke("open_code_review", {
        sessionId: id,
      }), session.id);
      assert.equal(review.files.length, 1, "the review snapshot did not see the modified file");
      assert.equal(review.status, "active");
      assert.equal(review.decision, "pending");
      const file = review.files[0];

      // get_code_review.rs:6-9 -- `review_id`. Reopening by id has to find the same review rather
      // than snapshotting a second one.
      const reread = await invoke(({ core }, id) => core.invoke("get_code_review", { reviewId: id }), review.id);
      assert.equal(reread.id, review.id);
      assert.equal(reread.fingerprint, review.fingerprint);

      // load_code_review_file.rs:6-12 -- `session_id`, `path`, `expected_snapshot`.
      const diff = await invoke(({ core }, request) => core.invoke("load_code_review_file", request), {
        sessionId: session.id,
        path: file.path,
        expectedSnapshot: review.fingerprint,
      });
      assert.equal(diff.hunks.length, 1, "the one-line edit did not produce exactly one hunk");
      const hunk = diff.hunks[0];

      // add_code_review_comment.rs:9-20 -- `input` with a nested `ReviewAnchorInput`
      // (review_dto.rs:8-17). `side` must be "old" or "new" and `startLine` must be >= 1
      // (contexts/sessions/domain/review.rs:53-79); the anchored file must be one the review
      // already knows about (review.rs:283-303).
      const anchor = {
        filePath: file.path,
        side: "new",
        startLine: Math.max(1, hunk.newStart),
        endLine: Math.max(1, hunk.newStart),
        hunkFingerprint: hunk.fingerprint,
        contextFingerprint: hunk.contextFingerprints[0],
      };
      const comment = await invoke(({ core }, input) => core.invoke("add_code_review_comment", { input }), {
        reviewId: review.id,
        anchor,
        body: "Domain sweep: restore the committed line.",
      });
      assert.ok(comment.id, "the comment was persisted without an id");
      assert.equal(comment.status, "active");
      // Selected on creation by design (contexts/sessions/domain/review.rs:150): a comment just
      // written is part of the batch the reviewer is assembling, and would otherwise have to be
      // ticked immediately after being typed.
      assert.equal(comment.selected, true, "a new comment must join the outgoing batch");
      assert.equal(comment.anchor.state, "current");

      const unknownFile = await attempt("add_code_review_comment", {
        input: {
          reviewId: review.id,
          anchor: { ...anchor, filePath: "not-in-the-review.txt" },
          body: "anchored nowhere",
        },
      });
      assert.equal(unknownFile.ok, false, "a comment anchored outside the review was accepted");

      // Deselect first. A comment is selected the moment it is created
      // (contexts/sessions/domain/review.rs:150), so "nothing selected" has to be arranged rather
      // than assumed -- asserting the refusal without this step tests nothing, because the review
      // still has a selected comment to send.
      await invoke(({ core }, request) => core.invoke("select_code_review_comment", request), {
        reviewId: review.id,
        commentId: comment.id,
        selected: false,
      });

      // send_code_review_feedback.rs:13-18 -- `review_id`, `acknowledge_stale`. Only selected
      // comments are sent, so an unselected review is refused rather than sending nothing
      // (contexts/sessions/application/review.rs:354-356).
      const nothingSelected = await attempt("send_code_review_feedback", {
        reviewId: review.id,
        acknowledgeStale: false,
      });
      assert.equal(nothingSelected.ok, false, "feedback was sent with no comment selected");

      // select_code_review_comment.rs:7-12 -- `review_id`, `comment_id`, `selected`.
      const selected = await invoke(({ core }, request) => core.invoke("select_code_review_comment", request), {
        reviewId: review.id,
        commentId: comment.id,
        selected: true,
      });
      assert.equal(selected.comments.find((entry) => entry.id === comment.id).selected, true);

      const sent = await invoke(({ core }, request) => core.invoke("send_code_review_feedback", request), {
        reviewId: review.id,
        acknowledgeStale: false,
      });
      assert.ok(sent.messageId, "feedback reported no message id");
      // The feedback is a plain user message on the session, not an Agent turn
      // (src-tauri/src/bootstrap/sessions.rs:195-224), so it has to be readable straight back.
      const messages = await invoke(({ core }, id) => core.invoke("list_messages", {
        sessionId: id,
        limit: null,
        beforeId: null,
      }), session.id);
      const delivered = messages.find((message) => message.id === sent.messageId);
      assert.ok(delivered, "the feedback message is missing from the session");
      assert.equal(delivered.role, "user");
      assert.match(delivered.content, /restore the committed line/);

      // set_code_review_decision.rs:8-21 -- `review_id`, `decision`. The accepted strings are
      // kebab-case and hand-parsed, so "changesRequested" would be rejected.
      const decided = await invoke(({ core }, request) => core.invoke("set_code_review_decision", request), {
        reviewId: review.id,
        decision: "changes-requested",
      });
      assert.equal(decided.decision, "changes-requested");
      const badDecision = await attempt("set_code_review_decision", {
        reviewId: review.id,
        decision: "changesRequested",
      });
      assert.equal(badDecision.ok, false, "a camelCase decision was accepted");

      // start_code_review_action.rs:14-27 -- `review_id`, `action`. This only opens an operation
      // record (bootstrap/sessions.rs:132-151); it does not launch an Agent.
      const action = await invoke(({ core }, request) => core.invoke("start_code_review_action", request), {
        reviewId: review.id,
        action: "tests",
      });
      assert.ok(action.operationId, "no operation was opened for the review action");
      operationId = action.operationId;
      const operation = await invoke(({ core }, id) => core.invoke("get_operation_status", {
        operationId: id,
      }), operationId);
      assert.equal(operation.kind, "workspace", `unexpected operation kind: ${operation.kind}`);
      assert.equal(operation.status, "running");
      assert.equal(operation.relatedEntityId, review.id);

      // complete_code_review_action.rs:9-24 -- `input` with `findings[]`. Severity is parsed by
      // hand against exactly info/warning/error (application/review.rs:312-316).
      const projected = await invoke(({ core }, input) => core.invoke("complete_code_review_action", { input }), {
        reviewId: review.id,
        operationId,
        action: "tests",
        findings: [{ title: "Domain sweep finding", severity: "warning", anchor: null }],
      });
      assert.equal(projected.findings.length, 1);
      assert.equal(projected.findings[0].source, "tests");
      assert.equal(projected.findings[0].severity, "warning");
      assert.equal(projected.findings[0].operationId, operationId);
      const badSeverity = await attempt("complete_code_review_action", {
        input: {
          reviewId: review.id,
          operationId,
          action: "tests",
          findings: [{ title: "Unsafe", severity: "critical", anchor: null }],
        },
      });
      assert.equal(badSeverity.ok, false, "an unknown finding severity was accepted");

      // resolve_code_review_comment.rs:7-11 -- `review_id`, `comment_id`.
      const resolved = await invoke(({ core }, request) => core.invoke("resolve_code_review_comment", request), {
        reviewId: review.id,
        commentId: comment.id,
      });
      assert.equal(resolved.comments.find((entry) => entry.id === comment.id).status, "resolved");

      // revert_code_review_change.rs:8-21 -- `input`. `confirmed: false` is denied before the
      // working tree is touched (infrastructure/session_queries.rs:1196-1201), which is a
      // different gate from the stale-snapshot rejection smoke.e2e.mjs covers.
      const unconfirmed = await attempt("revert_code_review_change", {
        input: {
          sessionId: session.id,
          path: file.path,
          expectedSnapshot: review.fingerprint,
          hunkFingerprint: hunk.fingerprint,
          confirmed: false,
        },
      });
      assert.equal(unconfirmed.ok, false, "an unconfirmed revert rewrote the working tree");
      assert.equal(
        await readFile(join(reviewRepository, "seed.txt"), "utf8"),
        "after\n",
        "the refused revert still changed the file",
      );

      const receipt = await invoke(({ core }, input) => core.invoke("revert_code_review_change", { input }), {
        sessionId: session.id,
        path: file.path,
        expectedSnapshot: review.fingerprint,
        hunkFingerprint: hunk.fingerprint,
        confirmed: true,
      });
      // `ReviewRevertReceiptDto` -- review_dto.rs:267-275.
      assert.equal(receipt.path, file.path);
      assert.equal(receipt.revertedHunks, 1);
      assert.equal(receipt.simulated, false);
      assert.equal(receipt.previousSnapshot, review.fingerprint);
      assert.notEqual(receipt.resultingSnapshot, receipt.previousSnapshot,
        "the revert reported no change to the snapshot");
      // The real proof: git apply --reverse ran and the file is back to what was committed.
      // Line endings are normalised first. The content returns through git, and on a host with
      // `core.autocrlf` enabled a committed LF is checked out as CRLF — comparing exact bytes
      // fails on Windows for a revert that worked perfectly.
      assert.equal(
        (await readFile(join(reviewRepository, "seed.txt"), "utf8")).replaceAll("\r\n", "\n"),
        "before\n",
        "the confirmed revert did not restore the committed content",
      );
    } finally {
      if (operationId) {
        // `complete_code_review_action` projects findings but never closes the operation it was
        // handed, so the record would otherwise stay `running` for the rest of the run.
        await invoke(({ core }, id) => core.invoke("cancel_operation", { operationId: id }), operationId)
          .catch(() => {});
      }
      await deleteSession(session.id);
    }
  });

  globalThis.it("registers, configures and deletes a workspace code index", async () => {
    // get_retrieval_configuration.rs:19-22 and get_retrieval_index_status.rs:53-56 -- no
    // arguments; both reject with a bare category string, never a message
    // (contexts/retrieval/domain/error.rs:18-27).
    const configuration = await invoke(({ core }) => core.invoke("get_retrieval_configuration"));
    assert.ok(
      ["disabled", "local", "semantic"].includes(configuration.automaticCodeIndexMode),
      `unexpected automatic mode: ${configuration.automaticCodeIndexMode}`,
    );
    const status = await invoke(({ core }) => core.invoke("get_retrieval_index_status"));
    for (const key of ["indexed", "pending", "failed"]) {
      assert.equal(typeof status[key], "number", `index status is missing ${key}`);
    }

    try {
      // save_code_index_automatic_mode.rs:87-91 -- `mode`, hand-parsed against exactly
      // disabled/local/semantic (contexts/retrieval/domain/code_index.rs:30-37).
      await invoke(({ core }, mode) => core.invoke("save_code_index_automatic_mode", { mode }), "local");
      assert.equal(
        (await invoke(({ core }) => core.invoke("get_retrieval_configuration"))).automaticCodeIndexMode,
        "local",
      );
      const badMode = await attempt("save_code_index_automatic_mode", { mode: "turbo" });
      assert.equal(badMode.ok, false, "an unknown automatic mode was accepted");
      assert.equal(badMode.error, "validation", `unexpected category: ${badMode.error}`);
    } finally {
      await invoke(({ core }, mode) => core.invoke("save_code_index_automatic_mode", { mode }),
        configuration.automaticCodeIndexMode).catch(() => {});
    }

    // Its own directory, not the shared fixture: registration is idempotent on the canonical root
    // (code_index_repository.rs:43-48), so a root that automatic discovery already claimed would
    // hand this case a pre-existing `automatic` workspace instead of the one it registered.
    await mkdir(fixtureRoot, { recursive: true });
    const indexRoot = await mkdtemp(join(fixtureRoot, "index-"));
    await writeFile(join(indexRoot, "main.rs"), "fn main() {}\n", "utf8");

    // register_code_index_workspace.rs:7-11 -- `root`, `display_name`. Registration is pure local
    // work: it canonicalizes the path and writes one row
    // (contexts/retrieval/infrastructure/code_index_repository.rs:37-48). Nothing is scheduled,
    // because `CodeWorkspace::new` starts disabled in local mode (domain/code_index.rs:300-319).
    const workspace = await invoke(({ core }, root) => core.invoke("register_code_index_workspace", {
      root,
      displayName: "Domain sweep workspace",
    }), indexRoot);
    assert.ok(workspace.workspaceId, "the workspace was registered without an id");
    assert.equal(workspace.origin, "manual");
    assert.equal(workspace.enabled, false, "a freshly registered workspace must not index anything yet");
    assert.equal(workspace.mode, "local");
    assert.equal(workspace.generation, 0);
    assert.deepEqual(workspace.selectedRoots, [""]);
    assert.equal(workspace.status.phase, "disabled");

    try {
      const missingRoot = await attempt("register_code_index_workspace", {
        root: join(indexRoot, "does-not-exist"),
        displayName: "Nowhere",
      });
      assert.equal(missingRoot.ok, false, "a workspace was registered on a path that does not exist");
      assert.equal(missingRoot.error, "storage", `unexpected category: ${missingRoot.error}`);

      // get_code_index_workspace.rs:44-48 / get_code_index_status.rs:57-61 -- `workspace_id`.
      const fetched = await invoke(({ core }, id) => core.invoke("get_code_index_workspace", {
        workspaceId: id,
      }), workspace.workspaceId);
      assert.deepEqual(fetched.languages.slice().sort(), workspace.languages.slice().sort());
      const fetchedStatus = await invoke(({ core }, id) => core.invoke("get_code_index_status", {
        workspaceId: id,
      }), workspace.workspaceId);
      // Invariants the frontend normalizer also enforces (src/services/code-index-contract.ts).
      assert.ok(fetchedStatus.processedFiles <= fetchedStatus.totalFiles);
      assert.equal(fetchedStatus.processedChunks, fetchedStatus.indexedChunks + fetchedStatus.failedChunks);
      assert.equal(fetchedStatus.totalChunks, fetchedStatus.processedChunks + fetchedStatus.pendingChunks);

      // save_code_index_configuration.rs:95-100 -- `workspace_id` plus a `configuration` object
      // whose six fields are all required (commands/retrieval/code_index/dto.rs:7-16); there is no
      // `#[serde(default)]` on any of them. Left disabled on purpose: `reconcile_workspace`
      // returns immediately for a disabled workspace
      // (contexts/retrieval/infrastructure/code_reconciler.rs:66-69), which is what keeps this
      // case away from a real indexing pass.
      const configured = await invoke(({ core }, request) => (
        core.invoke("save_code_index_configuration", request)
      ), {
        workspaceId: workspace.workspaceId,
        configuration: {
          enabled: false,
          mode: "local",
          selectedRoots: [""],
          languages: ["rust"],
          exclusionPatterns: ["target/**"],
          maxFileBytes: 102_400,
        },
      });
      assert.deepEqual(configured.languages, ["rust"]);
      assert.deepEqual(configured.exclusionPatterns, ["target/**"]);
      assert.equal(configured.enabled, false);
      // Saving a configuration bumps the generation and clears any embedding confirmation
      // (code_index_repository.rs:235-236), which is what makes a stale confirmation detectable.
      assert.equal(configured.generation, workspace.generation + 1);

      for (const [configuration, label] of [
        [{ enabled: false, mode: "local", selectedRoots: [""], languages: ["cobol"], exclusionPatterns: [], maxFileBytes: 102_400 }, "an unsupported language"],
        [{ enabled: false, mode: "turbo", selectedRoots: [""], languages: ["rust"], exclusionPatterns: [], maxFileBytes: 102_400 }, "an unsupported mode"],
        [{ enabled: false, mode: "local", selectedRoots: [""], languages: ["rust"], exclusionPatterns: [], maxFileBytes: 0 }, "a zero file ceiling"],
        [{ enabled: false, mode: "local", selectedRoots: ["../escape"], languages: ["rust"], exclusionPatterns: [], maxFileBytes: 102_400 }, "a root that escapes the workspace"],
      ]) {
        const refused = await attempt("save_code_index_configuration", {
          workspaceId: workspace.workspaceId,
          configuration,
        });
        assert.equal(refused.ok, false, `${label} was accepted`);
        assert.equal(refused.error, "validation", `${label}: ${refused.error}`);
      }

      // refresh_code_index_workspace.rs:112-116 -- `workspace_id`. It runs the reconciliation pass
      // inline, so it is only cheap while the workspace is disabled.
      const refreshed = await invoke(({ core }, id) => core.invoke("refresh_code_index_workspace", {
        workspaceId: id,
      }), workspace.workspaceId);
      assert.equal(refreshed.phase, "disabled", "a disabled workspace started indexing");

      // rebuild_code_index_workspace.rs:6 -- `workspace_id`. It clears indexed files, bumps the
      // generation and writes one audit row.
      const rebuilt = await invoke(({ core }, id) => core.invoke("rebuild_code_index_workspace", {
        workspaceId: id,
      }), workspace.workspaceId);
      assert.equal(rebuilt.generation, configured.generation + 1);

      // list_code_index_audit.rs:70-75 -- `workspace_id` and an optional `limit`.
      const audit = await invoke(({ core }, id) => core.invoke("list_code_index_audit", {
        workspaceId: id,
        limit: 20,
      }), workspace.workspaceId);
      assert.ok(audit.some((entry) => entry.event === "rebuilt"),
        "the rebuild left no audit trail");
      for (const entry of audit) {
        assert.equal(entry.workspaceId, workspace.workspaceId);
        assert.ok(
          ["admitted", "skipped", "indexed", "failed", "deleted", "rebuilt"].includes(entry.event),
          `unknown audit event: ${entry.event}`,
        );
      }

      const unknown = await attempt("get_code_index_status", { workspaceId: "not-a-workspace" });
      assert.equal(unknown.ok, false, "status was served for a workspace that does not exist");
      assert.equal(unknown.error, "invalid_scope", `unexpected category: ${unknown.error}`);
    } finally {
      // delete_code_index_workspace.rs:85-89 -- `workspace_id`. This is the only full teardown:
      // `disable_code_index_workspace` keeps the row, its files and its audit trail.
      await invoke(({ core }, id) => core.invoke("delete_code_index_workspace", {
        workspaceId: id,
      }), workspace.workspaceId).catch(() => {});
    }

    const remaining = await invoke(({ core }) => core.invoke("list_code_index_workspaces"));
    assert.equal(
      remaining.find((entry) => entry.workspaceId === workspace.workspaceId),
      undefined,
      "the deleted workspace is still registered",
    );
    const gone = await attempt("get_code_index_workspace", { workspaceId: workspace.workspaceId });
    assert.equal(gone.ok, false, "a deleted workspace is still readable");
    assert.equal(gone.error, "invalid_scope", `unexpected category: ${gone.error}`);
  });

  globalThis.it("requeues the global retrieval index when no embedding provider is configured", async function requeue() {
    const configuration = await invoke(({ core }) => core.invoke("get_retrieval_configuration"));
    if (configuration.sourceProfileId && configuration.embeddingModel) {
      // `rebuild_retrieval_index` requeues rows and wakes the background worker
      // (contexts/retrieval/api.rs:225-230). With a resolved model the woken worker starts
      // calling the embedding provider, which this suite must not trigger.
      blocked.push("retrieval rebuild: an embedding provider is configured, so a requeue would start real embedding calls");
      this.skip();
    }

    // rebuild_retrieval_index.rs:108-111 -- no arguments. Safe here precisely because
    // `resolved_model()` is empty, so the worker has nothing to embed with.
    const rebuilt = await attempt("rebuild_retrieval_index", {});
    assert.equal(rebuilt.ok, true, `the retrieval requeue failed: ${rebuilt.error}`);
    const status = await invoke(({ core }) => core.invoke("get_retrieval_index_status"));
    assert.equal(status.failed, 0, `the requeue left failed rows behind: ${status.lastFailureCategory}`);
  });

  globalThis.after(async () => {
    // No UI navigation happens in this file, so there is no route to restore -- every case goes
    // through the command surface. Teardown is otherwise left to WDIO: calling `exit_application`
    // here raced its `deleteSession` and discarded every per-test result for the file.
    if (blocked.length > 0) {
      globalThis.console.warn(`BLOCKED on this host:\n  ${blocked.join("\n  ")}`);
    }
  });
});
