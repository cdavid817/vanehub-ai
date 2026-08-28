import type {
  EvidenceRecordId,
  EvidenceSessionId,
  ExecutionEvidenceNotice,
} from "../types/session-workspace-evidence";

/**
 * How long identifier-only notices accumulate before one invalidation is applied.
 *
 * A run that appends two hundred records in a second would otherwise mean two hundred cache
 * invalidations and two hundred refetches of the same page. Fixed rather than adaptive, and
 * exported rather than inlined, so a test drives the same number the workspace does.
 */
export const EVIDENCE_NOTICE_WINDOW_MS = 250;

/**
 * How many distinct records one window will track individually.
 *
 * Past this the buffer stops naming them and invalidates the current session instead. Keeping an
 * unbounded set would trade a bounded refetch for unbounded memory, and the whole point of the
 * bound is that a burst has a ceiling.
 */
export const MAX_TRACKED_NOTICE_RECORDS = 32;

/** The finite set of evidence query families. A notice maps to these and to nothing else. */
export type EvidenceQueryFamily = "summary" | "records" | "record-detail" | "report";

export interface EvidenceInvalidation {
  /**
   * Everything for this session. Set when a gap or an overflow means the buffer can no longer say
   * which rows moved — and cannot honestly claim the ones it did name are all of them.
   */
  broad: boolean;
  families: readonly EvidenceQueryFamily[];
  /** Records whose detail is known to have changed. Identifiers only; never content. */
  recordIds: readonly EvidenceRecordId[];
}

/**
 * Which caches a notice can possibly have invalidated.
 *
 * Enumerated rather than derived so the mapping stays finite: a notice kind added later fails to
 * compile here instead of quietly invalidating nothing.
 */
export function noticeQueryFamilies(
  notice: ExecutionEvidenceNotice,
): readonly EvidenceQueryFamily[] {
  switch (notice.kind) {
    case "record-appended":
      return ["records", "summary"];
    case "record-updated":
      return ["records", "record-detail", "summary"];
    case "summary-changed":
      return ["summary"];
    case "coverage-gap":
      // A gap means rows were dropped without saying which, so no narrower answer is honest.
      return ["records", "record-detail", "summary", "report"];
  }
}

export interface EvidenceInvalidationBuffer {
  /** Returns whether the notice was accepted. A notice for another session never is. */
  accept: (notice: ExecutionEvidenceNotice) => boolean;
  pending: () => boolean;
  /** Takes what has accumulated and resets. Null when there is nothing to invalidate. */
  drain: () => EvidenceInvalidation | null;
}

/**
 * Accumulates identifier-only notices into one invalidation.
 *
 * Nothing that arrives here is displayable — a notice carries ids, a sequence, and counts, and the
 * buffer keeps a subset of the ids. That is deliberate: this is the one path that crosses from the
 * event channel into React, and anything it retained would be text that redaction can no longer be
 * applied to.
 */
export function createEvidenceInvalidationBuffer(
  sessionId: EvidenceSessionId,
): EvidenceInvalidationBuffer {
  let broad = false;
  const families = new Set<EvidenceQueryFamily>();
  const recordIds = new Set<EvidenceRecordId>();

  return {
    accept(notice) {
      // Fail closed on a foreign session. Invalidating this workspace's caches because another
      // session moved would refetch the right keys for the wrong reason, and hide the fact that
      // the subscription is pointed at the wrong place.
      if (notice.sessionId !== sessionId) return false;

      if (notice.kind === "coverage-gap") broad = true;
      for (const family of noticeQueryFamilies(notice)) families.add(family);

      if (notice.recordId !== undefined && !broad) {
        recordIds.add(notice.recordId);
        if (recordIds.size > MAX_TRACKED_NOTICE_RECORDS) {
          // Naming a truncated subset would read as "these and only these changed".
          broad = true;
        }
      }
      if (broad) recordIds.clear();
      return true;
    },

    pending() {
      return broad || families.size > 0;
    },

    drain() {
      if (!broad && families.size === 0) return null;
      const invalidation: EvidenceInvalidation = {
        broad,
        families: [...families],
        recordIds: [...recordIds],
      };
      broad = false;
      families.clear();
      recordIds.clear();
      return invalidation;
    },
  };
}
