import { useTranslation } from "react-i18next";
import type { SessionShellDescriptor } from "../types/session-workspace-shell-frames";
import { shellControls, shellEndingDetail, shellRuntimeKey, shellStateKey } from "./shell-status";

interface ShellStripProps {
  shells: SessionShellDescriptor[];
  activeShellId: string | null;
  onSelect(shellId: string): void;
  onAdd(): void;
  onRename(shellId: string): void;
  onClose(shellId: string): void;
}

/**
 * The Shell selector.
 *
 * A tablist rather than a row of buttons, because these are views of sibling panels and a keyboard
 * user navigates them with arrow keys. Close is a separate control with its own label: a tab whose
 * activation could also end a process would make one keystroke mean two things.
 */
export function ShellStrip({
  activeShellId,
  onAdd,
  onClose,
  onRename,
  onSelect,
  shells,
}: ShellStripProps) {
  const { t } = useTranslation();
  const active = shells.find((shell) => shell.shellId === activeShellId) ?? null;
  const controls = active
    ? shellControls(active)
    : { canRename: false, canClose: false, closeIntent: "close" as const };
  return (
    <div className="flex flex-wrap items-center gap-1 border-b border-border p-2 text-xs">
      <div aria-label={t("sessionTabs.shell.strip")} className="flex flex-wrap gap-1" role="tablist">
        {shells.map((shell) => {
          const selected = shell.shellId === activeShellId;
          return (
            <button
              aria-selected={selected}
              className={`flex h-7 items-center gap-1 rounded border px-2 ${
                selected ? "border-primary bg-muted" : "border-border hover:bg-muted"
              }`}
              key={shell.shellId}
              onDoubleClick={() => onRename(shell.shellId)}
              onClick={() => onSelect(shell.shellId)}
              role="tab"
              tabIndex={selected ? 0 : -1}
              type="button"
            >
              <span>{shell.title}</span>
              <span className="text-muted-foreground">{t(shellStateKey(shell))}</span>
            </button>
          );
        })}
      </div>
      <button
        className="h-7 rounded border border-border px-2 hover:bg-muted"
        onClick={onAdd}
        type="button"
      >
        {t("sessionTabs.shell.add")}
      </button>
      {active ? (
        <div className="ml-auto flex items-center gap-1">
          <ActiveShellDetail shells={shells} activeShellId={active.shellId} />
          <button
            className="h-7 rounded border border-border px-2 hover:bg-muted disabled:opacity-50"
            disabled={!controls.canRename}
            onClick={() => onRename(active.shellId)}
            type="button"
          >
            {t("sessionTabs.shell.rename")}
          </button>
          <button
            className="h-7 rounded border border-border px-2 hover:bg-muted disabled:opacity-50"
            disabled={!controls.canClose}
            onClick={() => onClose(active.shellId)}
            type="button"
          >
            {/* "Retry" rather than "Close" once a close failed. The same button doing a different
                thing under the same word would let a reader press it believing nothing had been
                tried yet. */}
            {t(
              controls.closeIntent === "retry"
                ? "sessionTabs.shell.retryClose"
                : "sessionTabs.shell.close",
            )}
          </button>
        </div>
      ) : null}
    </div>
  );
}

function ActiveShellDetail({
  activeShellId,
  shells,
}: {
  activeShellId: string;
  shells: SessionShellDescriptor[];
}) {
  const { t } = useTranslation();
  const active = shells.find((shell) => shell.shellId === activeShellId);
  if (!active) return null;
  const ending = shellEndingDetail(active);
  return (
    <>
      <span className="rounded-full bg-muted px-2 py-1 text-muted-foreground">
        {t(shellRuntimeKey(active))}
      </span>
      {ending ? (
        <span className="rounded-full border border-border px-2 py-1 text-muted-foreground">
          {ending}
        </span>
      ) : null}
    </>
  );
}
