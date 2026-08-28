/**
 * The memory half of the personalization wire contract, mirroring
 * `src-tauri/src/commands/personalization/dto.rs`.
 *
 * Split from [personalization.ts] only because one file may not exceed 300 lines; the two halves
 * are one contract and the enums they share live next door.
 */

import type {
  MemoryScopeKind,
  MemorySensitivity,
  MemorySource,
  MemoryStatus,
  MemoryType,
} from "./personalization";

/** `candidate` is refused here: a queue entry is not a memory the list may return. */
export type MemoryQueryStatus = Exclude<MemoryStatus, "candidate">;

export type MemoryScopeFilterKind = "any" | "global" | "workspace";

export interface MemoryQuery {
  text?: string;
  scopeKind?: MemoryScopeFilterKind;
  workspaceKey?: string;
  memoryType?: Exclude<MemoryType, "untyped">;
  status?: MemoryQueryStatus;
  sourceAgentId?: string;
  /** Which Agent may read it, as opposed to which one produced it. A memory recorded by one
   * Agent and readable by another answers differently to each. */
  audienceAgentId?: string;
  /** Opaque, and only ever one a page handed back: a hand-built cursor encodes a sort order the
   * store never promised. */
  cursor?: string;
  limit?: number;
}

/** A list entry. No body -- the detail call exists for the one the user opens. */
export interface MemorySummary {
  id: string;
  name: string;
  description: string;
  memoryType: MemoryType;
  scopeKind: MemoryScopeKind;
  workspaceKey: string | null;
  status: MemoryStatus;
  source: MemorySource;
  sensitivity: MemorySensitivity | "restricted";
  revision: number;
  updatedAt: string;
}

export interface MemoryPage {
  items: MemorySummary[];
  nextCursor: string | null;
  /** Present only when the store can produce it cheaply; a screen must render without it. */
  totalMatched: number | null;
}

export interface MemoryDetail {
  id: string;
  name: string;
  description: string;
  memoryType: MemoryType;
  content: string;
  scopeKind: MemoryScopeKind;
  workspaceKey: string | null;
  /** Null means every Agent; a list names the only ones that may read it. */
  audienceAgentIds: string[] | null;
  status: MemoryStatus;
  source: MemorySource;
  sensitivity: MemorySensitivity;
  revision: number;
  sourceAgentId: string | null;
  /** The session it was recorded in, when one was. An id rather than a title: the title belongs to
   * the session and a copy kept here would name something that has since been renamed. */
  sourceSessionId: string | null;
  createdAt: string;
  updatedAt: string;
}

export interface CreateMemoryInput {
  name: string;
  description: string;
  memoryType: Exclude<MemoryType, "untyped">;
  content: string;
  scopeKind: MemoryScopeKind;
  workspaceKey?: string;
  audienceAgentIds?: string[];
}

export interface UpdateMemoryInput {
  id: string;
  expectedRevision: number;
  name?: string;
  description?: string;
  memoryType?: Exclude<MemoryType, "untyped">;
  content?: string;
  status?: MemoryQueryStatus;
  sensitivity?: MemorySensitivity;
}

export type MemoryCandidateKind = "create" | "update" | "archive";

/** One proposal awaiting a decision. Which fields are present follows `kind`. */
export interface MemoryCandidate {
  id: string;
  kind: MemoryCandidateKind;
  name: string | null;
  description: string | null;
  memoryType: MemoryType | null;
  content: string | null;
  targetId: string | null;
  expectedTargetRevision: number | null;
  source: MemorySource;
  sourceAgentId: string | null;
  sourceSessionId: string | null;
  sourceMessageId: string | null;
  createdAt: string;
}

export type MemoryCandidateAction =
  | "approve"
  | "approve-with-edits"
  | "reject"
  | "mark-sensitive-and-archive"
  | "merge-into";

export interface ReviewCandidateInput {
  candidateId: string;
  action: MemoryCandidateAction;
  name?: string;
  description?: string;
  content?: string;
  memoryType?: Exclude<MemoryType, "untyped">;
  /** Absent keeps the proposed scope. Choosing global for an edit that only reworded the
   * text would widen a workspace memory to every project. */
  scopeKind?: MemoryScopeKind;
  workspaceKey?: string;
  /** Absent keeps the proposed audience; an empty list means no Agent may read it. */
  audienceAgentIds?: string[];
  /** Required by `merge-into`, together with the target's revision. */
  mergeTargetId?: string;
  mergeExpectedRevision?: number;
}

export interface ReviewOutcome {
  candidateId: string;
  status: "pending" | "approved" | "rejected";
  resultingMemoryId: string | null;
  retainedContent: boolean;
}

export interface ResetScope {
  scopeKind?: MemoryScopeFilterKind;
  workspaceKey?: string;
  includeArchived: boolean;
}

export interface ResetPreview {
  /** Quoted back on execute: it names the scope and statuses this preview counted. */
  confirmationToken: string;
  matched: number;
  global: number;
  workspace: number;
  candidates: number;
  /** Files the store could not parse. A reset removes them too, so a preview that omitted them
   * would understate what the user is about to lose. */
  malformed: number;
}

export type MaintenancePhase =
  | "authoritative-file"
  | "sqlite-projection"
  | "derived-index"
  | "retrieval-index"
  | "quarantine"
  | "unclassifiable-entry";

export interface MaintenanceResult {
  matched: number;
  deletedFiles: number;
  removedProjectionRows: number;
  revokedRetrievalEntries: number;
  quarantined: number;
  /** A partial result must say so: a caller told a reset succeeded while a projection row survived
   * would leave a memory recallable that the user believes is gone. */
  failures: MaintenancePhase[];
}

/** The typed phrase a reset requires, matched exactly. */
export const RESET_CONFIRMATION_PHRASE = "DELETE";
