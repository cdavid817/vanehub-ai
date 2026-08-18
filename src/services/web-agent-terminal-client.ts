import type { AgentTerminalEvent, AgentTerminalSession, AgentTerminalSize } from "../types/agent";
import type { AgentTerminalService } from "./agent-terminal-service";
import { findWebSession, updateWebSession } from "./web-session-state";

const webRetainedTerminalTranscriptBytes = 1_000_000;

const terminalSubscribersBySession = new Map<string, Set<(event: AgentTerminalEvent) => void>>();
const terminalsBySession = new Map<string, AgentTerminalSession>();
const terminalTranscriptsBySession = new Map<string, string>();

function appendTerminalTranscript(current: string, content: string) {
  let transcript = `${current}${content}`;
  if (transcript.length <= webRetainedTerminalTranscriptBytes) {
    return transcript;
  }
  transcript = transcript.slice(transcript.length - webRetainedTerminalTranscriptBytes);
  return transcript;
}

function emitTerminalEvent(event: AgentTerminalEvent, recordOutput = true) {
  if (recordOutput && event.type === "output") {
    terminalTranscriptsBySession.set(event.sessionId, appendTerminalTranscript(
      terminalTranscriptsBySession.get(event.sessionId) ?? "",
      event.content,
    ));
  }
  const subscribers = terminalSubscribersBySession.get(event.sessionId);
  subscribers?.forEach((handler) => handler(event));
}

function upsertTerminalSession(session: AgentTerminalSession) {
  terminalsBySession.set(session.sessionId, session);
}

export const webAgentTerminalClient: AgentTerminalService = {
  async openAgentTerminal(sessionId: string, size: AgentTerminalSize) {
    const session = findWebSession(sessionId);
    const existing = terminalsBySession.get(sessionId);
    if (existing?.state === "running") {
      const transcript = terminalTranscriptsBySession.get(sessionId) ?? "";
      if (transcript) {
        setTimeout(() => {
          emitTerminalEvent(
            {
              type: "output",
              terminalId: existing.terminalId,
              sessionId,
              content: transcript,
            },
            false,
          );
        }, 0);
      }
      return existing;
    }
    const runtimeSessionId = session.runtimeSessionId ?? `web-runtime-${session.id}`;
    const terminal: AgentTerminalSession = {
      terminalId: `web-terminal-${session.id}`,
      sessionId: session.id,
      agentId: session.agentId,
      state: "running",
      capability: "simulated",
      size,
      runtimeSessionId,
      retained: true,
    };
    upsertTerminalSession(terminal);
    updateWebSession(sessionId, { lifecycleState: "running", runtimeSessionId });
    setTimeout(() => {
      emitTerminalEvent({
        type: "runtime_session_id",
        terminalId: terminal.terminalId,
        sessionId,
        runtimeSessionId,
      });
    }, 30);
    return terminal;
  },

  async sendAgentTerminalInput(terminalId: string, content: string) {
    const terminal = [...terminalsBySession.values()].find((candidate) => candidate.terminalId === terminalId);
    if (!terminal) {
      throw new Error("Agent terminal is not connected.");
    }
    emitTerminalEvent({
      type: "output",
      terminalId,
      sessionId: terminal.sessionId,
      content,
    });
  },

  async resizeAgentTerminal(terminalId: string, size: AgentTerminalSize) {
    const terminal = [...terminalsBySession.values()].find((candidate) => candidate.terminalId === terminalId);
    if (!terminal) {
      throw new Error("Agent terminal is not connected.");
    }
    upsertTerminalSession({ ...terminal, size });
  },

  async stopAgentTerminal(terminalId: string) {
    const terminal = [...terminalsBySession.values()].find((candidate) => candidate.terminalId === terminalId);
    if (!terminal) return false;
    terminalsBySession.delete(terminal.sessionId);
    terminalTranscriptsBySession.delete(terminal.sessionId);
    updateWebSession(terminal.sessionId, { lifecycleState: "stopped" });
    emitTerminalEvent({
      type: "state",
      terminalId,
      sessionId: terminal.sessionId,
      state: "stopped",
      error: null,
    });
    return true;
  },

  async subscribeAgentTerminalEvents(sessionId, handler) {
    const subscribers = terminalSubscribersBySession.get(sessionId) ?? new Set<(event: AgentTerminalEvent) => void>();
    subscribers.add(handler);
    terminalSubscribersBySession.set(sessionId, subscribers);
    return () => {
      const currentSubscribers = terminalSubscribersBySession.get(sessionId);
      currentSubscribers?.delete(handler);
      if (currentSubscribers?.size === 0) {
        terminalSubscribersBySession.delete(sessionId);
      }
    };
  },
};
