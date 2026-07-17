import { useEffect, useMemo, useRef, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { ArrowLeft, Bot, GripHorizontal, Home, LogOut, MessageSquare, Minimize2, Plus, Send, Settings, Sparkles, Square } from "lucide-react";
import { useTranslation } from "react-i18next";
import { MessageItem } from "../components/chat/MessageItem";
import { cn } from "../lib/utils";
import { floatingAssistantService } from "../services/runtime-floating-assistant-client";
import { agentService } from "../services/runtime-agent-client";
import type { FloatingAssistantMainAction, FloatingAssistantSurfaceMode } from "../types/floating-assistant";
import { useSettings } from "../settings/settings-provider";
import { useActiveSessionQuery, useSessionMessageEvents } from "../hooks/use-active-session-chat";
import { resolveFloatingAssistantStatus, type FloatingAssistantStatus } from "./floating-assistant-status";

const statusDotClass: Record<FloatingAssistantStatus, string> = {
  idle: "bg-[hsl(var(--success))]",
  starting: "animate-pulse bg-primary motion-reduce:animate-none",
  running: "bg-[hsl(var(--success))]",
  failed: "bg-[hsl(var(--danger))]",
  stopped: "bg-muted-foreground",
  unavailable: "bg-muted-foreground",
};

function AssistantMark({ compact = false, status }: { compact?: boolean; status: FloatingAssistantStatus }) {
  return (
    <span className={cn("relative flex items-center justify-center rounded-2xl bg-primary text-primary-foreground shadow-lg", compact ? "h-9 w-9" : "h-14 w-14")}>
      <Bot className={compact ? "h-5 w-5" : "h-7 w-7"} aria-hidden="true" />
      <Sparkles className="absolute right-1 top-1 h-3.5 w-3.5" aria-hidden="true" />
      <span className={cn("absolute bottom-0.5 right-0.5 h-3 w-3 rounded-full border-2 border-background", statusDotClass[status])} />
    </span>
  );
}

function MenuAction({ icon: Icon, label, onClick }: { icon: typeof Plus; label: string; onClick: () => void }) {
  return (
    <button className="ucd-interactive flex h-11 w-full items-center gap-3 rounded-lg border border-border px-3 text-left text-sm" onClick={onClick} type="button">
      <Icon className="h-4 w-4 text-primary" aria-hidden="true" />
      <span>{label}</span>
    </button>
  );
}

export function FloatingAssistantApp() {
  const { t } = useTranslation();
  const { reportClientLogEvent } = useSettings();
  const queryClient = useQueryClient();
  const [mode, setMode] = useState<FloatingAssistantSurfaceMode>("collapsed");
  const [draft, setDraft] = useState("");
  const [error, setError] = useState<string | null>(null);
  const messagesEndRef = useRef<HTMLDivElement>(null);
  const activeSessionQuery = useActiveSessionQuery();
  const activeSession = activeSessionQuery.data ?? null;
  const sessionId = activeSession?.id ?? null;
  const messagesKey = useMemo(() => ["floating-messages", sessionId] as const, [sessionId]);
  const messagesQuery = useQuery({
    enabled: Boolean(sessionId) && mode === "chat",
    queryKey: messagesKey,
    queryFn: () => sessionId ? agentService.listMessages({ sessionId, limit: 50 }) : Promise.resolve([]),
  });
  const configQuery = useQuery({
    enabled: Boolean(sessionId),
    queryKey: ["session-chat-config", sessionId],
    queryFn: () => sessionId ? agentService.getSessionChatConfig(sessionId) : Promise.reject(new Error("No session")),
  });
  const messages = messagesQuery.data ?? [];
  const isStreaming = messages.some((message) => message.status === "streaming");
  const assistantStatus = resolveFloatingAssistantStatus(activeSession, messages);
  const statusText = t(`floating.status.${assistantStatus}`);

  function reportFailure(source: string, cause: unknown) {
    const message = cause instanceof Error ? cause.message : String(cause);
    setError(message);
    void reportClientLogEvent({ level: "error", kind: "critical-operation-failure", message, source });
  }

  function changeMode(nextMode: FloatingAssistantSurfaceMode) {
    setError(null);
    setMode(nextMode);
    void floatingAssistantService.setSurfaceMode(nextMode).catch((cause) => reportFailure("FloatingAssistantApp.changeMode", cause));
  }

  function openMain(action: FloatingAssistantMainAction) {
    void floatingAssistantService.showMainWindow(action)
      .then(() => changeMode("collapsed"))
      .catch((cause) => reportFailure("FloatingAssistantApp.openMain", cause));
  }

  const sendMutation = useMutation({
    mutationFn: async () => {
      if (!sessionId || !configQuery.data) throw new Error(t("floating.noSession"));
      return agentService.sendMessage({ sessionId, content: draft.trim(), config: configQuery.data });
    },
    onSuccess: () => {
      setDraft("");
      setError(null);
      void queryClient.invalidateQueries({ queryKey: ["floating-messages", sessionId] });
      void queryClient.invalidateQueries({ queryKey: ["sessions", "active"] });
    },
    onError: (cause) => reportFailure("FloatingAssistantApp.send", cause),
  });
  const stopMutation = useMutation({
    mutationFn: () => sessionId ? agentService.stopGeneration(sessionId) : Promise.resolve(),
    onError: (cause) => reportFailure("FloatingAssistantApp.stop", cause),
  });

  useSessionMessageEvents({
    sessionId,
    queryKey: messagesKey,
    onTerminal: () => {
      void queryClient.invalidateQueries({ queryKey: messagesKey });
      void queryClient.invalidateQueries({ queryKey: ["sessions", "active"] });
    },
  });

  useEffect(() => {
    let active = true;
    let cleanup: (() => void) | undefined;
    void agentService.subscribeSessionEvents((event) => {
      if (event.kind === "active-session-changed") {
        void queryClient.invalidateQueries({ queryKey: ["sessions", "active"] });
      } else {
        void queryClient.invalidateQueries({ queryKey: ["session-chat-config", event.sessionId] });
      }
    }).then((unsubscribe) => {
      if (active) {
        cleanup = unsubscribe;
        // Close the gap between the initial query and asynchronous event subscription.
        void queryClient.invalidateQueries({ queryKey: ["sessions", "active"] });
      } else {
        unsubscribe();
      }
    });
    return () => {
      active = false;
      cleanup?.();
    };
  }, [queryClient]);

  useEffect(() => {
    let active = true;
    let cleanup: (() => void) | undefined;
    void floatingAssistantService.subscribeEvents(() => undefined).then((unsubscribe) => {
      if (active) cleanup = unsubscribe;
      else unsubscribe();
    });
    return () => {
      active = false;
      cleanup?.();
    };
  }, []);

  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [messages]);

  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape" && mode !== "collapsed") changeMode(mode === "chat" ? "menu" : "collapsed");
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [mode]);

  if (mode === "collapsed") {
    return (
      <main className="flex h-screen w-screen items-center justify-center bg-transparent p-1">
        <div className="relative">
          <button className="absolute -top-1 left-1/2 z-10 flex h-4 w-8 -translate-x-1/2 items-center justify-center rounded-full bg-background/80 text-muted-foreground" onPointerDown={() => void floatingAssistantService.startDragging()} title={t("floating.drag")} type="button">
            <GripHorizontal className="h-3 w-3" aria-hidden="true" />
          </button>
          <button aria-label={`${t("floating.openMenu")} · ${statusText}`} className="rounded-2xl transition-transform hover:scale-105 motion-reduce:transform-none motion-reduce:transition-none focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring" onClick={() => changeMode("menu")} type="button">
            <AssistantMark status={assistantStatus} />
          </button>
        </div>
      </main>
    );
  }

  if (mode === "menu") {
    return (
      <main className="h-screen w-screen bg-transparent p-2">
        <section className="ucd-panel flex h-full flex-col rounded-2xl p-3">
          <header className="mb-3 flex items-center gap-2" onPointerDown={() => void floatingAssistantService.startDragging()}>
            <AssistantMark compact status={assistantStatus} />
            <div className="min-w-0 flex-1">
              <h1 className="truncate text-sm font-semibold">{t("floating.title")}</h1>
              <p className="truncate text-xs text-muted-foreground">{activeSession ? `${activeSession.title} · ${statusText}` : statusText}</p>
            </div>
            <button aria-label={t("floating.collapse")} className="flex h-8 w-8 items-center justify-center rounded-md text-muted-foreground hover:bg-muted" onClick={() => changeMode("collapsed")} onPointerDown={(event) => event.stopPropagation()} type="button"><Minimize2 className="h-4 w-4" /></button>
          </header>
          <div className="grid flex-1 content-start gap-2">
            <MenuAction icon={Plus} label={t("floating.newSession")} onClick={() => openMain("new-session")} />
            <MenuAction icon={Home} label={t("floating.currentSession")} onClick={() => openMain("current-session")} />
            <MenuAction icon={MessageSquare} label={t("floating.miniChat")} onClick={() => changeMode("chat")} />
            <MenuAction icon={Settings} label={t("floating.settings")} onClick={() => openMain("settings")} />
          </div>
          <button className="mt-2 flex h-9 items-center justify-center gap-2 rounded-lg text-xs text-muted-foreground hover:bg-muted" onClick={() => void floatingAssistantService.exitApplication()} type="button">
            <LogOut className="h-3.5 w-3.5" aria-hidden="true" />{t("floating.exit")}
          </button>
        </section>
      </main>
    );
  }

  return (
    <main className="h-screen w-screen bg-transparent p-2">
      <section className="ucd-panel flex h-full min-h-0 flex-col rounded-2xl">
        <header className="flex items-center gap-2 border-b border-border p-3" onPointerDown={() => void floatingAssistantService.startDragging()}>
          <button aria-label={t("floating.back")} className="flex h-8 w-8 items-center justify-center rounded-md hover:bg-muted" onClick={() => changeMode("menu")} onPointerDown={(event) => event.stopPropagation()} type="button"><ArrowLeft className="h-4 w-4" /></button>
          <AssistantMark compact status={assistantStatus} />
          <div className="min-w-0 flex-1"><h1 className="truncate text-sm font-semibold">{activeSession?.title ?? t("floating.noSession")}</h1><p className="truncate text-xs text-muted-foreground">{activeSession ? `${statusText} · ${activeSession.agentId} · ${activeSession.interactionMode}` : statusText}</p></div>
          <button aria-label={t("floating.returnMain")} className="flex h-8 w-8 items-center justify-center rounded-md text-muted-foreground hover:bg-muted" onClick={() => openMain("current-session")} onPointerDown={(event) => event.stopPropagation()} type="button"><Home className="h-4 w-4" /></button>
          <button aria-label={t("floating.collapse")} className="flex h-8 w-8 items-center justify-center rounded-md text-muted-foreground hover:bg-muted" onClick={() => changeMode("collapsed")} onPointerDown={(event) => event.stopPropagation()} type="button"><Minimize2 className="h-4 w-4" /></button>
        </header>
        <div className="grid min-h-0 flex-1 content-start gap-3 overflow-y-auto p-3">
          {messages.length ? messages.map((message) => <MessageItem key={message.id} message={message} />) : activeSession ? <div className="flex h-full items-center justify-center text-center text-sm text-muted-foreground">{t("floating.emptyChat")}</div> : <div className="flex h-full flex-col items-center justify-center gap-3 text-center text-sm text-muted-foreground"><p>{t("floating.noSessionHint")}</p><button className="flex h-9 items-center gap-2 rounded-lg bg-primary px-4 text-primary-foreground" onClick={() => openMain("new-session")} type="button"><Plus className="h-4 w-4" aria-hidden="true" />{t("floating.newSession")}</button></div>}
          <div ref={messagesEndRef} />
        </div>
        {error ? <p className="mx-3 mb-2 rounded-md bg-[hsl(var(--danger-soft))] p-2 text-xs text-[hsl(var(--danger))]">{error}</p> : null}
        <div className="border-t border-border p-3">
          <textarea className="ucd-input min-h-16 w-full resize-none rounded-lg px-3 py-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-ring" disabled={!activeSession || sendMutation.isPending || isStreaming} onChange={(event) => setDraft(event.target.value)} onKeyDown={(event) => { if (event.key === "Enter" && !event.shiftKey && !event.nativeEvent.isComposing) { event.preventDefault(); if (draft.trim() && !isStreaming) sendMutation.mutate(); } }} placeholder={t("floating.placeholder")} value={draft} />
          <div className="mt-2 flex items-center justify-between text-xs text-muted-foreground">
            <span>{t("floating.usesSessionConfig")}</span>
            {isStreaming ? <button className="flex h-8 items-center gap-1.5 rounded-md border border-border px-3 text-foreground hover:bg-muted" onClick={() => stopMutation.mutate()} type="button"><Square className="h-3.5 w-3.5" />{t("floating.stop")}</button> : <button className="flex h-8 items-center gap-1.5 rounded-md bg-primary px-3 text-primary-foreground disabled:opacity-40" disabled={!activeSession || !configQuery.data || !draft.trim() || sendMutation.isPending} onClick={() => sendMutation.mutate()} type="button"><Send className="h-3.5 w-3.5" />{t("floating.send")}</button>}
          </div>
        </div>
      </section>
    </main>
  );
}
