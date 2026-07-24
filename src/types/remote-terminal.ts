export type RemoteTerminalState = "connecting" | "connected" | "trust-required" | "authentication-failed" | "stale" | "disconnected";
export type RemoteTerminalOutputSource = "pty" | "quick-command" | "gap";
export type RemoteCommandTemplateScope = "global" | "connection" | "workspace";
export type RemoteCommandRunStatus = "queued" | "running" | "succeeded" | "failed" | "cancelled";

export interface RemoteTerminalEndpoint { host: string; port: number; user: string; path: string; uri: string; }
export interface RemoteTerminalBinding { connectionId: string; revision: number; }
export interface RemoteTerminalStatus { state: RemoteTerminalState; endpoint: RemoteTerminalEndpoint | null; binding: RemoteTerminalBinding | null; error: string | null; }
export interface RemoteHostKeyChallenge { connectionId: string; revision: number; kind: "first-seen" | "changed"; algorithm: string; fingerprint: string; previousFingerprint: string | null; }
export interface RemoteCommandTemplate { id: string; name: string; command: string; scope: RemoteCommandTemplateScope; connectionId: string | null; workspaceUri: string | null; workingDirectory: string | null; tags: string[]; }
export interface RemoteCommandRun { id: string; templateId: string | null; sessionId: string; connectionId: string | null; commandSnapshot: string; workingDirectory: string | null; status: RemoteCommandRunStatus; exitCode: number | null; startedAt: string; finishedAt: string | null; }
export interface RemoteOutputChunk { id: number; streamId: string; sequence: number; sessionId: string; connectionId: string | null; terminalId: string | null; runId: string | null; source: RemoteTerminalOutputSource; content: string; capturedAt: string; }
export interface RemoteOutputSearchQuery { query: string; sessionId?: string; connectionId?: string; terminalId?: string; runId?: string; offset?: number; limit?: number; }
export interface RemoteOutputSearchResult { hits: RemoteOutputChunk[]; nextOffset: number | null; }
