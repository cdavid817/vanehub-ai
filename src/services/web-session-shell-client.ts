import type {
  SessionShellDescriptor,
  SessionShellNotice,
  ShellCloseOutcome,
  ShellOutputFrame,
} from "../types/session-workspace-shell-frames";
import { onceDetach } from "./session-shell-frames";
import type {
  AttachSessionShellInput,
  CreateSessionShellInput,
  ResizeSessionShellInput,
  SessionShellEvent,
  SessionShellService,
  ShellAttachment,
  ShellAttachmentScope,
  WriteSessionShellInput,
} from "./session-shell-service";
import type { MockShell, WebSessionShellOptions } from "./web-session-shell-state";
import {
  alreadyTerminalClose,
  confirmedClose,
  evictRetainedFrames,
  unconfirmedClose,
} from "./web-session-shell-state";

export type { WebSessionShellOptions } from "./web-session-shell-state";
export {
  MOCK_SHELL_RETAINED_CHARACTERS,
  SHELL_RETAINED_OUTPUT_BYTES,
} from "./web-session-shell-state";

/**
 * A deterministic multi-Shell mock.
 *
 * Deterministic means no clock and no randomness: ids count up, timestamps advance by a fixed step,
 * and the same sequence of calls produces the same transcript on every run. A mock that reached for
 * `Date.now()` would make its own tests flaky and would let a component that depends on ordering
 * pass here and fail on the desktop.
 */
export function createWebSessionShellClient(
  options: WebSessionShellOptions = {},
): SessionShellService {
  const perSessionCapacity = options.perSessionCapacity ?? Number.POSITIVE_INFINITY;
  const closeAttemptsBeforeConfirming = options.closeAttemptsBeforeConfirming ?? 0;
  const shells = new Map<string, MockShell>();
  const listeners = new Map<string, (event: SessionShellEvent) => void>();
  let counter = 0;
  const nextId = (prefix: string): string => `${prefix}-${++counter}`;
  // A fixed epoch advanced by a fixed step, so a transcript is comparable across runs.
  const timestampAt = (step: number): string =>
    new Date(Date.UTC(2026, 0, 1, 0, 0, 0) + step * 1000).toISOString();

  function publish(shell: MockShell, notice: SessionShellNotice): void {
    if (!shell.attachmentId) return;
    listeners.get(shell.descriptor.shellId)?.(notice);
  }

  function emitOutput(shell: MockShell, data: string): void {
    const frame: ShellOutputFrame = {
      shellId: shell.descriptor.shellId,
      sequence: shell.nextSequence++,
      occurredAt: timestampAt(shell.nextSequence),
      stream: "pty",
      data,
    };
    shell.frames.push(frame);
    shell.descriptor = { ...shell.descriptor, lastActivityAt: frame.occurredAt };
    evictRetainedFrames(shell);
    publish(shell, { type: "output", sessionId: shell.descriptor.sessionId, ...frame });
  }

  function require_(shellId: string): MockShell {
    const shell = shells.get(shellId);
    if (!shell) throw new Error("shell_not_found");
    return shell;
  }

  function authorize(scope: ShellAttachmentScope): MockShell {
    const shell = require_(scope.shellId);
    if (shell.attachmentId !== scope.attachmentId) throw new Error("shell_attachment_stale");
    // `running` alone, which is what the native store allows. `opening` is addressable and not
    // writable: a keystroke accepted there would race the handoff that decides whether the Shell
    // exists at all, and answering it with success would say it reached a process.
    if (shell.descriptor.state !== "running") {
      throw new Error("shell_not_accepting_input");
    }
    return shell;
  }

  function setState(shell: MockShell, state: SessionShellDescriptor["state"]): void {
    shell.descriptor = {
      ...shell.descriptor,
      state,
      revision: shell.descriptor.revision + 1,
      lastActivityAt: timestampAt(shell.nextSequence),
    };
    publish(shell, {
      type: "state",
      shellId: shell.descriptor.shellId,
      generation: shell.descriptor.generation,
      sessionId: shell.descriptor.sessionId,
      state,
      revision: shell.descriptor.revision,
      occurredAt: shell.descriptor.lastActivityAt,
    });
  }

  return {
    async listSessionShells(sessionId: string): Promise<SessionShellDescriptor[]> {
      return [...shells.values()]
        .filter((shell) => shell.descriptor.sessionId === sessionId)
        .map((shell) => shell.descriptor);
    },

    async createSessionShell(input: CreateSessionShellInput): Promise<SessionShellDescriptor> {
      // The same idempotency rule the native registry applies: a retried Add returns the Shell it
      // already made, and two tabs opening the default Shell get one process rather than two.
      const existing = [...shells.values()].find((shell) =>
        input.requestId
          ? shell.requestId === input.requestId
          : shell.requestId === null &&
            shell.descriptor.sessionId === input.sessionId &&
            shell.descriptor.seatId === input.seatId &&
            shell.descriptor.state !== "closed",
      );
      if (existing) return existing.descriptor;

      // Refused before anything is created, mirroring a controller that reserves before it spawns.
      const live = [...shells.values()].filter(
        (shell) => shell.descriptor.sessionId === input.sessionId,
      ).length;
      if (live >= perSessionCapacity) throw new Error("shell_session_capacity_reached");

      const shellId = nextId("shell");
      const createdAt = timestampAt(counter);
      const shell: MockShell = {
        descriptor: {
          shellId,
          // Counts up with the id, so a stale-generation test names a number it chose rather than
          // reaching into an implementation detail.
          generation: counter,
          sessionId: input.sessionId,
          seatId: input.seatId,
          title: input.title ?? `Shell ${shells.size + 1}`,
          // A simulated runtime has no geometry to change, and the descriptor says so rather than
          // claiming a capability the mock cannot honour.
          runtime: {
            kind: "simulated",
            supportsResize: false,
            supportsReplay: true,
            supportsReconnect: false,
          },
          // `opening` when startup is deferred: registered and addressable, runtime not yet
          // committed. The default stays `running`, which is what a mock with no process to wait
          // for honestly is.
          state: options.deferStartupUntilAttach ? "opening" : "running",
          createdAt,
          lastActivityAt: createdAt,
          revision: 1,
          // The mock knows exactly what it is running, so it answers rather than shrugging. That is
          // what makes the three-state foreground warning testable without a real PTY.
          foregroundProcess: "absent",
        },
        frames: [],
        gap: null,
        attachmentId: null,
        requestId: input.requestId ?? null,
        nextSequence: 1,
        closeAttempts: 0,
      };
      shells.set(shellId, shell);
      if (options.fastExit) {
        // Before anything can attach. A view that only rendered live frames would show an empty
        // terminal for a Shell that had already said everything it was going to say.
        emitOutput(shell, options.fastExit.output);
        shell.descriptor = { ...shell.descriptor, exitCode: options.fastExit.exitCode };
        setState(shell, "exited");
        return shell.descriptor;
      }
      emitOutput(shell, `${input.sessionId} $ `);
      return shell.descriptor;
    },

    async attachSessionShell(
      input: AttachSessionShellInput,
      listener: (event: SessionShellEvent) => void,
    ): Promise<ShellAttachment> {
      const shell = require_(input.shellId);
      const attachmentId = nextId("attach");
      // The previous attachment is displaced rather than refused: a view that never ran its cleanup
      // must not lock the Shell out of reach.
      shell.attachmentId = attachmentId;
      listeners.set(input.shellId, listener);
      // Committed here rather than on a timer. Attaching is the one call in the mock's world that
      // maps to "something is watching this Shell now", and a timer would be a second clock every
      // test waiting on it would race.
      if (shell.descriptor.state === "opening") {
        if (options.startupFailureReason) {
          shell.descriptor = { ...shell.descriptor, reason: options.startupFailureReason };
          setState(shell, "failed");
        } else {
          setState(shell, "running");
        }
      }
      const after = input.afterSequence ?? 0;
      const replay = shell.frames.filter((frame) => frame.sequence > after);
      return {
        attachmentId,
        descriptor: shell.descriptor,
        replay,
        nextSequence: shell.nextSequence,
        gap: shell.gap ?? undefined,
        detach: onceDetach(async () => {
          if (shell.attachmentId !== attachmentId) return;
          shell.attachmentId = null;
          listeners.delete(input.shellId);
        }),
      };
    },

    async detachSessionShell(scope: ShellAttachmentScope): Promise<void> {
      const shell = shells.get(scope.shellId);
      // A stale detach is a no-op, never a teardown of the attachment that replaced it.
      if (!shell || shell.attachmentId !== scope.attachmentId) return;
      shell.attachmentId = null;
      listeners.delete(scope.shellId);
    },

    async writeSessionShell(input: WriteSessionShellInput): Promise<void> {
      const shell = authorize(input);
      emitOutput(shell, input.content);
      if (input.content.includes("\n")) {
        emitOutput(shell, `\n${shell.descriptor.sessionId} $ `);
      }
    },

    async resizeSessionShell(input: ResizeSessionShellInput): Promise<void> {
      authorize(input);
    },

    async renameSessionShell(input: {
      shellId: string;
      title: string;
    }): Promise<SessionShellDescriptor> {
      const shell = require_(input.shellId);
      const title = input.title.trim();
      if (!title) throw new Error("shell_invalid_title");
      shell.descriptor = {
        ...shell.descriptor,
        title,
        revision: shell.descriptor.revision + 1,
      };
      return shell.descriptor;
    },

    async closeSessionShell(shellId: string): Promise<ShellCloseOutcome> {
      const shell = shells.get(shellId);
      // Closing a Shell that is already gone is settled, so a retry after a partial failure has the
      // same result as the first attempt. Reported as `already_terminal` rather than as a confirmed
      // close: this call ended nothing.
      if (!shell) return alreadyTerminalClose(shellId);
      shell.closeAttempts += 1;
      const attempt = shell.closeAttempts;
      const generation = shell.descriptor.generation;
      if (attempt <= closeAttemptsBeforeConfirming) {
        // Unconfirmed. The Shell keeps its entry, its transcript, and its listener, because the
        // simulated process is still there — and taking those away is what removes the user's only
        // way to retry.
        const unconfirmed = options.unconfirmedCloseFails ? "close_failed" : "reaping";
        if (unconfirmed === "close_failed") {
          // Carried on the descriptor as well as in the outcome. A view rebuilding its list from
          // `listSessionShells` has no outcome to read, and without the flag it cannot tell whether
          // offering a retry would be honest.
          shell.descriptor = { ...shell.descriptor, retryable: true };
        }
        setState(shell, unconfirmed);
        return unconfirmedClose(shellId, generation, unconfirmed, attempt);
      }
      setState(shell, "closed");
      shells.delete(shellId);
      listeners.delete(shellId);
      return confirmedClose(shellId, generation, attempt);
    },
  };
}

export const webSessionShellClient: SessionShellService = createWebSessionShellClient();
