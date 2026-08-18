import type { AgentRunRunner, AgentRunState } from "../types/agent-run";

export type MissionControlSort = "newest" | "oldest" | "attention";
export type MissionControlAttention = "approval" | "user" | "stuck" | "failed" | "review";
export type MissionControlAction = "open" | "cancel" | "resume" | "retry" | "approval" | "review" | "verify";
export type MissionControlFacet = "overview" | "plan" | "timeline" | "tools" | "files" | "review" | "verification" | "context" | "usage" | "logs";
export type MissionControlFacetState = "available" | "unavailable" | "restricted";
export interface MissionControlQuery { cursor?: string | null; limit?: number; states?: AgentRunState[]; agentId?: string; projectId?: string; runner?: "local" | "ssh"; sort?: MissionControlSort }
export interface MissionControlCounts { running: number; waitingApproval: number; waitingUser: number; retrying: number; blocked: number; failed: number; completedRecently: number }
export interface MissionControlNavigationTarget { kind: "session" | "approval" | "review" | "plan" | "loop" | "goal" | "evaluation"; id: string; sessionId?: string | null }
export interface MissionControlRunSummary { runId: string; version: number; ownerType: string; ownerId: string; agentId: string | null; title: string; state: AgentRunState; createdAt: string; updatedAt: string; endedAt: string | null; projectId: string | null; workspace: string | null; phase: string | null; attention: MissionControlAttention | null; reasonCode: string | null; verification: "pending" | "running" | "passed" | "failed" | "unavailable"; tokens: number | null; cost: number | null; actions: MissionControlAction[]; navigation: MissionControlNavigationTarget | null; runner: AgentRunRunner | null }
export interface MissionControlPage { items: MissionControlRunSummary[]; nextCursor: string | null }
export interface MissionControlOverview { counts: MissionControlCounts; attention: MissionControlPage; active: MissionControlPage; recent: MissionControlPage }
export interface MissionControlFacetAvailability { facet: MissionControlFacet; state: MissionControlFacetState }
export interface MissionControlRunDetail { run: MissionControlRunSummary; facets: MissionControlFacetAvailability[] }
export interface MissionControlActionInput { runId: string; version: number; action: Exclude<MissionControlAction, "open" | "approval" | "review"> }
export interface MissionControlActionReceipt { run: MissionControlRunSummary; operationId: string | null }
export type MissionControlErrorCode = "invalid_query" | "invalid_cursor" | "not_found" | "conflict" | "unsupported" | "forbidden" | "storage_unavailable";
export interface MissionControlSafeError { code: MissionControlErrorCode; message: string; field?: string | null }
