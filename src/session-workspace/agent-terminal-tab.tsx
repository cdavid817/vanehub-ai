import { useCallback, useEffect, useRef, useState } from "react";
import { FitAddon } from "@xterm/addon-fit";
import { Terminal as XtermTerminal } from "@xterm/xterm";
import "@xterm/xterm/css/xterm.css";
import { useQueryClient } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { agentService } from "../services/runtime-agent-client";
import type { AgentTerminalState, Session } from "../types/agent";
import { AgentTerminalComposer } from "./agent-terminal-composer";
import { createTerminalTheme, terminalFontFamily } from "./terminal-theme";
import { TerminalReplayStore } from "../lib/terminal-replay-store";
import { WorkspaceState } from "./workspace-state";
import { workspaceErrorKey, type WorkspaceErrorKey } from "./workspace-error";

const retainedTerminalReplayBytes = 1024 * 1024;
const retainedTerminalReplayTotalBytes = 8 * retainedTerminalReplayBytes;
const terminalReplay = new TerminalReplayStore(
  retainedTerminalReplayBytes,
  retainedTerminalReplayTotalBytes,
);
export { agentTerminalInputClassName } from "./agent-terminal-composer";

function readReplay(sessionId: string) {
  return terminalReplay.read(sessionId);
}

function appendReplay(sessionId: string, content: string) {
  terminalReplay.append(sessionId, content);
}

function clearReplay(sessionId: string) {
  terminalReplay.clear(sessionId);
}

export function AgentTerminalTab({ isVisible, session, sessionActivationKey }: { isVisible: boolean; session: Session | null; sessionActivationKey: number }) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const sessionId = session?.id ?? null;
  const hostRef = useRef<HTMLDivElement>(null);
  const terminalRef = useRef<XtermTerminal | null>(null);
  const fitRef = useRef<FitAddon | null>(null);
  const terminalIdRef = useRef<string | null>(null);
  const visibleRef = useRef(isVisible);
  const activationKeyRef = useRef(sessionActivationKey);
  // `t` is rebound by react-i18next after every `changeLanguage`, which the settings provider
  // calls on every save (theme, font size, CLI palette included). Holding it in a ref keeps it
  // out of the terminal effect's dependencies, so a settings change never tears the terminal
  // down and reconnects it; the banner only needs whatever `t` is current when it is written.
  const tRef = useRef(t);
  tRef.current = t;
  const [state, setState] = useState<AgentTerminalState>("starting");
  // Mirrored into state so controls that depend on "is a terminal attached" re-render when the
  // id arrives through an event that changes nothing else; the ref stays for callbacks.
  const [terminalId, setTerminalId] = useState<string | null>(null);
  const assignTerminalId = useCallback((id: string | null) => {
    terminalIdRef.current = id;
    setTerminalId(id);
  }, []);
  const [connectNonce, setConnectNonce] = useState(0);
  const [simulated, setSimulated] = useState(false);
  const [error, setError] = useState<WorkspaceErrorKey | null>(null);

  useEffect(() => {
    visibleRef.current = isVisible;
  }, [isVisible]);

  useEffect(() => {
    if (!sessionId || !hostRef.current) return;
    const targetSessionId = sessionId;
    const host = hostRef.current;
    let disposed = false;
    let unsubscribe: (() => void) | null = null;
    // Read from the host, not the root: the CLI terminal palette is scoped to this container so
    // the ordinary Shell keeps its own colors.
    const terminal = new XtermTerminal({
      allowTransparency: false,
      convertEol: true,
      cursorBlink: true,
      fontFamily: terminalFontFamily,
      fontSize: 13,
      theme: createTerminalTheme(host),
    });
    const fit = new FitAddon();
    terminal.loadAddon(fit);
    terminal.open(hostRef.current);
    fit.fit();
    terminalRef.current = terminal;
    fitRef.current = fit;
    const cachedReplay = readReplay(targetSessionId);
    if (cachedReplay) terminal.write(cachedReplay);
    let pendingServerReplayPrefix = cachedReplay;
    let outputBuffer = "";
    let outputFrame = 0;
    const flushOutput = () => {
      outputFrame = 0;
      if (!outputBuffer) return;
      terminal.write(outputBuffer);
      outputBuffer = "";
    };
    const queueOutput = (content: string) => {
      if (!content) return;
      appendReplay(targetSessionId, content);
      outputBuffer += content;
      if (outputFrame === 0) outputFrame = requestAnimationFrame(flushOutput);
    };

    const inputDisposable = terminal.onData((content) => {
      const terminalId = terminalIdRef.current;
      if (terminalId) agentService.sendAgentTerminalInput(terminalId, content).catch((reason: unknown) => setError(workspaceErrorKey(reason)));
    });
    // Resize notifications are coalesced to one per frame and sent only when the grid actually
    // changed: dragging a window edge fires the observer continuously, and every PTY resize makes
    // a full-screen TUI repaint.
    let resizeFrame = 0;
    let lastSize = { rows: terminal.rows, cols: terminal.cols };
    const syncSize = () => {
      resizeFrame = 0;
      if (!visibleRef.current) return;
      fit.fit();
      const terminalId = terminalIdRef.current;
      if (!terminalId || (terminal.rows === lastSize.rows && terminal.cols === lastSize.cols)) return;
      lastSize = { rows: terminal.rows, cols: terminal.cols };
      agentService.resizeAgentTerminal(terminalId, lastSize).catch(() => undefined);
    };
    const resizeObserver = new ResizeObserver(() => {
      if (resizeFrame === 0) resizeFrame = requestAnimationFrame(syncSize);
    });
    resizeObserver.observe(host);
    // The palette is applied by the settings provider as a root attribute; only the CLI palette
    // attribute matters here, the application theme never changes what xterm paints. Reassigning
    // the theme object is the whole update: xterm repaints in place, so neither setting is a
    // dependency of this effect and a color change can never recreate, reconnect, or clear it.
    const themeObserver = new MutationObserver(() => {
      terminal.options.theme = createTerminalTheme(host);
    });
    themeObserver.observe(document.documentElement, { attributes: true, attributeFilter: ["data-cli-terminal-theme"] });

    async function connect() {
      try {
        setError(null);
        setState("starting");
        unsubscribe = await agentService.subscribeAgentTerminalEvents(targetSessionId, (event) => {
          if (event.type === "output") {
            if (pendingServerReplayPrefix) {
              const prefix = pendingServerReplayPrefix;
              pendingServerReplayPrefix = "";
              if (event.content === prefix) return;
              if (event.content.startsWith(prefix)) {
                const suffix = event.content.slice(prefix.length);
                queueOutput(suffix);
                return;
              }
            }
            queueOutput(event.content);
            return;
          }
          if (event.type === "runtime_session_id") {
            assignTerminalId(event.terminalId);
            void queryClient.invalidateQueries({ queryKey: ["sessions"] });
            return;
          }
          if (event.type === "state") {
            flushOutput();
            assignTerminalId(event.terminalId);
            setState(event.state);
            void queryClient.invalidateQueries({ queryKey: ["sessions"] });
            if (event.state === "stopped" || event.state === "failed") {
              assignTerminalId(null);
              clearReplay(targetSessionId);
              void queryClient.invalidateQueries({ queryKey: ["session-usage-summary", targetSessionId] });
            }
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
        assignTerminalId(opened.terminalId);
        setState(opened.state);
        setSimulated(opened.capability === "simulated");
        void queryClient.invalidateQueries({ queryKey: ["sessions"] });
        if (opened.capability === "simulated") terminal.writeln(tRef.current("sessionTabs.agentTerminal.simulatedBanner"));
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
      if (outputFrame !== 0) cancelAnimationFrame(outputFrame);
      if (resizeFrame !== 0) cancelAnimationFrame(resizeFrame);
      terminal.dispose();
      assignTerminalId(null);
      terminalRef.current = null;
      fitRef.current = null;
    };
  }, [assignTerminalId, connectNonce, queryClient, sessionId]);

  useEffect(() => {
    if (activationKeyRef.current === sessionActivationKey) return;
    activationKeyRef.current = sessionActivationKey;
    if (!isVisible || !sessionId || terminalIdRef.current || (state !== "stopped" && state !== "failed")) return;
    setConnectNonce((value) => value + 1);
  }, [isVisible, sessionActivationKey, sessionId, state]);

  useEffect(() => {
    if (!isVisible) return;
    const frame = requestAnimationFrame(() => {
      fitRef.current?.fit();
      const terminal = terminalRef.current;
      const terminalId = terminalIdRef.current;
      if (terminal && terminalId) agentService.resizeAgentTerminal(terminalId, { rows: terminal.rows, cols: terminal.cols }).catch(() => undefined);
    });
    return () => cancelAnimationFrame(frame);
  }, [isVisible]);

  if (!session) return <WorkspaceState kind="unavailable" />;
  const canSubmitCommand = terminalId !== null && state === "running";

  function submitCommand(content: string) {
    if (!terminalId) return;
    agentService.sendAgentTerminalInput(terminalId, `${content}\r`).catch((reason: unknown) => setError(workspaceErrorKey(reason)));
    terminalRef.current?.focus();
  }

  return (
    <div className="flex h-full min-h-0 flex-col overflow-hidden bg-[hsl(var(--panel-muted))]">
      <div className="flex items-center gap-2 border-b border-border p-2 text-xs">
        <span className="rounded-full border border-border px-2 py-1">{t(`sessionTabs.agentTerminal.state.${state}`)}</span>
        {simulated ? <span className="rounded-full bg-muted px-2 py-1 text-muted-foreground">{t("sessionTabs.agentTerminal.simulated")}</span> : null}
        <span className="min-w-0 truncate text-muted-foreground">{session.agentId}</span>
        <div className="ml-auto flex gap-1">
          <button className="h-7 rounded border border-border px-2 hover:bg-muted" onClick={() => { if (sessionId) clearReplay(sessionId); terminalRef.current?.clear(); }} type="button">{t("sessionTabs.agentTerminal.clear")}</button>
          <button className="h-7 rounded border border-border px-2 hover:bg-muted" disabled={!terminalId} onClick={() => { if (!terminalId || !sessionId) return; assignTerminalId(null); clearReplay(sessionId); setState("stopped"); agentService.stopAgentTerminal(terminalId).catch((reason: unknown) => setError(workspaceErrorKey(reason))); }} type="button">{t("sessionTabs.agentTerminal.stop")}</button>
        </div>
      </div>
      {error ? <div className="p-2"><WorkspaceState kind="error" message={t(error)} /></div> : null}
      <div aria-label={t("sessionTabs.agentTerminal.terminal")} className="ucd-agent-terminal min-h-0 flex-1" ref={hostRef} />
      <AgentTerminalComposer canSubmit={canSubmitCommand} onSubmit={submitCommand} />
    </div>
  );
}
