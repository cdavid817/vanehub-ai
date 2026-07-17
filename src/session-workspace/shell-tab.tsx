import { useEffect, useRef, useState } from "react";
import { Terminal as XtermTerminal } from "@xterm/xterm";
import { FitAddon } from "@xterm/addon-fit";
import "@xterm/xterm/css/xterm.css";
import { useTranslation } from "react-i18next";
import { agentService } from "../services/runtime-agent-client";
import type { ShellConnectionState } from "../types/session-workspace";
import { WorkspaceState } from "./workspace-state";
import { workspaceErrorKey, type WorkspaceErrorKey } from "./workspace-error";

function semanticColor(name: string, fallback: string) {
  const value = getComputedStyle(document.documentElement).getPropertyValue(name).trim();
  return value ? `hsl(${value})` : fallback;
}

function terminalTheme() {
  return {
    background: semanticColor("--panel-muted", "#111827"),
    foreground: semanticColor("--foreground", "#f3f4f6"),
    cursor: semanticColor("--primary", "#60a5fa"),
    selectionBackground: semanticColor("--accent", "#334155"),
  };
}

export function ShellTab({ active, sessionId }: { active: boolean; sessionId: string | null }) {
  const { t } = useTranslation();
  const hostRef = useRef<HTMLDivElement>(null);
  const terminalRef = useRef<XtermTerminal | null>(null);
  const fitRef = useRef<FitAddon | null>(null);
  const shellIdRef = useRef<string | null>(null);
  const activeRef = useRef(active);
  const [state, setState] = useState<ShellConnectionState>("connecting");
  const [simulated, setSimulated] = useState(false);
  const [error, setError] = useState<WorkspaceErrorKey | null>(null);

  useEffect(() => { activeRef.current = active; }, [active]);

  useEffect(() => {
    if (!sessionId || !hostRef.current) return;
    const targetSessionId = sessionId;
    let disposed = false;
    let unsubscribe: (() => void) | null = null;
    const terminal = new XtermTerminal({
      convertEol: true,
      cursorBlink: true,
      fontFamily: "ui-monospace, SFMono-Regular, Menlo, Consolas, monospace",
      fontSize: 13,
      theme: terminalTheme(),
    });
    const fit = new FitAddon();
    terminal.loadAddon(fit); terminal.open(hostRef.current); fit.fit();
    terminalRef.current = terminal; fitRef.current = fit;
    const inputDisposable = terminal.onData((content) => {
      const shellId = shellIdRef.current;
      if (shellId) void agentService.writeShellInput(shellId, content);
    });
    const resizeObserver = new ResizeObserver(() => {
      if (!activeRef.current) return;
      fit.fit();
      const shellId = shellIdRef.current;
      if (shellId) void agentService.resizeShell({ shellId, rows: terminal.rows, cols: terminal.cols });
    });
    resizeObserver.observe(hostRef.current);
    const themeObserver = new MutationObserver(() => {
      terminal.options.theme = terminalTheme();
    });
    themeObserver.observe(document.documentElement, { attributes: true, attributeFilter: ["data-theme"] });

    async function connect() {
      try {
        setState("connecting");
        const shell = await agentService.createShell({ sessionId: targetSessionId, rows: terminal.rows, cols: terminal.cols });
        if (disposed) { await agentService.killShell(shell.shellId); return; }
        shellIdRef.current = shell.shellId; setState(shell.state); setSimulated(shell.capability === "simulated");
        if (shell.capability === "simulated") terminal.writeln(t("sessionTabs.shell.simulatedBanner"));
        unsubscribe = await agentService.subscribeShellEvents(shell.shellId, (event) => {
          if (event.type === "output") terminal.write(event.content);
          else {
            setState(event.state);
            if (event.state === "disconnected" || event.state === "failed") shellIdRef.current = null;
            if (event.error) setError(workspaceErrorKey(event.error));
          }
        });
      } catch (reason) { setState("failed"); setError(workspaceErrorKey(reason)); }
    }
    void connect();
    return () => {
      disposed = true; resizeObserver.disconnect(); themeObserver.disconnect(); inputDisposable.dispose(); unsubscribe?.(); terminal.dispose();
      const shellId = shellIdRef.current; shellIdRef.current = null;
      if (shellId) void agentService.killShell(shellId);
      terminalRef.current = null; fitRef.current = null;
    };
  }, [sessionId]);

  useEffect(() => {
    if (!active) return;
    const frame = requestAnimationFrame(() => {
      fitRef.current?.fit();
      const terminal = terminalRef.current; const shellId = shellIdRef.current;
      if (terminal && shellId) void agentService.resizeShell({ shellId, rows: terminal.rows, cols: terminal.cols });
    });
    return () => cancelAnimationFrame(frame);
  }, [active]);

  if (!sessionId) return <WorkspaceState kind="unavailable" />;
  return (
    <div className="flex h-full min-h-0 flex-col overflow-hidden rounded-lg border border-border bg-[hsl(var(--panel-muted))]">
      <div className="flex items-center gap-2 border-b border-border p-2 text-xs">
        <span className="rounded-full border border-border px-2 py-1">{t(`sessionTabs.shell.state.${state}`)}</span>
        {simulated ? <span className="rounded-full bg-muted px-2 py-1 text-muted-foreground">{t("sessionTabs.shell.simulated")}</span> : null}
        <div className="ml-auto flex gap-1">
          <button className="h-7 rounded border border-border px-2 hover:bg-muted" disabled={!shellIdRef.current} onClick={() => { const shellId = shellIdRef.current; if (shellId) void agentService.resetShellDirectory(shellId); }} type="button">{t("sessionTabs.shell.cd")}</button>
          <button className="h-7 rounded border border-border px-2 hover:bg-muted" onClick={() => terminalRef.current?.clear()} type="button">{t("sessionTabs.shell.clear")}</button>
          <button className="h-7 rounded border border-border px-2 hover:bg-muted" disabled={state !== "connected"} onClick={() => { const shellId = shellIdRef.current; if (!shellId) return; shellIdRef.current = null; setState("disconnected"); void agentService.killShell(shellId); }} type="button">{t("sessionTabs.shell.disconnect")}</button>
        </div>
      </div>
      {error ? <div className="p-2"><WorkspaceState kind="error" message={t(error)} /></div> : null}
      <div aria-label={t("sessionTabs.shell.terminal")} className="min-h-0 flex-1 p-2" ref={hostRef} />
    </div>
  );
}
