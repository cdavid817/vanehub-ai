import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { AgentService } from "./agent-service";
import type {
  DirectoryListing,
  DocumentListing,
  FileContent,
  FileSearchListing,
  GitDiffResult,
  GitStatusResult,
  SessionLogEntry,
  SessionLogExportResult,
  SessionLogPage,
  WorkspaceInspectionCapabilities,
} from "../types/session-workspace";
import type { FolderOpenerAvailability, FolderOpenerPreferences, OpenSessionFolderResult } from "../types/folder-opener";
import { normalizeFolderOpeners, normalizeFolderOpenerPreferences } from "../contracts/folder-opener";
import {
  parseFileEvidenceLinks,
  parseWorkspaceContentSearchResult,
  parseWorkspacePathSearchResult,
  safeParseWorkspaceInvalidationNotice,
} from "../contracts/session-workspace-inspection";

/**
 * The native event channel. It has to match `WORKSPACE_INVALIDATION_EVENT` in the Rust publisher
 * verbatim; a mismatch produces a subscription that never fires and never errors, which on screen
 * is a workspace that simply never changes.
 */
export const WORKSPACE_INVALIDATION_EVENT_CHANNEL = "workspace-invalidation:notice";

type SessionWorkspaceMethods = Pick<
  AgentService,
  | "getWorkspaceInspectionCapabilities"
  | "subscribeWorkspaceInvalidation"
  | "searchWorkspacePaths"
  | "getFileEvidenceLinks"
  | "searchWorkspaceContent"
  | "cancelWorkspaceSearch"
  | "listSessionDirectory"
  | "readSessionFile"
  | "listSessionDocuments"
  | "searchSessionFiles"
  | "getSessionGitStatus"
  | "getSessionGitDiff"
  | "listSessionLogs"
  | "getSessionLogRecord"
  | "exportSessionLogs"
  | "listFolderOpeners"
  | "refreshFolderOpeners"
  | "getFolderOpenerPreferences"
  | "saveFolderOpenerPreferences"
  | "openSessionFolder"
  | "subscribeFolderOpenerEvents"
>;

export const tauriSessionWorkspaceClient: SessionWorkspaceMethods = {
  async listFolderOpeners() {
    return normalizeFolderOpeners(await invoke<FolderOpenerAvailability[]>("list_folder_openers"));
  },
  async refreshFolderOpeners() {
    return normalizeFolderOpeners(await invoke<FolderOpenerAvailability[]>("refresh_folder_openers"));
  },
  async getFolderOpenerPreferences() {
    return normalizeFolderOpenerPreferences(await invoke<FolderOpenerPreferences>("get_folder_opener_preferences"));
  },
  async saveFolderOpenerPreferences(input) {
    return normalizeFolderOpenerPreferences(await invoke<FolderOpenerPreferences>("save_folder_opener_preferences", { input }));
  },
  openSessionFolder(sessionId, openerId, relativePath) {
    return invoke<OpenSessionFolderResult>("open_session_folder", {
      sessionId,
      openerId,
      relativePath: relativePath ?? null,
    });
  },
  async subscribeFolderOpenerEvents(handler) {
    return listen<string>("folder-openers:event", () => handler());
  },
  async subscribeWorkspaceInvalidation(handler) {
    return listen<unknown>(WORKSPACE_INVALIDATION_EVENT_CHANNEL, (event) => {
      // Dropped rather than thrown. An event handler has no caller to reject to, and one notice
      // this build cannot read must not tear down the subscription carrying the rest.
      const notice = safeParseWorkspaceInvalidationNotice(event.payload);
      if (notice) handler(notice);
    });
  },
  async searchWorkspacePaths(input) {
    return parseWorkspacePathSearchResult(
      await invoke("search_workspace_paths", {
        sessionId: input.sessionId,
        query: input.query,
        searchId: input.searchId,
        cursor: input.cursor ?? null,
        limit: input.limit ?? null,
      }),
    );
  },
  async searchWorkspaceContent(input) {
    return parseWorkspaceContentSearchResult(
      await invoke("search_workspace_content", {
        sessionId: input.sessionId,
        query: input.query,
        searchId: input.searchId,
        limit: input.limit ?? null,
      }),
    );
  },
  cancelWorkspaceSearch(searchId) {
    return invoke<boolean>("cancel_workspace_search", { searchId });
  },
  async getFileEvidenceLinks(sessionId, relativePath) {
    return parseFileEvidenceLinks(
      await invoke("get_file_evidence_links", { sessionId, relativePath }),
    );
  },
  listSessionDirectory(sessionId, path = "", cursor = null, limit) {
    return invoke<DirectoryListing>("list_session_directory", {
      sessionId,
      path,
      cursor,
      limit: limit ?? null,
    });
  },
  readSessionFile(sessionId, path) {
    return invoke<FileContent>("read_session_file", { sessionId, path });
  },
  listSessionDocuments(sessionId, searchId) {
    return invoke<DocumentListing>("list_session_documents", { sessionId, searchId });
  },
  searchSessionFiles(sessionId, query, maxResults) {
    return invoke<FileSearchListing>("search_session_files", { sessionId, query, maxResults });
  },
  getWorkspaceInspectionCapabilities(sessionId) {
    return invoke<WorkspaceInspectionCapabilities>("get_workspace_inspection_capabilities", {
      sessionId,
    });
  },
  getSessionGitStatus(sessionId) {
    return invoke<GitStatusResult>("get_session_git_status", { sessionId });
  },
  getSessionGitDiff(sessionId, path, source) {
    return invoke<GitDiffResult>("get_session_git_diff", { sessionId, path, source });
  },
  listSessionLogs(input) {
    return invoke<SessionLogPage>("list_session_logs", { input });
  },
  getSessionLogRecord(recordId) {
    return invoke<SessionLogEntry | null>("get_session_log_record", { recordId });
  },
  exportSessionLogs(input) {
    return invoke<SessionLogExportResult>("export_session_logs", { input });
  },
};
