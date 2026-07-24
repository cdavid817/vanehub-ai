import type { RemoteTerminalClient } from "./remote-terminal-client";
import type { RemoteCommandRun, RemoteCommandTemplate, RemoteOutputSearchQuery, RemoteOutputSearchResult, RemoteTerminalStatus } from "../types/remote-terminal";
import { describe, expect, it } from "vitest";

describe("remote terminal service contract", () => {
  it("keeps the adapter surface explicit", () => {
    const client: RemoteTerminalClient = {
      getStatus: async (sessionId: string): Promise<RemoteTerminalStatus> => { void sessionId; return { state: "disconnected", endpoint: null, binding: null, error: null }; },
      listTemplates: async (scope?: RemoteCommandTemplate["scope"]): Promise<RemoteCommandTemplate[]> => { void scope; return []; },
      insertTemplate: async (templateId: string): Promise<string> => { void templateId; return ""; },
      executeTemplate: async (_templateId: string, sessionId: string): Promise<RemoteCommandRun> => ({ id: "run", templateId: null, sessionId, connectionId: null, commandSnapshot: "", workingDirectory: null, status: "cancelled", exitCode: null, startedAt: "", finishedAt: null }),
      cancelRun: async (runId: string): Promise<RemoteCommandRun | null> => { void runId; return null; },
      searchOutput: async (query: RemoteOutputSearchQuery): Promise<RemoteOutputSearchResult> => { void query; return { hits: [], nextOffset: null }; },
    };
    expect(client).toBeDefined();
  });
});
