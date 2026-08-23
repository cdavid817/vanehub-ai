import { useEffect, useRef, useState } from "react";
import { Terminal as XtermTerminal } from "@xterm/xterm";
import { FitAddon } from "@xterm/addon-fit";
import "@xterm/xterm/css/xterm.css";
import { useTranslation } from "react-i18next";
import { agentService } from "../services/runtime-agent-client";
import type { ShellConnectionState, ShellRuntimeDescriptor } from "../types/session-workspace";
import { createTerminalTheme } from "./terminal-theme";
import { WorkspaceState } from "./workspace-state";
import { workspaceErrorKey, type WorkspaceErrorKey } from "./workspace-error";
import { retainAsyncCleanup } from "../lib/async-cleanup";

/**
 * The xterm surface detaches when the tab is hidden; the shell itself does not.
 *
 * Hiding stops the fit pass, the resize round-trip, and the keystroke path — the view work. The
 * native process, its scrollback, and its working directory are untouched, because a shell the
 * user started is work in progress and glancing at another tab is not a request to end it. Output
 * still lands in the terminal buffer, so nothing arrives missing when the tab comes back.
 */
export function ShellTab({ isVisible, seatId = null, sessionId }: { isVisible: boolean; seatId?: string | null; sessionId: string | null }) {
  const { t } = useTranslation();
  const hostRef = useRef<HTMLDivElement>(null);
  const terminalRef = useRef<XtermTerminal | null>(null);
  const fitRef = useRef<FitAddon | null>(null);
  const shellIdRef = useRef<string | null>(null);
  const runtimeRef = useRef<ShellRuntimeDescriptor | null>(null);
  const visibleRef = useRef(isVisible);
  const [state, setState] = useState<ShellConnectionState>("connecting");
  const [runtime, setRuntime] = useState<ShellRuntimeDescriptor | null>(null);
  const [error, setError] = useState<WorkspaceErrorKey | null>(null);

  useEffect(() => { visibleRef.current = isVisible; }, [isVisible]);

  useEffect(() => {
    if (!sessionId || !hostRef.current) return;
    const targetSessionId = sessionId;
    let disposed = false;
    let unsubscribe: (() => void) | null = null;
    // A rejected write or resize used to be dropped on the floor by `void`, so a shell whose
    // channel had already failed kept accepting keystrokes that went nowhere.
    const reportFailure = (reason: unknown) => {
      if (!disposed) setError(workspaceErrorKey(reason));
    };
    const terminal = new XtermTerminal({
      allowTransparency: false,
      convertEol: true,
      cursorBlink: true,
      fontFamily: "ui-monospace, SFMono-Regular, Menlo, Consolas, monospace",
      fontSize: 13,
      theme: createTerminalTheme(),
    });
    const fit = new FitAddon();
    terminal.loadAddon(fit); terminal.open(hostRef.current); fit.fit();
    terminalRef.current = terminal; fitRef.current = fit;
    const inputDisposable = terminal.onData((content) => {
      // The input binding is inert while the view is detached. A hidden xterm can still be handed
      // synthetic data — a paste replayed into a detached buffer, a focus race during a tab
      // switch — and forwarding that would run a command the user never typed here.
      if (!visibleRef.current) return;
      const shellId = shellIdRef.current;
      if (shellId) agentService.writeShellInput(shellId, content).catch(reportFailure);
    });
    const resizeObserver = new ResizeObserver(() => {
      if (!visibleRef.current) return;
      fit.fit();
      const shellId = shellIdRef.current;
      // A simulated runtime has no PTY to resize; asking it to would be a request the adapter
      // can only reject.
      if (shellId && runtimeRef.current?.kind !== "simulated") {
        agentService.resizeShell({ shellId, rows: terminal.rows, cols: terminal.cols }).catch(reportFailure);
      }
    });
    resizeObserver.observe(hostRef.current);
    const themeObserver = new MutationObserver(() => {
      terminal.options.theme = createTerminalTheme();
    });
    themeObserver.observe(document.documentElement, { attributes: true, attributeFilter: ["data-theme"] });

    let outputBuffer = "";
    let outputFrame = 0;
    const flushOutput = () => {
      outputFrame = 0;
      if (outputBuffer.length === 0) return;
      terminal.write(outputBuffer);
      outputBuffer = "";
    };

    async function connect() {
      try {
        setState("connecting");
        const shell = await agentService.createShell({ sessionId: targetSessionId, rows: terminal.rows, cols: terminal.cols, seatId });
        if (disposed) { await agentService.killShell(shell.shellId); return; }
        shellIdRef.current = shell.shellId; runtimeRef.current = shell.runtime;
        setState(shell.state); setRuntime(shell.runtime);
        if (shell.runtime.kind === "simulated") terminal.writeln(t("sessionTabs.shell.simulatedBanner"));
        const nextUnsubscribe = await agentService.subscribeShellEvents(shell.shellId, (event) => {
          if (disposed) return;
          if (event.type === "output") {
            // PTY backends emit many small chunks under burst output (a build log); writing
            // each synchronously stalls the main thread. Coalesce chunks and write once per
            // animation frame.
            outputBuffer += event.content;
            if (outputFrame === 0) outputFrame = requestAnimationFrame(flushOutput);
          } else {
            flushOutput();
            setState(event.state);
            if (event.state === "disconnected" || event.state === "failed") shellIdRef.current = null;
            if (event.error) setError(workspaceErrorKey(event.error));
          }
        });
        unsubscribe = retainAsyncCleanup(disposed, nextUnsubscribe);
      } catch (reason) {
        if (!disposed) { setState("failed"); setError(workspaceErrorKey(reason)); }
      }
    }
    void connect();
    return () => {
      disposed = true; resizeObserver.disconnect(); themeObserver.disconnect(); inputDisposable.dispose(); unsubscribe?.(); terminal.dispose();
      if (outputFrame !== 0) cancelAnimationFrame(outputFrame);
      const shellId = shellIdRef.current; shellIdRef.current = null; runtimeRef.current = null;
      if (shellId) void agentService.killShell(shellId);
      terminalRef.current = null; fitRef.current = null;
    };
    // A shell belongs to one seat, so selecting another seat opens that seat's shell rather than
    // silently reassigning this one. Task Group 7 replaces this teardown with a retained registry.
  }, [seatId, sessionId, t]);

  useEffect(() => {
    if (!isVisible) return;
    const frame = requestAnimationFrame(() => {
      fitRef.current?.fit();
      const terminal = terminalRef.current; const shellId = shellIdRef.current;
      if (terminal && shellId && runtimeRef.current?.kind !== "simulated") {
        agentService.resizeShell({ shellId, rows: terminal.rows, cols: terminal.cols })
          .catch((reason: unknown) => setError(workspaceErrorKey(reason)));
      }
    });
    return () => cancelAnimationFrame(frame);
  }, [isVisible]);

  if (!sessionId) return <WorkspaceState kind="unavailable" />;
  return (
    <div className="flex h-full min-h-0 flex-col overflow-hidden rounded-lg border border-border bg-[hsl(var(--panel-muted))]">
      <div className="flex items-center gap-2 border-b border-border p-2 text-xs">
        <span className="rounded-full border border-border px-2 py-1">{t(`sessionTabs.shell.state.${state}`)}</span>
        {runtime ? <span className="rounded-full bg-muted px-2 py-1 text-muted-foreground">{t(`sessionTabs.shell.runtime.${runtime.kind}`)}</span> : null}
        <div className="ml-auto flex gap-1">
          <button className="h-7 rounded border border-border px-2 hover:bg-muted" disabled={!shellIdRef.current} onClick={() => { const shellId = shellIdRef.current; if (shellId) agentService.resetShellDirectory(shellId).catch((reason: unknown) => setError(workspaceErrorKey(reason))); }} type="button">{t("sessionTabs.shell.cd")}</button>
          <button className="h-7 rounded border border-border px-2 hover:bg-muted" onClick={() => terminalRef.current?.clear()} type="button">{t("sessionTabs.shell.clear")}</button>
          <button className="h-7 rounded border border-border px-2 hover:bg-muted" disabled={state !== "connected"} onClick={() => { const shellId = shellIdRef.current; if (!shellId) return; shellIdRef.current = null; setState("disconnected"); agentService.killShell(shellId).catch((reason: unknown) => setError(workspaceErrorKey(reason))); }} type="button">{t("sessionTabs.shell.disconnect")}</button>
        </div>
      </div>
      {error ? <div className="p-2"><WorkspaceState kind="error" message={t(error)} /></div> : null}
      <div aria-label={t("sessionTabs.shell.terminal")} className="ucd-shell-terminal min-h-0 flex-1 p-2" ref={hostRef} />
    </div>
  );
}
