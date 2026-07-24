import type { RemoteCommandRun, RemoteCommandTemplate, RemoteOutputSearchQuery, RemoteOutputSearchResult, RemoteTerminalStatus } from "../types/remote-terminal";

export interface RemoteTerminalClient {
  getStatus(sessionId: string): Promise<RemoteTerminalStatus>;
  listTemplates(scope?: RemoteCommandTemplate["scope"]): Promise<RemoteCommandTemplate[]>;
  insertTemplate(templateId: string): Promise<string>;
  executeTemplate(templateId: string, sessionId: string): Promise<RemoteCommandRun>;
  cancelRun(runId: string): Promise<RemoteCommandRun | null>;
  searchOutput(query: RemoteOutputSearchQuery): Promise<RemoteOutputSearchResult>;
}
