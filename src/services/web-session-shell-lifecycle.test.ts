import { describe, expect, it, vi } from "vitest";
import type { SessionShellEvent } from "./session-shell-service";
import { createWebSessionShellClient } from "./web-session-shell-client";
import { sessionShellCleanupReport } from "../types/session-workspace-shell-frames";

/**
 * The lifecycle phases the browser build could not previously reach.
 *
 * The mock created straight into `running`, so startup, a fast exit before anything attached, and a
 * failed start never happened here at all — and a panel written against it would accept a keystroke
 * the desktop build refuses, and render an empty terminal for a command that had already finished.
 */
describe("web Shell startup", () => {
  it("holds a deferred Shell in opening and refuses input there", async () => {
    const client = createWebSessionShellClient({ deferStartupUntilAttach: true });

    const descriptor = await client.createSessionShell({ sessionId: "session-1", rows: 24, cols: 80 });

    expect(descriptor.state).toBe("opening");
    // Addressable and not writable, exactly as the native store has it. Nothing can be written yet
    // because there is no attachment either, which is the same order the real one enforces.
    await expect(
      client.writeSessionShell({
        shellId: descriptor.shellId,
        attachmentId: "attach-nobody",
        content: "ls\n",
      }),
    ).rejects.toThrow("shell_attachment_stale");
  });

  it("commits startup when a view attaches, and publishes the change", async () => {
    const client = createWebSessionShellClient({ deferStartupUntilAttach: true });
    const created = await client.createSessionShell({ sessionId: "session-1", rows: 24, cols: 80 });
    const listener = vi.fn<(event: SessionShellEvent) => void>();

    const attachment = await client.attachSessionShell({ shellId: created.shellId }, listener);

    // The snapshot carries the committed state rather than the one from before the attach. Handing
    // back `opening` and correcting it a microtask later would make every view handle a snapshot it
    // knows is already wrong.
    expect(attachment.descriptor.state).toBe("running");
    // And the notice is published anyway, to the listener registered before the commit. A view that
    // trusts the stream rather than the snapshot has to converge too — those are two different
    // readers of the same Shell, and only one of them called attach.
    const states = listener.mock.calls
      .map(([event]) => event)
      .filter((event) => event.type === "state");
    expect(states.at(-1)).toMatchObject({ state: "running" });

    await client.writeSessionShell({
      shellId: created.shellId,
      attachmentId: attachment.attachmentId,
      content: "ls\n",
    });
  });

  it("reports a startup that rolled back as failed rather than closed", async () => {
    const client = createWebSessionShellClient({
      deferStartupUntilAttach: true,
      startupFailureReason: "shell_spawn_failed",
    });
    const created = await client.createSessionShell({ sessionId: "session-1", rows: 24, cols: 80 });

    const attachment = await client.attachSessionShell({ shellId: created.shellId }, vi.fn());
    const [after] = await client.listSessionShells("session-1");

    // `failed` with the reason it rolled back for. A startup that never committed has nothing to
    // confirm, and reporting `closed` would claim a clean ending for a Shell that never began.
    expect(after.state).toBe("failed");
    expect(after.reason).toBe("shell_spawn_failed");
    expect(attachment.attachmentId).toBeTruthy();
  });
});

describe("web Shell fast exit", () => {
  it("retains what a command said before anything could attach", async () => {
    const client = createWebSessionShellClient({
      fastExit: { output: "done\n", exitCode: 0 },
    });
    const created = await client.createSessionShell({ sessionId: "session-1", rows: 24, cols: 80 });

    expect(created.state).toBe("exited");
    const attachment = await client.attachSessionShell({ shellId: created.shellId }, vi.fn());

    // The whole point of a replay. A view that rendered only live frames would show an empty
    // terminal for a command that had already said everything it was going to say.
    expect(attachment.replay.map((frame) => frame.data).join("")).toContain("done");
    expect(attachment.descriptor.exitCode).toBe(0);
  });
});

describe("a session-wide cleanup report", () => {
  it("names the Shells that blocked it and whether waiting will help", async () => {
    const client = createWebSessionShellClient({ closeAttemptsBeforeConfirming: 1 });
    const created = await client.createSessionShell({ sessionId: "session-1", rows: 24, cols: 80 });

    const outcome = await client.closeSessionShell(created.shellId);
    const report = sessionShellCleanupReport("session-1", await client.listSessionShells("session-1"));

    expect(outcome.disposition).toBe("reaping");
    // The refusal a session archive produces says only that something is winding down. This says
    // which — assembled from a call the view already makes rather than from a report crossing two
    // contexts.
    expect(report.pending.map((shell) => shell.shellId)).toEqual([created.shellId]);
    expect(report.retryable).toBe(true);
  });

  it("is empty once every Shell confirmed", async () => {
    const client = createWebSessionShellClient();
    const created = await client.createSessionShell({ sessionId: "session-1", rows: 24, cols: 80 });
    await client.closeSessionShell(created.shellId);

    const report = sessionShellCleanupReport("session-1", await client.listSessionShells("session-1"));

    expect(report.pending).toEqual([]);
    expect(report.retryable).toBe(false);
  });

  it("does not report another session's Shells", async () => {
    const client = createWebSessionShellClient({ closeAttemptsBeforeConfirming: 1 });
    const other = await client.createSessionShell({ sessionId: "session-2", rows: 24, cols: 80 });
    await client.closeSessionShell(other.shellId);

    const report = sessionShellCleanupReport("session-1", await client.listSessionShells("session-2"));

    // A report is about one session. Counting somebody else's unfinished cleanup would block an
    // archive on work that has nothing to do with it.
    expect(report.pending).toEqual([]);
  });
});
