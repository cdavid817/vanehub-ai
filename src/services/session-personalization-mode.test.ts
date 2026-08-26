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

  it("refuses project-only with nothing to be isolated to, in the native runtime's words", async () => {
    // Refused outright rather than as a failed operation, matching the native runtime: starting an
    // operation for a request that was never answerable leaves the user a failed task to dismiss.
    // The message is the contract the frontend matches on, so the mock refusing in different words
    // would pass a looser assertion while leaving the desktop wording untested.
    await expect(
      webAgentClient.createSession({
        agentId: "codex-cli",
        interactionMode: "cli",
        personalizationMode: "project-only",
        title: "no workspace",
        projectPath: null,
        folder: null,
      }),
    ).rejects.toThrow("A project-only session needs a workspace to be isolated to.");
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
