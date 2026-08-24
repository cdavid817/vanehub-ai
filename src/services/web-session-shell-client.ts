import type {
  SessionShellDescriptor,
  SessionShellNotice,
  ShellOutputFrame,
  ShellReplayGap,
} from "../types/session-workspace-shell-frames";
import { SHELL_RETAINED_OUTPUT_BYTES } from "../types/session-workspace-shell-frames";
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

/**
 * How much retained output the mock keeps per Shell, in characters.
 *
 * Far below the native byte bound on purpose: a browser demo that had to produce a megabyte of
 * output before a gap appeared would never show one, and the gap marker is behaviour the mock
 * exists to exercise. The native bound is re-exported alongside it so a view that explains an
 * eviction reads one number rather than inventing its own.
 */
export const MOCK_SHELL_RETAINED_CHARACTERS = 4096;

export { SHELL_RETAINED_OUTPUT_BYTES };

interface MockShell {
  descriptor: SessionShellDescriptor;
  frames: ShellOutputFrame[];
  gap: ShellReplayGap | null;
  attachmentId: string | null;
  requestId: string | null;
  nextSequence: number;
}

/**
 * A deterministic multi-Shell mock.
 *
 * Deterministic means no clock and no randomness: ids count up, timestamps advance by a fixed step,
 * and the same sequence of calls produces the same transcript on every run. A mock that reached for
 * `Date.now()` would make its own tests flaky and would let a component that depends on ordering
 * pass here and fail on the desktop.
 */
export function createWebSessionShellClient(): SessionShellService {
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
    evict(shell);
    publish(shell, { type: "output", sessionId: shell.descriptor.sessionId, ...frame });
  }

  /** Drops whole frames from the oldest end and records one contiguous gap, as the native buffer
   * does — a reader that saw two gaps could not tell which side of which it was looking at. */
  function evict(shell: MockShell): void {
    let retained = shell.frames.reduce((total, frame) => total + frame.data.length, 0);
    while (retained > MOCK_SHELL_RETAINED_CHARACTERS && shell.frames.length > 1) {
      const dropped = shell.frames.shift();
      if (!dropped) break;
      retained -= dropped.data.length;
      shell.gap = {
        fromSequence: shell.gap?.fromSequence ?? dropped.sequence,
        toSequence: dropped.sequence,
        reason: "shell_replay_evicted",
      };
    }
  }

  function require_(shellId: string): MockShell {
    const shell = shells.get(shellId);
    if (!shell) throw new Error("shell_not_found");
    return shell;
  }

  function authorize(scope: ShellAttachmentScope): MockShell {
    const shell = require_(scope.shellId);
    if (shell.attachmentId !== scope.attachmentId) throw new Error("shell_attachment_stale");
    if (shell.descriptor.state !== "starting" && shell.descriptor.state !== "running") {
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

      const shellId = nextId("shell");
      const createdAt = timestampAt(counter);
      const shell: MockShell = {
        descriptor: {
          shellId,
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
          state: "running",
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
      };
      shells.set(shellId, shell);
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

    async closeSessionShell(shellId: string): Promise<void> {
      const shell = shells.get(shellId);
      // Closing a Shell that is already gone is a success, so a retry after a partial failure has
      // the same result as the first attempt.
      if (!shell) return;
      setState(shell, "closed");
      shells.delete(shellId);
      listeners.delete(shellId);
    },
  };
}

export const webSessionShellClient: SessionShellService = createWebSessionShellClient();
