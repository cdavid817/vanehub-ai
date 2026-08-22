import type {
  CursorPage,
  ExecutionEvidenceNotice,
  ExecutionRecord,
  ExecutionRecordDetail,
  SessionRunReport,
  WorkspaceEvidenceSummary,
} from "../types/session-workspace-evidence";
import type { ShellAttachSnapshot, ShellOutputFrame } from "../types/session-workspace-shell-frames";
import {
  cursorPageSchema,
  executionEvidenceNoticeSchema,
  workspaceEvidenceSummarySchema,
} from "./session-workspace-evidence-core";
import {
  executionRecordDetailSchema,
  executionRecordSchema,
} from "./session-workspace-evidence-records";
import { sessionRunReportSchema } from "./session-workspace-evidence-report";
import {
  shellAttachSnapshotSchema,
  shellOutputFrameSchema,
} from "./session-workspace-shell-frames";
import { normalizeShellRuntimeDescriptor } from "./session-workspace";

export * from "./session-workspace-evidence-core";
export * from "./session-workspace-evidence-ids";
export * from "./session-workspace-evidence-records";
export * from "./session-workspace-evidence-report";

const executionRecordPageSchema = cursorPageSchema(executionRecordSchema);

/**
 * The transport boundary. Every value that crosses it is parsed here, so a branded id downstream is
 * evidence that validation happened rather than a promise that it will.
 *
 * The declared return types are the enforcement: if a schema drifts from the DTO it claims to
 * produce, this file stops compiling instead of the drift surfacing as a runtime shape mismatch in
 * a panel.
 */
export function parseWorkspaceEvidenceSummary(value: unknown): WorkspaceEvidenceSummary {
  return workspaceEvidenceSummarySchema.parse(value);
}

export function parseExecutionRecord(value: unknown): ExecutionRecord {
  return executionRecordSchema.parse(value);
}

export function parseExecutionRecordPage(value: unknown): CursorPage<ExecutionRecord> {
  return executionRecordPageSchema.parse(value);
}

export function parseExecutionRecordDetail(value: unknown): ExecutionRecordDetail {
  return executionRecordDetailSchema.parse(value);
}

export function parseExecutionEvidenceNotice(value: unknown): ExecutionEvidenceNotice {
  return executionEvidenceNoticeSchema.parse(value);
}

export function parseSessionRunReport(value: unknown): SessionRunReport {
  return sessionRunReportSchema.parse(value);
}

export function parseShellOutputFrame(value: unknown): ShellOutputFrame {
  return shellOutputFrameSchema.parse(value);
}

export function parseShellAttachSnapshot(value: unknown): ShellAttachSnapshot {
  const snapshot = shellAttachSnapshotSchema.parse(value);
  return {
    ...snapshot,
    descriptor: {
      ...snapshot.descriptor,
      runtime: normalizeShellRuntimeDescriptor(snapshot.descriptor.runtime),
    },
  };
}
