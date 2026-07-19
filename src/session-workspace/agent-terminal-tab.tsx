import { useEffect, useRef, useState, type KeyboardEvent } from "react";
import { FitAddon } from "@xterm/addon-fit";
import { Terminal as XtermTerminal } from "@xterm/xterm";
import "@xterm/xterm/css/xterm.css";
import { useQueryClient } from "@tanstack/react-query";
import { Send } from "lucide-react";
import { useTranslation } from "react-i18next";
import { agentService } from "../services/runtime-agent-client";
import type { AgentTerminalState, Session } from "../types/agent";
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

export function AgentTerminalTab({ active, session }: { active: boolean; session: Session | null }) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const sessionId = session?.id ?? null;
  const hostRef = useRef<HTMLDivElement>(null);
  const terminalRef = useRef<XtermTerminal | null>(null);
  const fitRef = useRef<FitAddon | null>(null);
  const terminalIdRef = useRef<string | null>(null);
  const activeRef = useRef(active);
  const [state, setState] = useState<AgentTerminalState>("starting");
  const [simulated, setSimulated] = useState(false);
  const [error, setError] = useState<WorkspaceErrorKey | null>(null);
  const [commandInput, setCommandInput] = useState("");

  useEffect(() => {
    activeRef.current = active;
  }, [active]);

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
    terminal.loadAddon(fit);
    terminal.open(hostRef.current);
    fit.fit();
    terminalRef.current = terminal;
    fitRef.current = fit;

    const inputDisposable = terminal.onData((content) => {
      const terminalId = terminalIdRef.current;
      if (terminalId) void agentService.sendAgentTerminalInput(terminalId, content);
    });
    const resizeObserver = new ResizeObserver(() => {
      if (!activeRef.current) return;
      fit.fit();
      const terminalId = terminalIdRef.current;
      if (terminalId) void agentService.resizeAgentTerminal(terminalId, { rows: terminal.rows, cols: terminal.cols });
    });
    resizeObserver.observe(hostRef.current);
    const themeObserver = new MutationObserver(() => {
      terminal.options.theme = terminalTheme();
    });
    themeObserver.observe(document.documentElement, { attributes: true, attributeFilter: ["data-theme"] });

    async function connect() {
      try {
        setError(null);
        setState("starting");
        unsubscribe = await agentService.subscribeAgentTerminalEvents(targetSessionId, (event) => {
          if (event.type === "output") {
            terminal.write(event.content);
            return;
          }
          if (event.type === "state") {
            terminalIdRef.current = event.terminalId;
            setState(event.state);
            void queryClient.invalidateQueries({ queryKey: ["sessions"] });
            if (event.state === "stopped" || event.state === "failed") terminalIdRef.current = null;
            if (event.error) setError(workspaceErrorKey(event.error));
          }
        });
        if (disposed) {
          unsubscribe();
          return;
        }
        const opened = await agentService.openAgentTerminal(targetSessionId, {
          rows: terminal.rows,
          cols: terminal.cols,
        });
        if (disposed) return;
        terminalIdRef.current = opened.terminalId;
        setState(opened.state);
        setSimulated(opened.capability === "simulated");
        void queryClient.invalidateQueries({ queryKey: ["sessions"] });
        if (opened.capability === "simulated") terminal.writeln(t("sessionTabs.agentTerminal.simulatedBanner"));
      } catch (reason) {
        setState("failed");
        setError(workspaceErrorKey(reason));
        void queryClient.invalidateQueries({ queryKey: ["sessions"] });
      }
    }

    void connect();
    return () => {
      disposed = true;
      resizeObserver.disconnect();
      themeObserver.disconnect();
      inputDisposable.dispose();
      unsubscribe?.();
      terminal.dispose();
      terminalIdRef.current = null;
      terminalRef.current = null;
      fitRef.current = null;
    };
  }, [queryClient, sessionId, t]);

  useEffect(() => {
    if (!active) return;
    const frame = requestAnimationFrame(() => {
      fitRef.current?.fit();
      const terminal = terminalRef.current;
      const terminalId = terminalIdRef.current;
      if (terminal && terminalId) void agentService.resizeAgentTerminal(terminalId, { rows: terminal.rows, cols: terminal.cols });
    });
    return () => cancelAnimationFrame(frame);
  }, [active]);

  if (!session) return <WorkspaceState kind="unavailable" />;
  const canSubmitCommand = Boolean(terminalIdRef.current) && state === "running";

  function submitCommand() {
    const terminalId = terminalIdRef.current;
    const content = commandInput.trimEnd();
    if (!terminalId || !content) return;
    void agentService.sendAgentTerminalInput(terminalId, `${content}\r`);
    setCommandInput("");
    terminalRef.current?.focus();
  }

  function handleComposerKeyDown(event: KeyboardEvent<HTMLTextAreaElement>) {
    if (event.key !== "Enter" || event.shiftKey) return;
    event.preventDefault();
    submitCommand();
  }

  return (
    <div className="flex h-full min-h-0 flex-col overflow-hidden rounded-lg border border-border bg-[hsl(var(--panel-muted))]">
      <div className="flex items-center gap-2 border-b border-border p-2 text-xs">
        <span className="rounded-full border border-border px-2 py-1">{t(`sessionTabs.agentTerminal.state.${state}`)}</span>
        {simulated ? <span className="rounded-full bg-muted px-2 py-1 text-muted-foreground">{t("sessionTabs.agentTerminal.simulated")}</span> : null}
        <span className="min-w-0 truncate text-muted-foreground">{session.agentId}</span>
        <div className="ml-auto flex gap-1">
          <button className="h-7 rounded border border-border px-2 hover:bg-muted" onClick={() => terminalRef.current?.clear()} type="button">{t("sessionTabs.agentTerminal.clear")}</button>
          <button className="h-7 rounded border border-border px-2 hover:bg-muted" disabled={!terminalIdRef.current} onClick={() => { const terminalId = terminalIdRef.current; if (!terminalId) return; terminalIdRef.current = null; setState("stopped"); void agentService.stopAgentTerminal(terminalId); }} type="button">{t("sessionTabs.agentTerminal.stop")}</button>
        </div>
      </div>
      {error ? <div className="p-2"><WorkspaceState kind="error" message={t(error)} /></div> : null}
      <div aria-label={t("sessionTabs.agentTerminal.terminal")} className="min-h-0 flex-1 p-2" ref={hostRef} />
      <form className="shrink-0 border-t border-border bg-background/80 p-2" onSubmit={(event) => { event.preventDefault(); submitCommand(); }}>
        <div className="rounded-lg border border-border bg-[hsl(var(--panel-muted))] p-2 shadow-sm focus-within:border-primary">
          <textarea
          aria-label={t("sessionTabs.agentTerminal.input")}
          className="min-h-20 w-full resize-none border-0 bg-transparent px-2 py-1 text-sm outline-none placeholder:text-muted-foreground disabled:cursor-not-allowed disabled:opacity-60"
          disabled={!canSubmitCommand}
          onKeyDown={handleComposerKeyDown}
          onChange={(event) => setCommandInput(event.target.value)}
          placeholder={t("sessionTabs.agentTerminal.inputPlaceholder")}
          rows={3}
          value={commandInput}
          />
          <div className="mt-2 flex items-center justify-end">
            <button
              className="flex h-8 items-center gap-1 rounded border border-border px-3 text-xs text-primary hover:bg-muted disabled:cursor-not-allowed disabled:opacity-50"
              disabled={!canSubmitCommand || commandInput.trimEnd().length === 0}
              title={t("sessionTabs.agentTerminal.send")}
              type="submit"
            >
              <Send className="h-3.5 w-3.5" />
              {t("sessionTabs.agentTerminal.send")}
            </button>
          </div>
        </div>
      </form>
    </div>
  );
}
