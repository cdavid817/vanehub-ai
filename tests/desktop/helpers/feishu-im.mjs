import assert from "node:assert/strict";
import { execFile } from "node:child_process";
import { mkdtemp, readFile, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { promisify } from "node:util";
import { fileURLToPath, URL } from "node:url";

const run = promisify(execFile);
const invoke = (fn, ...args) => globalThis.browser.tauri.execute(fn, ...args);

export const persistedSessionTitle = "Feishu IM persisted fixture";
export const events = JSON.parse(await readFile(
  fileURLToPath(new URL("../fixtures/feishu/events.json", import.meta.url)),
  "utf8",
));

export const coreInvoke = (command, payload = {}) => invoke(
  ({ core }, [name, args]) => core.invoke(name, args),
  [command, payload],
);

export const listMessages = (sessionId) => coreInvoke("list_messages", {
  sessionId,
  limit: null,
  beforeId: null,
});

export const comparablePath = (value) => value?.startsWith("\\\\?\\") ? value.slice(4) : value;

export async function createFeishuSession(title = "Feishu IM fixture") {
  const repository = await mkdtemp(join(tmpdir(), "vanehub-feishu-im-"));
  await run("git", ["init"], { cwd: repository });
  await writeFile(join(repository, "readme.md"), "Feishu IM desktop fixture\n", "utf8");
  const operation = await invoke(({ core }, [projectPath, sessionTitle]) => core.invoke("create_session", {
    input: {
      agentId: "opencode",
      interactionMode: "cli",
      title: sessionTitle,
      folder: projectPath,
      projectPath,
      remoteWorkspace: null,
      worktree: null,
    },
  }), [repository, title]);
  const settled = await globalThis.browser.waitUntil(async () => {
    const status = await invoke(
      ({ core }, operationId) => core.invoke("get_operation_status", { operationId }),
      operation.id,
    );
    return ["succeeded", "failed", "cancelled"].includes(status.status) ? status : false;
  }, { timeout: 60_000, timeoutMsg: "Creating the Feishu IM fixture session never settled." });
  assert.equal(settled.status, "succeeded", settled.error ?? "session creation failed");
  const session = await findSession(title);
  return { ...session, fixtureProjectPath: repository };
}

export async function createMultiAgentSession(title = "Feishu multi-Agent fixture") {
  const created = await createFeishuSession(title);
  return promoteToMultiAgentSession(created);
}

export async function promoteToMultiAgentSession(created) {
  const current = await findSession(created.title);
  const roles = await coreInvoke("list_expert_roles");
  const seatRoles = ["builtin-architect", "builtin-implementer"]
    .map((id) => roles.find((role) => role.id === id));
  assert.ok(seatRoles.every(Boolean), "the two built-in fixture roles are unavailable");
  const session = await coreInvoke("update_session_seats", {
    input: {
      sessionId: created.id,
      expectedUpdatedAt: current.updatedAt,
      seats: seatRoles.map((role) => ({ agentId: "opencode", roleId: role.id })),
    },
  });
  const seats = session.seats.filter((seat) => !seat.leftAt);
  assert.equal(seats.length, 2, "the fixture did not persist two active seats");
  return {
    ...session,
    fixtureProjectPath: created.fixtureProjectPath,
    seats,
    handles: seatRoles.map((role) => role.displayName.split(/\s+/u).filter(Boolean).join("-")),
  };
}

export async function findSession(title) {
  return globalThis.browser.waitUntil(async () => {
    const sessions = await coreInvoke("list_sessions");
    return sessions.find((session) => session.title === title) ?? false;
  }, { timeout: 30_000, timeoutMsg: `The Feishu IM fixture session '${title}' was not listed.` });
}

export async function openSessionIm(sessionId) {
  await globalThis.browser.execute((target) => {
    globalThis.history.pushState({}, "", target);
    globalThis.dispatchEvent(new globalThis.PopStateEvent("popstate"));
  }, `/workspace/sessions/${encodeURIComponent(sessionId)}`);
  const tab = await globalThis.$('[data-testid="accordion-header-im"]');
  await tab.waitForClickable({ timeout: 30_000 });
  await tab.click();
  const accessSwitch = await globalThis.$('[data-testid="session-im-access"] [role="switch"]');
  await accessSwitch.waitForDisplayed({ timeout: 30_000 });
  return accessSwitch;
}
