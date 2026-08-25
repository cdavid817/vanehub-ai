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
} from "../types/session-workspace";
import type { FolderOpenerAvailability, FolderOpenerPreferences, OpenSessionFolderResult } from "../types/folder-opener";
import { normalizeFolderOpeners, normalizeFolderOpenerPreferences } from "../contracts/folder-opener";

type SessionWorkspaceMethods = Pick<
  AgentService,
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
  openSessionFolder(sessionId, openerId) {
    return invoke<OpenSessionFolderResult>("open_session_folder", { sessionId, openerId });
  },
  async subscribeFolderOpenerEvents(handler) {
    return listen<string>("folder-openers:event", () => handler());
  },
  listSessionDirectory(sessionId, path = "") {
    return invoke<DirectoryListing>("list_session_directory", { sessionId, path });
  },
  readSessionFile(sessionId, path) {
    return invoke<FileContent>("read_session_file", { sessionId, path });
  },
  listSessionDocuments(sessionId) {
    return invoke<DocumentListing>("list_session_documents", { sessionId });
  },
  searchSessionFiles(sessionId, query, maxResults) {
    return invoke<FileSearchListing>("search_session_files", { sessionId, query, maxResults });
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
