import type { AgentTerminalEvent, AgentTerminalSession, AgentTerminalSize } from "../types/agent";

export interface AgentTerminalService {
  openAgentTerminal(sessionId: string, size: AgentTerminalSize): Promise<AgentTerminalSession>;
  sendAgentTerminalInput(terminalId: string, content: string): Promise<void>;
  resizeAgentTerminal(terminalId: string, size: AgentTerminalSize): Promise<void>;
  stopAgentTerminal(terminalId: string): Promise<boolean>;
  subscribeAgentTerminalEvents(
    sessionId: string,
    handler: (event: AgentTerminalEvent) => void,
  ): Promise<() => void>;
}
