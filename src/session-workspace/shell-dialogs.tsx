import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import type { SessionShellDescriptor } from "../types/session-workspace-shell-frames";
import { closeWarningFor } from "./shell-status";

interface ShellCloseDialogProps {
  shell: SessionShellDescriptor;
  onConfirm(): void;
  onCancel(): void;
}

/**
 * The one place a Shell is ended on purpose.
 *
 * The warning is three-valued because the runtime's answer is. A PTY cannot be asked what is
 * running in it, and a dialog that turned that silence into "nothing is running" would be making a
 * claim the product cannot support — about the exact moment the user is deciding whether to kill a
 * deploy.
 */
export function ShellCloseDialog({ onCancel, onConfirm, shell }: ShellCloseDialogProps) {
  const { t } = useTranslation();
  const cancelRef = useRef<HTMLButtonElement>(null);
  const warning = closeWarningFor(shell.foregroundProcess);

  // Focus starts on Cancel, not Confirm: the destructive choice should not be one Enter away from
  // a dialog the user has not read yet.
  useEffect(() => cancelRef.current?.focus(), []);

  return (
    <ShellDialog labelledBy="shell-close-title" onCancel={onCancel}>
      <h2 className="text-sm font-medium" id="shell-close-title">
        {t("sessionTabs.shell.closeTitle", { title: shell.title })}
      </h2>
      {warning !== "none" ? (
        <p className="text-xs text-muted-foreground" role="alert">
          {t(
            warning === "running"
              ? "sessionTabs.shell.closeForegroundRunning"
              : "sessionTabs.shell.closeForegroundUnknown",
          )}
        </p>
      ) : null}
      <div className="flex justify-end gap-2">
        <button
          className="h-7 rounded border border-border px-2 text-xs hover:bg-muted"
          onClick={onCancel}
          ref={cancelRef}
          type="button"
        >
          {t("sessionTabs.shell.closeCancel")}
        </button>
        <button
          className="h-7 rounded border border-destructive px-2 text-xs text-destructive hover:bg-muted"
          onClick={onConfirm}
          type="button"
        >
          {t("sessionTabs.shell.closeConfirm")}
        </button>
      </div>
    </ShellDialog>
  );
}

interface ShellRenameDialogProps {
  shell: SessionShellDescriptor;
  onSubmit(title: string): void;
  onCancel(): void;
}

export function ShellRenameDialog({ onCancel, onSubmit, shell }: ShellRenameDialogProps) {
  const { t } = useTranslation();
  const [title, setTitle] = useState(shell.title);
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => inputRef.current?.focus(), []);

  return (
    <ShellDialog labelledBy="shell-rename-input" onCancel={onCancel}>
      <form
        className="flex flex-col gap-2"
        onSubmit={(event) => {
          event.preventDefault();
          const next = title.trim();
          // An empty rename is a cancel, not an error the user has to dismiss.
          if (next) onSubmit(next);
          else onCancel();
        }}
      >
        {/* The dialog names itself through this label. One element rather than a heading plus a
            label, so the accessible name and the field's name are the same string by construction
            rather than by two translators agreeing. */}
        <label className="text-sm font-medium" htmlFor="shell-rename-input">
          {t("sessionTabs.shell.renameTitle")}
        </label>
        <input
          className="h-7 rounded border border-border bg-transparent px-2 text-xs"
          id="shell-rename-input"
          onChange={(event) => setTitle(event.target.value)}
          ref={inputRef}
          value={title}
        />
        <div className="flex justify-end gap-2">
          <button
            className="h-7 rounded border border-border px-2 text-xs hover:bg-muted"
            onClick={onCancel}
            type="button"
          >
            {t("sessionTabs.shell.closeCancel")}
          </button>
          <button
            className="h-7 rounded border border-border px-2 text-xs hover:bg-muted"
            type="submit"
          >
            {t("sessionTabs.shell.save")}
          </button>
        </div>
      </form>
    </ShellDialog>
  );
}

function ShellDialog({
  children,
  labelledBy,
  onCancel,
}: {
  children: React.ReactNode;
  labelledBy: string;
  onCancel(): void;
}) {
  return (
    <div
      aria-labelledby={labelledBy}
      aria-modal="true"
      className="flex flex-col gap-2 border-b border-border bg-[hsl(var(--panel-muted))] p-3"
      onKeyDown={(event) => {
        if (event.key === "Escape") onCancel();
      }}
      role="dialog"
    >
      {children}
    </div>
  );
}
