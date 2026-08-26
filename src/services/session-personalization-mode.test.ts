// @vitest-environment jsdom

import { beforeEach, describe, expect, it, vi } from "vitest";
import type { Session } from "../types/agent";
import type { SessionPersonalizationMode } from "../types/personalization";
import { webAgentClient } from "./web-agent-client";
import { webOperationClient } from "./web-operation-client";

/**
 * A session keeps the mode it was created with.
 *
 * Policy is layered and changes over time; a session's mode is a promise made once, at creation,
 * about what that conversation will and will not use. If editing global policy could rewrite it, a
 * user would be told their session is temporary while it had quietly started reading memory.
 */
async function createSession(mode?: SessionPersonalizationMode): Promise<Session> {
  vi.useFakeTimers();
  const operation = await webAgentClient.createSession({
    // `codex-cli` is available in the mock registry without provider configuration, which keeps
    // this test about the mode rather than about Agent readiness.
    agentId: "codex-cli",
    interactionMode: "cli",
    ...(mode ? { personalizationMode: mode } : {}),
    title: `session-${mode ?? "default"}`,
    projectPath: "D:/app",
    folder: "D:/app",
  });
  await vi.advanceTimersByTimeAsync(950);
  const completed = await webOperationClient.getOperationStatus(operation.id);
  expect(completed.status).toBe("succeeded");
  vi.useRealTimers();
  return completed.result as unknown as Session;
}

describe("session personalization mode over the service boundary", () => {
  beforeEach(() => {
    window.localStorage.clear();
  });

  it("records the mode the caller asked for", async () => {
    const session = await createSession("temporary");

    expect(session.personalizationMode).toBe("temporary");
  });

  it("defaults to standard when the caller names no mode", async () => {
    const session = await createSession();

    expect(session.personalizationMode).toBe("standard");
  });

  it("keeps an existing session's mode when global policy changes", async () => {
    const created = await createSession("temporary");

    await webAgentClient.patchPersonalizationPolicy({
      scopeKind: "global",
      aboutUser: "Edited after the session was created.",
    });

    const sessions = await webAgentClient.listSessions();
    // The policy edit lands on the policy layer; nothing about it addresses a session row.
    expect(sessions.find((session) => session.id === created.id)?.personalizationMode).toBe(
      "temporary",
    );
  });

  it("keeps two sessions' modes independent of each other", async () => {
    const temporary = await createSession("temporary");
    const standard = await createSession("standard");

    const sessions = await webAgentClient.listSessions();

    expect(sessions.find((session) => session.id === temporary.id)?.personalizationMode).toBe(
      "temporary",
    );
    expect(sessions.find((session) => session.id === standard.id)?.personalizationMode).toBe(
      "standard",
    );
  });
});
