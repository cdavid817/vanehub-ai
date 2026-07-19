import { Folder, GitBranch, Server } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Button } from "../components/ui/button";
import { cn } from "../lib/utils";
import { normalizeDisplayPath } from "../lib/session-path";
import type { KnownProject, KnownRemoteWorkspace, ProjectInspection } from "../types/agent";

export type WorkspaceMode = "local" | "remote";

export function WorkspaceModeSelector({
  mode,
  onModeChange,
}: {
  mode: WorkspaceMode;
  onModeChange: (mode: WorkspaceMode) => void;
}) {
  const { t } = useTranslation();
  return (
    <section className="grid gap-2">
      <span className="text-xs font-medium text-muted-foreground">{t("createSession.workspaceMode")}</span>
      <div className="grid grid-cols-2 gap-2">
        {(["local", "remote"] as const).map((candidate) => (
          <button
            className={cn(
              "ucd-list-row flex h-9 items-center justify-center gap-2 rounded-md px-3 text-xs text-foreground",
              mode === candidate && "ucd-choice-selected font-semibold",
            )}
            key={candidate}
            onClick={() => onModeChange(candidate)}
            type="button"
          >
            {candidate === "local" ? <Folder className="h-3.5 w-3.5" aria-hidden="true" /> : <Server className="h-3.5 w-3.5" aria-hidden="true" />}
            {candidate === "local" ? t("createSession.workspaceMode.local") : t("createSession.workspaceMode.remote")}
          </button>
        ))}
      </div>
    </section>
  );
}

export function LocalWorkspaceSection({
  gitCapable,
  inspection,
  knownProjects,
  onBrowseProject,
  onInspectPath,
  projectPath,
  setProjectPath,
  setWorktreeEnabled,
  setWorktreeName,
  worktreeEnabled,
  worktreeName,
}: {
  gitCapable: boolean;
  inspection: ProjectInspection | null;
  knownProjects: KnownProject[];
  onBrowseProject: () => void;
  onInspectPath: (path: string) => void;
  projectPath: string;
  setProjectPath: (value: string) => void;
  setWorktreeEnabled: (value: boolean) => void;
  setWorktreeName: (value: string) => void;
  worktreeEnabled: boolean;
  worktreeName: string;
}) {
  const { t } = useTranslation();
  return (
    <>
      <section className="grid gap-2">
        <span className="text-xs font-medium text-muted-foreground">{t("createSession.projectFolder")}</span>
        <div className="flex gap-2">
          <input
            className="ucd-input h-9 min-w-0 flex-1 rounded px-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-ring"
            onBlur={() => onInspectPath(projectPath)}
            onChange={(event) => setProjectPath(event.target.value)}
            placeholder="D:\\code\\project"
            value={normalizeDisplayPath(projectPath)}
          />
          <Button className="h-9 px-3 text-xs" onClick={onBrowseProject} type="button" variant="outline">
            <Folder className="h-3.5 w-3.5" aria-hidden="true" />
            {t("createSession.browse")}
          </Button>
        </div>
        {knownProjects.length > 0 ? (
          <div className="grid gap-1">
            <p className="px-1 text-xs font-medium text-muted-foreground">{t("createSession.recentProjects")}</p>
            {knownProjects.slice(0, 5).map((project) => (
              <button
                className="ucd-list-row flex items-center gap-2 rounded-md px-2 py-1.5 text-left text-xs"
                key={project.path}
                onClick={() => onInspectPath(project.path)}
                type="button"
              >
                <Folder className="h-3.5 w-3.5 text-primary" aria-hidden="true" />
                <span className="min-w-0 flex-1 truncate">{normalizeDisplayPath(project.path)}</span>
                <span className="text-muted-foreground">{project.isGit ? t("createSession.folderType.git") : t("createSession.folderType.folder")}</span>
              </button>
            ))}
          </div>
        ) : null}
        {inspection ? (
          <p className="text-xs text-muted-foreground">
            {inspection.isGit ? t("createSession.gitProject") : t("createSession.normalFolder")}
          </p>
        ) : null}
      </section>

      <section className="ucd-muted-panel grid gap-2 rounded-md p-3">
        <label className={cn("flex items-center gap-2 text-sm", !gitCapable && "text-muted-foreground")}>
          <input
            checked={worktreeEnabled}
            className="h-4 w-4"
            disabled={!gitCapable}
            onChange={(event) => setWorktreeEnabled(event.target.checked)}
            type="checkbox"
          />
          <GitBranch className="h-4 w-4" aria-hidden="true" />
          {t("createSession.createWorktree")}
        </label>
        {worktreeEnabled ? (
          <label className="grid gap-1">
            <span className="text-xs text-muted-foreground">{t("createSession.worktreeName")}</span>
            <input
              className="ucd-input h-9 rounded px-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-ring"
              onChange={(event) => setWorktreeName(event.target.value)}
              placeholder="feature-a"
              value={worktreeName}
            />
            <span className="text-xs text-muted-foreground">{t("createSession.worktreeHint")}</span>
          </label>
        ) : null}
      </section>
    </>
  );
}

export function RemoteWorkspaceSection({
  knownRemoteWorkspaces,
  remoteDisplayName,
  remoteHost,
  remotePath,
  remoteUser,
  setRemoteDisplayName,
  setRemoteHost,
  setRemotePath,
  setRemoteUser,
}: {
  knownRemoteWorkspaces: KnownRemoteWorkspace[];
  remoteDisplayName: string;
  remoteHost: string;
  remotePath: string;
  remoteUser: string;
  setRemoteDisplayName: (value: string) => void;
  setRemoteHost: (value: string) => void;
  setRemotePath: (value: string) => void;
  setRemoteUser: (value: string) => void;
}) {
  const { t } = useTranslation();
  return (
    <section className="grid gap-3">
      <div className="grid grid-cols-2 gap-2">
        <label className="grid gap-1">
          <span className="text-xs font-medium text-muted-foreground">{t("createSession.remoteHost")}</span>
          <input className="ucd-input h-9 rounded px-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-ring" onChange={(event) => setRemoteHost(event.target.value)} placeholder={t("createSession.remoteHostPlaceholder")} value={remoteHost} />
        </label>
        <label className="grid gap-1">
          <span className="text-xs font-medium text-muted-foreground">{t("createSession.remoteUser")}</span>
          <input className="ucd-input h-9 rounded px-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-ring" onChange={(event) => setRemoteUser(event.target.value)} placeholder={t("createSession.remoteUserPlaceholder")} value={remoteUser} />
        </label>
      </div>
      <label className="grid gap-1">
        <span className="text-xs font-medium text-muted-foreground">{t("createSession.remotePath")}</span>
        <input className="ucd-input h-9 rounded px-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-ring" onChange={(event) => setRemotePath(event.target.value)} placeholder={t("createSession.remotePathPlaceholder")} value={remotePath} />
      </label>
      <label className="grid gap-1">
        <span className="text-xs font-medium text-muted-foreground">{t("createSession.remoteDisplayName")}</span>
        <input className="ucd-input h-9 rounded px-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-ring" onChange={(event) => setRemoteDisplayName(event.target.value)} placeholder={t("createSession.remoteDisplayNamePlaceholder")} value={remoteDisplayName} />
      </label>
      {knownRemoteWorkspaces.length > 0 ? (
        <div className="grid gap-1">
          {knownRemoteWorkspaces.slice(0, 5).map((workspace) => (
            <button
              className="ucd-list-row flex items-center gap-2 rounded-md px-2 py-1.5 text-left text-xs"
              key={workspace.uri}
              onClick={() => {
                setRemoteHost(workspace.host);
                setRemoteUser(workspace.user ?? "");
                setRemotePath(workspace.path);
                setRemoteDisplayName(workspace.displayName);
              }}
              type="button"
            >
              <Server className="h-3.5 w-3.5 text-primary" aria-hidden="true" />
              <span className="min-w-0 flex-1 truncate">{workspace.displayName}</span>
              <span className="min-w-0 max-w-60 truncate text-muted-foreground">{workspace.uri}</span>
            </button>
          ))}
        </div>
      ) : null}
    </section>
  );
}
