import { z } from "zod";
import type {
  EvidenceAgentId,
  EvidenceBranded,
  EvidenceCommandId,
  EvidenceCursor,
  EvidenceOperationId,
  EvidenceRecordId,
  EvidenceRunId,
  EvidenceSeatId,
  EvidenceSessionId,
  EvidenceSpanId,
  EvidenceToolCallId,
  EvidenceTraceId,
} from "../types/session-workspace-evidence-ids";

/**
 * The single place a branded evidence id comes into existence.
 *
 * The assertion is confined to this function on purpose: a brand is a compile-time claim that the
 * value passed validation, so the claim has to be made where the validation happens and nowhere
 * else. Exporting a `brand()` helper, or asserting at a call site, would let an unvalidated string
 * acquire the same type and make every downstream signature meaningless.
 */
function brandedId<Name extends string>(minimum = 1) {
  return z
    .string()
    .min(minimum)
    .transform((value) => value as EvidenceBranded<Name>);
}

export const evidenceSessionIdSchema = brandedId<"EvidenceSessionId">();
export const evidenceSeatIdSchema = brandedId<"EvidenceSeatId">();
export const evidenceAgentIdSchema = brandedId<"EvidenceAgentId">();
export const evidenceRunIdSchema = brandedId<"EvidenceRunId">();
export const evidenceTraceIdSchema = brandedId<"EvidenceTraceId">();
export const evidenceSpanIdSchema = brandedId<"EvidenceSpanId">();
export const evidenceOperationIdSchema = brandedId<"EvidenceOperationId">();
export const evidenceCommandIdSchema = brandedId<"EvidenceCommandId">();
export const evidenceToolCallIdSchema = brandedId<"EvidenceToolCallId">();
export const evidenceRecordIdSchema = brandedId<"EvidenceRecordId">();

/**
 * Validated as a non-empty string and nothing more. The cursor's internals belong to the backend
 * that issued it; parsing its parts here would let the frontend construct one, which is how offset
 * pagination came back the last time.
 */
export const evidenceCursorSchema = brandedId<"EvidenceCursor">();

export type ParsedEvidenceIds = {
  sessionId: EvidenceSessionId;
  seatId: EvidenceSeatId;
  agentId: EvidenceAgentId;
  runId: EvidenceRunId;
  traceId: EvidenceTraceId;
  spanId: EvidenceSpanId;
  operationId: EvidenceOperationId;
  commandId: EvidenceCommandId;
  toolCallId: EvidenceToolCallId;
  recordId: EvidenceRecordId;
  cursor: EvidenceCursor;
};
