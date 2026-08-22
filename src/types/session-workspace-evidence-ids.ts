declare const evidenceBrand: unique symbol;

/**
 * An identifier that only the transport schema can produce.
 *
 * The brand carries no runtime cost and cannot be written by hand, so a raw string read out of a
 * URL, a React prop, or another panel's state cannot be passed where a validated id is required.
 * The only construction point is `src/contracts/session-workspace-evidence-ids.ts`; nothing here
 * exports a cast, on purpose.
 */
export type EvidenceBranded<Name extends string> = string & { readonly [evidenceBrand]: Name };

export type EvidenceSessionId = EvidenceBranded<"EvidenceSessionId">;
export type EvidenceSeatId = EvidenceBranded<"EvidenceSeatId">;
export type EvidenceAgentId = EvidenceBranded<"EvidenceAgentId">;
export type EvidenceRunId = EvidenceBranded<"EvidenceRunId">;
export type EvidenceTraceId = EvidenceBranded<"EvidenceTraceId">;
export type EvidenceSpanId = EvidenceBranded<"EvidenceSpanId">;
export type EvidenceOperationId = EvidenceBranded<"EvidenceOperationId">;
export type EvidenceCommandId = EvidenceBranded<"EvidenceCommandId">;
export type EvidenceToolCallId = EvidenceBranded<"EvidenceToolCallId">;
export type EvidenceRecordId = EvidenceBranded<"EvidenceRecordId">;

/**
 * A keyset continuation token. It is deliberately opaque: it encodes a version, a position, and a
 * fingerprint of the filters it was issued for, and the backend rejects it when replayed against a
 * different filter set. Frontend code passes it back unread — decoding it here would recreate the
 * offset arithmetic this change exists to remove.
 */
export type EvidenceCursor = EvidenceBranded<"EvidenceCursor">;
