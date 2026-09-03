import { useState } from "react";
import { useQueryClient } from "@tanstack/react-query";
import {
  Check,
  Copy,
  ExternalLink,
  RefreshCw,
  Search,
  SquareTerminal,
  Text,
} from "lucide-react";
import { useTranslation } from "react-i18next";
import { agentService } from "../services/runtime-agent-client";
import { sessionShellService } from "../services/runtime-session-shell-client";
import { copyFileReferencePath } from "../services/file-reference-transfer";
import { cn } from "../lib/utils";
import { workspaceQueryKeys } from "./workspace-query-keys";
import { parentDirectoryOf } from "./workspace-invalidation-targets";

/**
 * The actions that apply to the tree rather than to one row.
 *
 * Unsupported actions are rendered disabled with a reason rather than hidden. A control that
 * vanishes on a remote workspace makes a reader think they misremembered where it was; one that is
 * visibly unavailable tells them the truth, which is that this workspace is on another machine.
 */
export function FilesToolbar({
  isRemote,
  onContentSearch,
  onQuickOpen,
  onShellOpened,
  selectedPath,
  sessionId,
}: {
  /** Remote workspaces cannot be revealed in a local file manager. */
  isRemote: boolean;
  onContentSearch: () => void;
  onQuickOpen: () => void;
  /**
   * Called once a Shell exists, so whoever owns the tabs can move to it.
   *
   * A callback rather than the navigation context. Reaching for that context would make every
   * panel containing this toolbar unrenderable without it — a wide dependency for one button, and
   * one that says the tree knows about tabs. It does not; its parent does.
   */
  onShellOpened: () => void;
  /** The selected file, or null. Directory actions use its parent. */
  selectedPath: string | null;
  sessionId: string;
}) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [copied, setCopied] = useState(false);

  /** What a directory action applies to: the selection's folder, or the root. */
  const directory = selectedPath ? parentDirectoryOf(selectedPath) : "";

  return (
    <div className="mb-2 flex flex-wrap items-center gap-1">
      <ToolbarButton icon={<Search className="h-3.5 w-3.5" />} label={t("sessionTabs.files.quickOpen.open")} onClick={onQuickOpen} />
      <ToolbarButton icon={<Text className="h-3.5 w-3.5" />} label={t("sessionTabs.files.contentSearch.open")} onClick={onContentSearch} />
      <ToolbarButton
        icon={<RefreshCw className="h-3.5 w-3.5" />}
        label={t("sessionTabs.files.toolbar.refresh")}
        onClick={() => {
          // The whole session, because this is the explicit counterpart to targeted invalidation:
          // a reader presses it when something changed that no notice reported, and they cannot
          // say which part.
          void queryClient.invalidateQueries({ queryKey: workspaceQueryKeys.session(sessionId) });
        }}
      />
      <ToolbarButton
        disabled={!selectedPath}
        // Disabled rather than absent: the action exists, it simply has nothing to copy yet.
        disabledReason={t("sessionTabs.files.toolbar.needsSelection")}
        icon={copied ? <Check className="h-3.5 w-3.5 text-primary" /> : <Copy className="h-3.5 w-3.5" />}
        label={copied ? t("sessionTabs.files.copyPathDone") : t("sessionTabs.files.toolbar.copyPath")}
        onClick={() => {
          if (!selectedPath) return;
          void copyFileReferencePath(selectedPath)
            .then(() => setCopied(true))
            .catch(() => setCopied(false));
        }}
      />
      <ToolbarButton
        disabled={isRemote}
        disabledReason={t("sessionTabs.files.toolbar.remoteUnsupported")}
        icon={<ExternalLink className="h-3.5 w-3.5" />}
        label={t("sessionTabs.files.toolbar.reveal")}
        onClick={() => {
          // Failures are the opener's to report: it already answers with a status, and a second
          // error path here would put a message on screen for a case that has its own.
          void agentService
            .openSessionFolder(sessionId, "file-explorer", directory)
            .catch(() => {});
        }}
      />
      <ToolbarButton
        icon={<SquareTerminal className="h-3.5 w-3.5" />}
        label={t("sessionTabs.files.toolbar.openShell")}
        onClick={() => {
          void sessionShellService
            .createSessionShell({
              sessionId,
              rows: 24,
              cols: 80,
              // A request id, so this is an explicit "open another one" rather than a claim on the
              // session's default Shell — which starts at the root and is not what was asked for.
              requestId: `files-toolbar:${directory}`,
              workingDirectory: directory,
            })
            .then(onShellOpened)
            .catch(() => {});
        }}
      />
    </div>
  );
}

function ToolbarButton({
  disabled = false,
  disabledReason,
  icon,
  label,
  onClick,
}: {
  disabled?: boolean;
  disabledReason?: string;
  icon: React.ReactNode;
  label: string;
  onClick: () => void;
}) {
  return (
    <button
      className={cn(
        "flex h-7 items-center gap-1.5 rounded border border-border px-2 text-xs text-muted-foreground hover:bg-muted",
        disabled && "cursor-not-allowed opacity-50 hover:bg-transparent",
      )}
      disabled={disabled}
      onClick={onClick}
      // The reason travels with the disabled state, so a reader hovering an unavailable control
      // learns why rather than concluding the application is broken.
      title={disabled ? disabledReason : label}
      type="button"
    >
      {icon}
      <span>{label}</span>
    </button>
  );
}
