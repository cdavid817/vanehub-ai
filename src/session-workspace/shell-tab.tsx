import { useCallback, useState } from "react";
import { useTranslation } from "react-i18next";
import type { SessionShellDescriptor } from "../types/session-workspace-shell-frames";
import { ShellCloseDialog, ShellRenameDialog } from "./shell-dialogs";
import { ShellStrip } from "./shell-strip";
import { ShellSurface } from "./shell-surface";
import { useSessionShells } from "./use-session-shells";
import { WorkspaceState } from "./workspace-state";
import type { WorkspaceErrorKey } from "./workspace-error";

type ShellDialogRequest = { kind: "close" | "rename"; shellId: string } | null;

/**
 * The Shell tab, over a registry of retained Shells.
 *
 * Hiding this tab detaches its view; it does not end anything. That asymmetry is the whole
 * capability: a build survives a tab switch, a session switch, and a remount, and stops only when
 * the user says Close and confirms it.
 *
 * Every Shell keeps its own surface mounted so its scrollback survives selection. Only the selected
 * one holds an attachment, because a Shell has one current attachment and the others would be
 * displacing each other on every click.
 */
export function ShellTab({
  isVisible,
  seatId = null,
  sessionId,
}: {
  isVisible: boolean;
  seatId?: string | null;
  sessionId: string | null;
}) {
  const { t } = useTranslation();
  const shells = useSessionShells(sessionId, seatId, isVisible);
  const [dialog, setDialog] = useState<ShellDialogRequest>(null);
  const [surfaceError, setSurfaceError] = useState<WorkspaceErrorKey | null>(null);

  const applyDescriptor = shells.applyDescriptor;
  const onDescriptor = useCallback(
    (descriptor: SessionShellDescriptor) => applyDescriptor(descriptor),
    [applyDescriptor],
  );
  const onError = useCallback((error: WorkspaceErrorKey) => setSurfaceError(error), []);

  if (!sessionId) return <WorkspaceState kind="unavailable" />;

  const dialogShell = dialog
    ? (shells.shells.find((shell) => shell.shellId === dialog.shellId) ?? null)
    : null;
  const error = shells.error ?? surfaceError;

  return (
    <div className="flex h-full min-h-0 flex-col overflow-hidden rounded-lg border border-border bg-[hsl(var(--panel-muted))]">
      <ShellStrip
        activeShellId={shells.activeShellId}
        onAdd={() => void shells.addShell()}
        onClose={(shellId) => setDialog({ kind: "close", shellId })}
        onRename={(shellId) => setDialog({ kind: "rename", shellId })}
        onSelect={shells.selectShell}
        shells={shells.shells}
      />
      {dialogShell && dialog?.kind === "close" ? (
        <ShellCloseDialog
          onCancel={() => setDialog(null)}
          onConfirm={() => {
            setDialog(null);
            void shells.closeShell(dialogShell.shellId);
          }}
          shell={dialogShell}
        />
      ) : null}
      {dialogShell && dialog?.kind === "rename" ? (
        <ShellRenameDialog
          onCancel={() => setDialog(null)}
          onSubmit={(title) => {
            setDialog(null);
            void shells.renameShell(dialogShell.shellId, title);
          }}
          shell={dialogShell}
        />
      ) : null}
      {error ? (
        <div className="p-2">
          <WorkspaceState kind="error" message={t(error)} />
        </div>
      ) : null}
      {shells.shells.length === 0 ? (
        <div className="p-2">
          <WorkspaceState kind="empty" message={t("sessionTabs.shell.empty")} />
        </div>
      ) : null}
      {shells.shells.map((shell) => (
        <div
          className={shell.shellId === shells.activeShellId ? "flex min-h-0 flex-1" : "hidden"}
          key={shell.shellId}
        >
          <ShellSurface
            descriptor={shell}
            isVisible={isVisible && shell.shellId === shells.activeShellId}
            onDescriptor={onDescriptor}
            onError={onError}
          />
        </div>
      ))}
    </div>
  );
}
