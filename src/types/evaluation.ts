export type EvaluationCategory = "bugfix" | "feature" | "refactor" | "tests" | "code_review" | "tool_use" | "context" | "planning";
export type EvaluationOutcome = "queued" | "running" | "succeeded" | "task_failed" | "agent_failed" | "timed_out" | "stuck" | "cancelled" | "benchmark_error";
export type MetricQuality = "reported" | "estimated" | "unavailable";
export interface EvaluationTask { id: string; version: number; category: EvaluationCategory; prompt: string; timeoutSeconds: number; verifierProfiles: string[] }
export interface EvaluationAgentSnapshot { agentId: string; providerId: string; modelId: string | null; interactionMode: string; configurationFingerprint: string }
export interface EvaluationCheck { checkId: string; passed: boolean; summary: string }
export interface EvaluationMetric { name: string; value: number | null; unit: string; quality: MetricQuality; source: string }
export interface EvaluationTimelineItem { id: string; kind: "lifecycle" | "tool" | "context" | "verification"; label: string; status: string }
export interface EvaluationAttempt { id: string; arenaId: string; canonicalRunId: string; taskId: string; taskVersion: number; agent: EvaluationAgentSnapshot; outcome: EvaluationOutcome; checks: EvaluationCheck[]; judge?: unknown; metrics: EvaluationMetric[]; contextEvidenceManifestId: string | null; artifactIds: string[]; timeline: EvaluationTimelineItem[] }
export interface EvaluationArena { id: string; operationId: string; taskId: string; taskVersion: number; rankingVersion: string; attempts: EvaluationAttempt[] }
/** 18.6: cursor-shaped like `MissionControlPage` (`types/mission-control.ts`) for consistency with
 *  this app's other paginated list surfaces, even though the Rust repository underneath is plain
 *  offset/limit rather than a keyset cursor -- see `list_evaluation_arenas.rs`'s own doc comment. */
export interface EvaluationArenaPage { items: EvaluationArena[]; nextCursor: string | null }
export interface EvaluationArenaQuery { cursor?: string | null; limit?: number }
export interface StartEvaluationInput { taskId: string; taskVersion: number; agentIds: string[] }
export interface EvaluationExport { schemaVersion: number; arena: EvaluationArena }
