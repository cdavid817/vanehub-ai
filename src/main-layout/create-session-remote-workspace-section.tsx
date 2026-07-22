import { Server } from "lucide-react";
import { useTranslation } from "react-i18next";
import type { KnownRemoteWorkspace } from "../types/agent";
import type {
  SaveSshConnectionInput,
  SshAuthMode,
  SshConnection,
} from "../types/ssh-connection";
import { sshConnectionSaveErrorKey } from "./create-session-dialog-utils";

export function RemoteWorkspaceSection({
  knownRemoteWorkspaces,
  remoteDisplayName,
  remoteHost,
  remotePath,
  remotePort,
  remoteUser,
  saveSshConnection,
  selectedSshConnectionId,
  setRemoteDisplayName,
  setRemoteHost,
  setRemotePath,
  setRemotePort,
  setRemoteUser,
  setSaveSshConnection,
  setSelectedSshConnectionId,
  setSshConnectionDraft,
  sshConnectionDraft,
  sshConnections,
}: {
  knownRemoteWorkspaces: KnownRemoteWorkspace[];
  remoteDisplayName: string;
  remoteHost: string;
  remotePath: string;
  remotePort: string;
  remoteUser: string;
  saveSshConnection: boolean;
  selectedSshConnectionId: string;
  setRemoteDisplayName: (value: string) => void;
  setRemoteHost: (value: string) => void;
  setRemotePath: (value: string) => void;
  setRemotePort: (value: string) => void;
  setRemoteUser: (value: string) => void;
  setSaveSshConnection: (value: boolean) => void;
  setSelectedSshConnectionId: (value: string) => void;
  setSshConnectionDraft: (value: SaveSshConnectionInput) => void;
  sshConnectionDraft: SaveSshConnectionInput;
  sshConnections: SshConnection[];
}) {
  const { t } = useTranslation();
  const updateDraft = <K extends keyof SaveSshConnectionInput>(
    key: K,
    value: SaveSshConnectionInput[K],
  ) => {
    setSshConnectionDraft({ ...sshConnectionDraft, [key]: value });
  };
  const saveErrorKey = saveSshConnection
    ? sshConnectionSaveErrorKey(remoteUser, sshConnectionDraft)
    : null;
  return (
    <section className="grid gap-3">
      {sshConnections.length > 0 ? (
        <label className="grid gap-1">
          <span className="text-xs font-medium text-muted-foreground">
            {t("createSession.sshConnection")}
          </span>
          <select
            className="ucd-input h-9 rounded px-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-ring"
            onChange={(event) => selectProfile(event.target.value)}
            value={selectedSshConnectionId}
          >
            <option value="">{t("createSession.sshConnectionManual")}</option>
            {sshConnections.map((connection) => (
              <option key={connection.id} value={connection.id}>
                {connection.name}
              </option>
            ))}
          </select>
        </label>
      ) : null}
      <div className="grid grid-cols-3 gap-2">
        <RemoteField
          label={t("createSession.remoteHost")}
          value={remoteHost}
          onChange={setRemoteHost}
          placeholder={t("createSession.remoteHostPlaceholder")}
        />
        <RemoteField
          label={t("createSession.remotePort")}
          value={remotePort}
          onChange={setRemotePort}
          placeholder="22"
        />
        <RemoteField
          label={t("createSession.remoteUser")}
          value={remoteUser}
          onChange={setRemoteUser}
          placeholder={t("createSession.remoteUserPlaceholder")}
        />
      </div>
      <RemoteField
        label={t("createSession.remotePath")}
        value={remotePath}
        onChange={setRemotePath}
        placeholder={t("createSession.remotePathPlaceholder")}
      />
      <RemoteField
        label={t("createSession.remoteDisplayName")}
        value={remoteDisplayName}
        onChange={setRemoteDisplayName}
        placeholder={t("createSession.remoteDisplayNamePlaceholder")}
      />
      {knownRemoteWorkspaces.length > 0 ? (
        <div className="grid gap-1">
          {knownRemoteWorkspaces.slice(0, 5).map((workspace) => (
            <button
              className="ucd-list-row flex items-center gap-2 rounded-md px-2 py-1.5 text-left text-xs"
              key={workspace.uri}
              onClick={() => selectHistory(workspace)}
              type="button"
            >
              <Server className="h-3.5 w-3.5 text-primary" aria-hidden="true" />
              <span className="min-w-0 flex-1 truncate">
                {workspace.displayName}
              </span>
              <span className="min-w-0 max-w-60 truncate text-muted-foreground">
                {workspace.uri}
              </span>
            </button>
          ))}
        </div>
      ) : null}
      <label className="ucd-muted-panel flex items-center gap-2 rounded-md p-3 text-sm">
        <input
          checked={saveSshConnection}
          className="h-4 w-4"
          onChange={(event) => setSaveSshConnection(event.target.checked)}
          type="checkbox"
        />
        {t("createSession.saveSshConnection")}
      </label>
      {saveSshConnection ? (
        <>
          <SaveConnectionFields
            draft={sshConnectionDraft}
            onUpdate={updateDraft}
          />
          {saveErrorKey ? (
            <span className="text-xs text-destructive">{t(saveErrorKey)}</span>
          ) : null}
        </>
      ) : null}
    </section>
  );

  function selectProfile(profileId: string) {
    const profile = sshConnections.find(
      (connection) => connection.id === profileId,
    );
    setSelectedSshConnectionId(profileId);
    if (!profile) return;
    setSaveSshConnection(false);
    setRemoteHost(profile.host);
    setRemotePort(String(profile.port));
    setRemoteUser(profile.user);
    setRemotePath(profile.defaultPath);
    setRemoteDisplayName(profile.name);
  }

  function selectHistory(workspace: KnownRemoteWorkspace) {
    setRemoteHost(workspace.host);
    setRemotePort(String(workspace.port ?? 22));
    setRemoteUser(workspace.user ?? "");
    setRemotePath(workspace.path);
    setRemoteDisplayName(workspace.displayName);
  }
}

function SaveConnectionFields({
  draft,
  onUpdate,
}: {
  draft: SaveSshConnectionInput;
  onUpdate: <K extends keyof SaveSshConnectionInput>(
    key: K,
    value: SaveSshConnectionInput[K],
  ) => void;
}) {
  const { t } = useTranslation();
  return (
    <div className="ucd-muted-panel grid gap-2 rounded-md p-3">
      <div className="grid grid-cols-2 gap-2">
        <RemoteField
          label={t("createSession.sshConnectionName")}
          value={draft.name}
          onChange={(value) => onUpdate("name", value)}
        />
        <label className="grid gap-1">
          <span className="text-xs font-medium text-muted-foreground">
            {t("createSession.sshAuthMode")}
          </span>
          <select
            className="ucd-input h-9 rounded px-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-ring"
            onChange={(event) =>
              onUpdate("authMode", event.target.value as SshAuthMode)
            }
            value={draft.authMode}
          >
            <option value="key">{t("sshConnections.auth.key")}</option>
            <option value="password">
              {t("sshConnections.auth.password")}
            </option>
          </select>
        </label>
      </div>
      {draft.authMode === "key" ? (
        <RemoteField
          label={t("sshConnections.fields.keyPath")}
          value={draft.keyPath ?? ""}
          onChange={(value) => onUpdate("keyPath", value)}
        />
      ) : (
        <RemoteField
          label={t("sshConnections.fields.password")}
          type="password"
          value={draft.password ?? ""}
          onChange={(value) => onUpdate("password", value)}
        />
      )}
    </div>
  );
}

function RemoteField({
  label,
  value,
  onChange,
  placeholder,
  type = "text",
}: {
  label: string;
  value: string;
  onChange: (value: string) => void;
  placeholder?: string;
  type?: string;
}) {
  return (
    <label className="grid gap-1">
      <span className="text-xs font-medium text-muted-foreground">{label}</span>
      <input
        className="ucd-input h-9 rounded px-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-ring"
        onChange={(event) => onChange(event.target.value)}
        placeholder={placeholder}
        type={type}
        value={value}
      />
    </label>
  );
}
