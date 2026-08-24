import { describe, expect, it } from "vitest";

import type { SessionShellNotice } from "../types/session-workspace-shell-frames";
import { createShellFrameDispatcher, onceDetach, orderBufferedNotices } from "./session-shell-frames";
import { createTauriSessionShellClient, type NativeShellTransport } from "./tauri-session-shell-client";
import { createWebSessionShellClient } from "./web-session-shell-client";
import type { SessionShellService } from "./session-shell-service";

function outputNotice(sequence: number, data: string, shellId = "shell-1"): SessionShellNotice {
  return {
    type: "output",
    shellId,
    sessionId: "session-1",
    sequence,
    occurredAt: "2026-08-22T09:00:00Z",
    stream: "pty",
    data,
  };
}

function descriptorPayload(shellId: string): Record<string, unknown> {
  return {
    shellId,
    sessionId: "session-1",
    title: "Shell 1",
    runtime: { kind: "native", supportsResize: true, supportsReplay: true, supportsReconnect: false },
    state: "running",
    createdAt: "2026-08-22T09:00:00Z",
    lastActivityAt: "2026-08-22T09:00:00Z",
    revision: 1,
    foregroundProcess: "unknown",
  };
}

/** A transport that records calls and hands back a controllable notice emitter. */
function recordingTransport(overrides: Record<string, unknown> = {}): {
  transport: NativeShellTransport;
  calls: { command: string; payload: Record<string, unknown> }[];
  emit(payload: unknown): void;
  subscribed: () => boolean;
} {
  const calls: { command: string; payload: Record<string, unknown> }[] = [];
  let handler: ((payload: unknown) => void) | null = null;
  return {
    calls,
    emit: (payload) => handler?.(payload),
    subscribed: () => handler !== null,
    transport: {
      async invokeShell(command, payload) {
        calls.push({ command, payload });
        if (command in overrides) return overrides[command];
        if (command === "attach_session_shell") {
          return {
            attachmentId: "attach-1",
            descriptor: descriptorPayload("shell-1"),
            replay: [],
            nextSequence: 3,
          };
        }
        if (command === "list_session_shells") return [descriptorPayload("shell-1")];
        if (command === "create_session_shell" || command === "rename_session_shell") {
          return descriptorPayload("shell-1");
        }
        return null;
      },
      async subscribeShellNotices(next) {
        handler = next;
        return () => {
          handler = null;
        };
      },
    },
  };
}

describe("shell frame dispatcher", () => {
  it("drops frames the attach snapshot already replayed", () => {
    const seen: number[] = [];
    const dispatcher = createShellFrameDispatcher({
      shellId: "shell-1",
      fromSequence: 3,
      listener: (notice) => {
        if (notice.type === "output") seen.push(notice.sequence);
      },
    });

    // 1 and 2 are inside the snapshot; delivering them again would look like the shell echoed.
    for (const sequence of [1, 2, 3, 4]) dispatcher.accept(outputNotice(sequence, "x"));

    expect(seen).toEqual([3, 4]);
  });

  it("reports a discontinuity before the frame that revealed it", () => {
    const gaps: { fromSequence: number; toSequence: number }[] = [];
    const seen: number[] = [];
    const dispatcher = createShellFrameDispatcher({
      shellId: "shell-1",
      fromSequence: 1,
      listener: (notice) => {
        if (notice.type === "output") seen.push(notice.sequence);
      },
      onGap: (gap) => gaps.push({ fromSequence: gap.fromSequence, toSequence: gap.toSequence }),
    });

    dispatcher.accept(outputNotice(1, "a"));
    dispatcher.accept(outputNotice(5, "b"));

    expect(gaps).toEqual([{ fromSequence: 2, toSequence: 4 }]);
    expect(seen).toEqual([1, 5]);
  });

  it("ignores frames belonging to another shell", () => {
    const seen: number[] = [];
    const dispatcher = createShellFrameDispatcher({
      shellId: "shell-1",
      fromSequence: 1,
      listener: (notice) => {
        if (notice.type === "output") seen.push(notice.sequence);
      },
    });

    dispatcher.accept(outputNotice(1, "a", "shell-2"));

    expect(seen).toEqual([]);
  });

  it("orders buffered frames by sequence rather than arrival", () => {
    const ordered = orderBufferedNotices([outputNotice(4, "d"), outputNotice(2, "b")]);

    expect(ordered.map((notice) => (notice.type === "output" ? notice.sequence : 0))).toEqual([2, 4]);
  });

  it("runs a detach once however many times cleanup fires", async () => {
    let detaches = 0;
    const detach = onceDetach(async () => {
      detaches += 1;
    });

    await Promise.all([detach(), detach(), detach()]);

    expect(detaches).toBe(1);
  });
});

describe("tauri session shell client", () => {
  it("registers the listener before it attaches", async () => {
    const { transport, calls, subscribed } = recordingTransport();
    const client = createTauriSessionShellClient(transport);
    let subscribedBeforeAttach = false;
    const original = transport.invokeShell.bind(transport);
    transport.invokeShell = async (command, payload) => {
      if (command === "attach_session_shell") subscribedBeforeAttach = subscribed();
      return original(command, payload);
    };

    await client.attachSessionShell({ shellId: "shell-1" }, () => {});

    // Attaching first would lose every frame published in the window, and the sequences the view
    // then saw would be contiguous, so nothing downstream could tell anything was missing.
    expect(subscribedBeforeAttach).toBe(true);
    expect(calls.map((call) => call.command)).toEqual(["attach_session_shell"]);
  });

  it("replays frames that arrived before the snapshot was known", async () => {
    const { transport, emit } = recordingTransport();
    const client = createTauriSessionShellClient(transport);
    const seen: number[] = [];
    const attaching = client.attachSessionShell({ shellId: "shell-1" }, (notice) => {
      if (notice.type === "output") seen.push(notice.sequence);
    });
    // Racing the attach: the subscription is up, the snapshot is not back yet.
    await Promise.resolve();
    emit(outputNotice(4, "d"));
    emit(outputNotice(3, "c"));

    await attaching;

    expect(seen).toEqual([3, 4]);
  });

  it("releases the listener when the attach fails", async () => {
    const { transport, subscribed } = recordingTransport();
    const failing: NativeShellTransport = {
      ...transport,
      async invokeShell(command, payload) {
        if (command === "attach_session_shell") throw new Error("shell_not_found");
        return transport.invokeShell(command, payload);
      },
    };
    const client = createTauriSessionShellClient(failing);

    await expect(client.attachSessionShell({ shellId: "shell-1" }, () => {})).rejects.toThrow();

    expect(subscribed()).toBe(false);
  });

  it("carries the attachment id back on every later call", async () => {
    const { transport, calls } = recordingTransport();
    const client = createTauriSessionShellClient(transport);

    const attachment = await client.attachSessionShell({ shellId: "shell-1" }, () => {});
    await client.writeSessionShell({ shellId: "shell-1", attachmentId: attachment.attachmentId, content: "ls\n" });
    await attachment.detach();
    await attachment.detach();

    const detaches = calls.filter((call) => call.command === "detach_session_shell");
    expect(calls[1]?.payload).toEqual({
      input: { shellId: "shell-1", attachmentId: "attach-1", content: "ls\n" },
    });
    // A React cleanup can run more than once; a second detach must not disturb whatever attached
    // after it, so it never reaches the registry at all.
    expect(detaches).toHaveLength(1);
  });
});

describe("web session shell client", () => {
  async function shellFor(client: SessionShellService, requestId?: string) {
    return client.createSessionShell({ sessionId: "session-1", rows: 24, cols: 80, requestId });
  }

  it("answers a retried create with the shell it already made", async () => {
    const client = createWebSessionShellClient();

    const first = await shellFor(client, "add-1");
    const retry = await shellFor(client, "add-1");
    const other = await shellFor(client, "add-2");

    expect(retry.shellId).toBe(first.shellId);
    expect(other.shellId).not.toBe(first.shellId);
  });

  it("keeps a shell alive across detach and replays what it missed", async () => {
    const client = createWebSessionShellClient();
    const shell = await shellFor(client);
    const first = await client.attachSessionShell({ shellId: shell.shellId }, () => {});
    await client.writeSessionShell({
      shellId: shell.shellId,
      attachmentId: first.attachmentId,
      content: "echo hi",
    });
    await first.detach();

    const again = await client.attachSessionShell(
      { shellId: shell.shellId, afterSequence: first.nextSequence - 1 },
      () => {},
    );

    // Detaching left the shell running, and reattaching from the last consumed sequence returns
    // exactly what happened while nobody was watching.
    expect(again.replay.map((frame) => frame.data)).toContain("echo hi");
    expect(await client.listSessionShells("session-1")).toHaveLength(1);
  });

  it("refuses a write from an attachment that was replaced", async () => {
    const client = createWebSessionShellClient();
    const shell = await shellFor(client);
    const stale = await client.attachSessionShell({ shellId: shell.shellId }, () => {});
    await client.attachSessionShell({ shellId: shell.shellId }, () => {});

    // Input is not idempotent: delivering a keystroke from a view the user has left would run it
    // in the session they are looking at now.
    await expect(
      client.writeSessionShell({
        shellId: shell.shellId,
        attachmentId: stale.attachmentId,
        content: "rm -rf .\n",
      }),
    ).rejects.toThrow("shell_attachment_stale");
  });

  it("leaves the newer attachment alone when a stale detach arrives", async () => {
    const client = createWebSessionShellClient();
    const shell = await shellFor(client);
    const stale = await client.attachSessionShell({ shellId: shell.shellId }, () => {});
    const current = await client.attachSessionShell({ shellId: shell.shellId }, () => {});

    await stale.detach();

    await expect(
      client.writeSessionShell({
        shellId: shell.shellId,
        attachmentId: current.attachmentId,
        content: "ls\n",
      }),
    ).resolves.toBeUndefined();
  });

  it("marks one contiguous gap when retained output is evicted", async () => {
    const client = createWebSessionShellClient();
    const shell = await shellFor(client);
    const attachment = await client.attachSessionShell({ shellId: shell.shellId }, () => {});
    for (let index = 0; index < 8; index += 1) {
      await client.writeSessionShell({
        shellId: shell.shellId,
        attachmentId: attachment.attachmentId,
        content: "x".repeat(1024),
      });
    }

    const reattached = await client.attachSessionShell({ shellId: shell.shellId }, () => {});

    expect(reattached.gap).toBeDefined();
    expect(reattached.gap?.reason).toBe("shell_replay_evicted");
    expect(reattached.gap?.fromSequence).toBeLessThanOrEqual(reattached.gap?.toSequence ?? 0);
  });

  it("closes only on an explicit close", async () => {
    const client = createWebSessionShellClient();
    const shell = await shellFor(client);
    const attachment = await client.attachSessionShell({ shellId: shell.shellId }, () => {});

    await attachment.detach();
    expect(await client.listSessionShells("session-1")).toHaveLength(1);

    await client.closeSessionShell(shell.shellId);
    expect(await client.listSessionShells("session-1")).toHaveLength(0);
    // Closing a shell that is already gone is a success, so a retry after a partial failure has
    // the same result as the first attempt.
    await expect(client.closeSessionShell(shell.shellId)).resolves.toBeUndefined();
  });
});
