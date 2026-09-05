import { useEffect, useRef } from "react";
import { Terminal as XtermTerminal } from "@xterm/xterm";
import { FitAddon } from "@xterm/addon-fit";
import { attachAcceleratedRenderer } from "./terminal-renderer";
import "@xterm/xterm/css/xterm.css";
import { useTranslation } from "react-i18next";
import { sessionShellService } from "../services/runtime-session-shell-client";
import { replayText } from "../services/session-shell-frames";
import type { SessionShellDescriptor } from "../types/session-workspace-shell-frames";
import type { SessionShellEvent } from "../services/session-shell-service";
import { createTerminalTheme } from "./terminal-theme";
import { acceptsInput } from "./shell-status";
import { workspaceErrorKey, type WorkspaceErrorKey } from "./workspace-error";

interface ShellSurfaceProps {
  descriptor: SessionShellDescriptor;
  isVisible: boolean;
  onDescriptor(descriptor: SessionShellDescriptor): void;
  onError(error: WorkspaceErrorKey): void;
}

/**
 * One xterm surface bound to one retained Shell.
 *
 * Hiding this tab, switching sessions, and unmounting all detach. None of them close: a Shell the
 * user started is work in progress, and glancing at another tab is not a request to end a build.
 * Coming back reattaches from the last sequence this view consumed, so what happened while nobody
 * was watching is replayed rather than lost.
 */
export function ShellSurface({ descriptor, isVisible, onDescriptor, onError }: ShellSurfaceProps) {
  const { t } = useTranslation();
  const hostRef = useRef<HTMLDivElement>(null);
  const terminalRef = useRef<XtermTerminal | null>(null);
  const fitRef = useRef<FitAddon | null>(null);
  const attachmentRef = useRef<string | null>(null);
  // The last sequence written to this terminal, so a reattach asks for exactly what it missed.
  const consumedRef = useRef(0);
  // The live descriptor, read by callbacks that must not re-subscribe when it changes. A closure
  // over the rendered value would apply a state notice on top of whatever the descriptor was when
  // the attachment was made, quietly reverting every field that had moved since.
  const descriptorRef = useRef(descriptor);
  const acceptsInputRef = useRef(acceptsInput(descriptor));
  const shellId = descriptor.shellId;

  useEffect(() => {
    descriptorRef.current = descriptor;
    acceptsInputRef.current = acceptsInput(descriptor);
  }, [descriptor]);

  useEffect(() => {
    const host = hostRef.current;
    if (!host) return;
    const terminal = new XtermTerminal({
      allowTransparency: false,
      convertEol: true,
      cursorBlink: true,
      fontFamily: "ui-monospace, SFMono-Regular, Menlo, Consolas, monospace",
      fontSize: 13,
      theme: createTerminalTheme(),
    });
    const fit = new FitAddon();
    terminal.loadAddon(fit);
    terminal.open(host);
    fit.fit();
    terminalRef.current = terminal;
    fitRef.current = fit;
    const themeObserver = new MutationObserver(() => {
      terminal.options.theme = createTerminalTheme();
    });
    themeObserver.observe(document.documentElement, {
      attributes: true,
      attributeFilter: ["data-theme"],
    });
    const inputDisposable = terminal.onData((content) => {
      const attachmentId = attachmentRef.current;
      // Input is refused rather than queued when the Shell has ended: a keystroke held and
      // delivered later would run against whatever the user opened next.
      if (!attachmentId || !acceptsInputRef.current) return;
      sessionShellService
        .writeSessionShell({ shellId, attachmentId, content })
        .catch((reason: unknown) => onError(workspaceErrorKey(reason)));
    });
    const syncSize = () => {
      fit.fit();
      const attachmentId = attachmentRef.current;
      if (!attachmentId) return;
      sessionShellService
        .resizeSessionShell({ shellId, attachmentId, rows: terminal.rows, cols: terminal.cols })
        .catch((reason: unknown) => onError(workspaceErrorKey(reason)));
    };
    const resizeObserver = new ResizeObserver(syncSize);
    // Attached once `syncSize` exists: swapping the renderer can change the computed rows and
    // columns, and the Shell on the other end has already been told the pre-swap size.
    const detachRenderer = attachAcceleratedRenderer(terminal, syncSize);
    resizeObserver.observe(host);
    return () => {
      resizeObserver.disconnect();
      themeObserver.disconnect();
      inputDisposable.dispose();
      detachRenderer();
      terminal.dispose();
      terminalRef.current = null;
      fitRef.current = null;
    };
  }, [onError, shellId]);

  useEffect(() => {
    const terminal = terminalRef.current;
    // Attached only while shown. A hidden surface holds no claim, which is what lets another view
    // take the Shell without waiting for this one to be re-rendered.
    if (!isVisible || !terminal) return;
    let disposed = false;
    let detach: (() => Promise<void>) | null = null;
    const writeGap = (from: number, to: number) => {
      terminal.writeln(t("sessionTabs.shell.replayGap", { from, to }));
    };
    const writeFrame = (notice: SessionShellEvent) => {
      if (disposed) return;
      if (notice.type === "gap") {
        // Live frames jumped. The marker goes in where the jump was, because a transcript that
        // closed up over the missing range would read as complete.
        writeGap(notice.gap.fromSequence, notice.gap.toSequence);
        return;
      }
      if (notice.type === "state") {
        onDescriptor({
          ...descriptorRef.current,
          state: notice.state,
          reason: notice.reason,
          exitCode: notice.exitCode,
          revision: notice.revision,
          lastActivityAt: notice.occurredAt,
        });
        return;
      }
      consumedRef.current = Math.max(consumedRef.current, notice.sequence);
      terminal.write(notice.data);
    };

    void (async () => {
      try {
        const attachment = await sessionShellService.attachSessionShell(
          { shellId, afterSequence: consumedRef.current },
          writeFrame,
        );
        if (disposed) {
          await attachment.detach();
          return;
        }
        attachmentRef.current = attachment.attachmentId;
        // The marker goes in before the replay it precedes, so the scrollback reads in order and a
        // shortened history is never presented as continuous.
        if (attachment.gap) {
          writeGap(attachment.gap.fromSequence, attachment.gap.toSequence);
        }
        terminal.write(replayText(attachment.replay));
        consumedRef.current = Math.max(consumedRef.current, attachment.nextSequence - 1);
        onDescriptor(attachment.descriptor);
        detach = attachment.detach;
        fitRef.current?.fit();
      } catch (reason) {
        if (!disposed) onError(workspaceErrorKey(reason));
      }
    })();

    return () => {
      disposed = true;
      attachmentRef.current = null;
      void detach?.();
    };
    // The descriptor is read through a ref rather than listed here: it changes on every notice, and
    // depending on it would detach and reattach the Shell on its own output.
  }, [isVisible, onDescriptor, onError, shellId, t]);

  return (
    <div
      aria-label={t("sessionTabs.shell.terminal")}
      className="ucd-shell-terminal min-h-0 flex-1 p-2"
      ref={hostRef}
      role="log"
    />
  );
}
