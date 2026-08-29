import type {
  CursorPage,
  EvidenceSubscriptionBootstrap,
  ExecutionEvidenceNotice,
  ExecutionRecord,
  ExecutionRecordDetail,
  SessionRunReport,
  SessionRunReportExportResult,
  WorkspaceEvidenceSummary,
} from "../types/session-workspace-evidence";
import type {
  SessionShellDescriptor,
  SessionShellNotice,
  ShellAttachSnapshot,
  ShellOutputFrame,
} from "../types/session-workspace-shell-frames";
import {
  cursorPageSchema,
  evidenceSubscriptionBootstrapSchema,
  executionEvidenceNoticeSchema,
  workspaceEvidenceSummarySchema,
} from "./session-workspace-evidence-core";
import {
  executionRecordDetailSchema,
  executionRecordSchema,
} from "./session-workspace-evidence-records";
import {
  sessionRunReportExportSchema,
  sessionRunReportSchema,
} from "./session-workspace-evidence-report";
import {
  sessionShellDescriptorSchema,
  sessionShellNoticeSchema,
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

export function parseEvidenceSubscriptionBootstrap(value: unknown): EvidenceSubscriptionBootstrap {
  return evidenceSubscriptionBootstrapSchema.parse(value);
}

export function parseSessionRunReport(value: unknown): SessionRunReport {
  return sessionRunReportSchema.parse(value);
}

export function parseSessionRunReportExport(value: unknown): SessionRunReportExportResult {
  return sessionRunReportExportSchema.parse(value);
}

export function parseShellOutputFrame(value: unknown): ShellOutputFrame {
  return shellOutputFrameSchema.parse(value);
}

export function parseSessionShellDescriptor(value: unknown): SessionShellDescriptor {
  const descriptor = sessionShellDescriptorSchema.parse(value);
  return { ...descriptor, runtime: normalizeShellRuntimeDescriptor(descriptor.runtime) };
}

export function parseShellAttachSnapshot(value: unknown): ShellAttachSnapshot {
  const snapshot = shellAttachSnapshotSchema.parse(value);
  return { ...snapshot, descriptor: parseSessionShellDescriptor(snapshot.descriptor) };
}

/**
 * Nullish `reason` and `exitCode` become absent rather than `null`, so a reader tests one thing.
 * The native side omits the field it does not have and Tauri sends `null` for the other, and a view
 * that had to handle both would eventually check only one.
 */
export function parseSessionShellNotice(value: unknown): SessionShellNotice {
  const notice = sessionShellNoticeSchema.parse(value);
  if (notice.type === "output") return notice;
  return {
    ...notice,
    reason: notice.reason ?? undefined,
    exitCode: notice.exitCode ?? undefined,
  };
}
