import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import {
  parseSessionShellDescriptor,
  parseSessionShellNotice,
  parseShellAttachSnapshot,
} from "../contracts/session-workspace-evidence";
import type {
  SessionShellDescriptor,
  SessionShellNotice,
  ShellAttachSnapshot,
} from "../types/session-workspace-shell-frames";
import { createShellFrameDispatcher, onceDetach, orderBufferedNotices } from "./session-shell-frames";
import type {
  AttachSessionShellInput,
  CreateSessionShellInput,
  ResizeSessionShellInput,
  SessionShellService,
  ShellAttachment,
  ShellAttachmentScope,
  WriteSessionShellInput,
} from "./session-shell-service";

/**
 * The native event channel. It has to match `SESSION_SHELL_EVENT` in the Rust notice publisher
 * verbatim; a mismatch produces a subscription that never fires and never errors.
 */
export const SESSION_SHELL_EVENT_CHANNEL = "session-shell:notice";

/** The seam the client talks through, so its behaviour is testable without a running app. */
export interface NativeShellTransport {
  invokeShell(command: string, payload: Record<string, unknown>): Promise<unknown>;
  subscribeShellNotices(handler: (payload: unknown) => void): Promise<() => void>;
}

export function createNativeShellTransport(): NativeShellTransport {
  return {
    async invokeShell(command: string, payload: Record<string, unknown>): Promise<unknown> {
      return invoke(command, payload);
    },
    async subscribeShellNotices(handler: (payload: unknown) => void): Promise<() => void> {
      return listen<unknown>(SESSION_SHELL_EVENT_CHANNEL, (event) => handler(event.payload));
    },
  };
}

export function createTauriSessionShellClient(
  transport: NativeShellTransport,
): SessionShellService {
  return {
    async listSessionShells(sessionId: string): Promise<SessionShellDescriptor[]> {
      const shells = await transport.invokeShell("list_session_shells", { sessionId });
      return (Array.isArray(shells) ? shells : []).map(parseSessionShellDescriptor);
    },

    async createSessionShell(input: CreateSessionShellInput): Promise<SessionShellDescriptor> {
      return parseSessionShellDescriptor(
        await transport.invokeShell("create_session_shell", {
          input: {
            sessionId: input.sessionId,
            rows: input.rows,
            cols: input.cols,
            seatId: input.seatId ?? null,
            requestId: input.requestId ?? null,
            title: input.title ?? null,
          },
        }),
      );
    },

    async attachSessionShell(
      input: AttachSessionShellInput,
      listener: (notice: SessionShellNotice) => void,
    ): Promise<ShellAttachment> {
      // Listener first, attach second, buffer in between. Attaching first would lose every frame
      // published in that window, and the sequences the view then sees would be contiguous, so
      // nothing downstream could tell that anything was missing.
      let dispatcher: ReturnType<typeof createShellFrameDispatcher> | null = null;
      const buffered: SessionShellNotice[] = [];
      const unsubscribe = await transport.subscribeShellNotices((payload) => {
        // A malformed notice is dropped rather than thrown: an event handler has no caller to
        // reject to, and one bad frame must not tear down a live attachment.
        const parsed = safeParseNotice(payload);
        if (!parsed) return;
        if (dispatcher) dispatcher.accept(parsed);
        else buffered.push(parsed);
      });

      const snapshot = await attachOrRelease(transport, input, unsubscribe);
      dispatcher = createShellFrameDispatcher({
        shellId: input.shellId,
        fromSequence: snapshot.nextSequence,
        listener,
      });
      for (const notice of orderBufferedNotices(buffered)) dispatcher.accept(notice);

      const scope = { shellId: input.shellId, attachmentId: snapshot.attachmentId };
      return {
        ...snapshot,
        detach: onceDetach(async () => {
          unsubscribe();
          await transport.invokeShell("detach_session_shell", { input: scope });
        }),
      };
    },

    async detachSessionShell(scope: ShellAttachmentScope): Promise<void> {
      await transport.invokeShell("detach_session_shell", { input: scope });
    },

    async writeSessionShell(input: WriteSessionShellInput): Promise<void> {
      await transport.invokeShell("write_session_shell", { input });
    },

    async resizeSessionShell(input: ResizeSessionShellInput): Promise<void> {
      await transport.invokeShell("resize_session_shell", { input });
    },

    async renameSessionShell(input: {
      shellId: string;
      title: string;
    }): Promise<SessionShellDescriptor> {
      return parseSessionShellDescriptor(
        await transport.invokeShell("rename_session_shell", { input }),
      );
    },

    async closeSessionShell(shellId: string): Promise<void> {
      await transport.invokeShell("close_session_shell", { shellId });
    },
  };
}

/**
 * Attaches, releasing the listener if the attach fails.
 *
 * The listener goes up first by design, so a failed attach would otherwise leave a subscription
 * with nothing feeding it and no one holding its teardown.
 */
async function attachOrRelease(
  transport: NativeShellTransport,
  input: AttachSessionShellInput,
  release: () => void,
): Promise<ShellAttachSnapshot> {
  try {
    return parseShellAttachSnapshot(
      await transport.invokeShell("attach_session_shell", {
        input: { shellId: input.shellId, afterSequence: input.afterSequence ?? null },
      }),
    );
  } catch (error) {
    release();
    throw error;
  }
}

function safeParseNotice(payload: unknown): SessionShellNotice | null {
  try {
    return parseSessionShellNotice(payload);
  } catch {
    return null;
  }
}

export const tauriSessionShellClient: SessionShellService = createTauriSessionShellClient(
  createNativeShellTransport(),
);
