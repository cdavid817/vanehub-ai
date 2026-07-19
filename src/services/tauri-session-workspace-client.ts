import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { AgentService } from "./agent-service";
import type {
  DirectoryListing,
  DocumentListing,
  FileContent,
  GitDiffResult,
  GitStatusResult,
  SessionLogExportResult,
  SessionLogPage,
  ShellEvent,
  ShellSession,
} from "../types/session-workspace";
import type { FolderOpenerAvailability, FolderOpenerPreferences, OpenSessionFolderResult } from "../types/folder-opener";
import { normalizeFolderOpeners, normalizeFolderOpenerPreferences } from "../contracts/folder-opener";

type SessionWorkspaceMethods = Pick<
  AgentService,
  | "listSessionDirectory"
  | "readSessionFile"
  | "listSessionDocuments"
  | "getSessionGitStatus"
  | "getSessionGitDiff"
  | "listSessionLogs"
  | "exportSessionLogs"
  | "createShell"
  | "writeShellInput"
  | "resetShellDirectory"
  | "resizeShell"
  | "killShell"
  | "subscribeShellEvents"
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
  getSessionGitStatus(sessionId) {
    return invoke<GitStatusResult>("get_session_git_status", { sessionId });
  },
  getSessionGitDiff(sessionId, path, source) {
    return invoke<GitDiffResult>("get_session_git_diff", { sessionId, path, source });
  },
  listSessionLogs(input) {
    return invoke<SessionLogPage>("list_session_logs", { input });
  },
  exportSessionLogs(input) {
    return invoke<SessionLogExportResult>("export_session_logs", { input });
  },
  createShell(input) {
    return invoke<ShellSession>("shell_create", { input });
  },
  async writeShellInput(shellId, content) {
    await invoke<void>("shell_input", { shellId, content });
  },
  async resetShellDirectory(shellId) {
    await invoke<void>("shell_cd", { shellId });
  },
  async resizeShell(input) {
    await invoke<void>("shell_resize", { input });
  },
  async killShell(shellId) {
    await invoke<void>("shell_kill", { shellId });
  },
  async subscribeShellEvents(shellId, handler) {
    return listen<ShellEvent>("shell:event", (event) => {
      if (event.payload.shellId === shellId) handler(event.payload);
    });
  },
};
