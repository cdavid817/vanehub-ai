import { useTranslation } from "react-i18next";
import type { ChatMessage } from "../types/chat";
import { cn } from "../lib/utils";
import { PartialNotice, WorkspaceState } from "./workspace-state";
import { BuiltinToolActivity } from "./builtin-tool-activity";
import { ArtifactPanel } from "./artifact-panel";
import { DelegationPanel } from "./delegation-panel";
export { toolUseCount } from "./terminal-utils";

export function TerminalTab({ builtinToolsAvailable = false, messages, partial, seatId = null, sessionId = null, targetRoot = "" }: { builtinToolsAvailable?: boolean; messages: ChatMessage[]; partial: boolean; seatId?: string | null; sessionId?: string | null; targetRoot?: string }) {
  const { i18n, t } = useTranslation();
  const entries = messages
    // A message written before speaker attribution existed carries no seat, so it is not shown as
    // this seat's work rather than being attributed to whichever seat is selected.
    .filter((message) => seatId === null || message.speakerSeatId === seatId)
    .flatMap((message) => (message.toolUse ?? []).map((tool) => ({ message, tool })));
  return (
    <div className="grid gap-3 overflow-y-auto pr-1">
      {builtinToolsAvailable && sessionId ? <BuiltinToolActivity sessionId={sessionId} /> : null}
      {builtinToolsAvailable && sessionId ? <ArtifactPanel sessionId={sessionId} /> : null}
      {builtinToolsAvailable && sessionId ? <DelegationPanel defaultTargetRoot={targetRoot} sessionId={sessionId} /> : null}
      {partial ? <PartialNotice /> : null}
      {entries.length === 0 ? <WorkspaceState kind="empty" message={t("sessionTabs.terminal.empty")} /> : null}
      {entries.map(({ message, tool }) => (
        <article className="rounded-lg border border-border bg-[hsl(var(--panel-muted))] p-3" key={`${message.id}-${tool.id}`}>
          <div className="flex items-center justify-between gap-3">
            <h3 className="truncate font-mono text-sm font-semibold text-primary">{tool.name}</h3>
            <span className={cn("rounded-full px-2 py-1 text-xs", tool.status === "failed" ? "bg-destructive/10 text-destructive" : "bg-muted text-muted-foreground")}>
              {t(`sessionTabs.toolStatus.${tool.status}`)}
            </span>
          </div>
          <p className="mt-1 text-xs text-muted-foreground">{new Intl.DateTimeFormat(i18n.language, { dateStyle: "short", timeStyle: "medium" }).format(new Date(message.createdAt))}</p>
          {tool.input !== undefined ? <DataBlock label={t("sessionTabs.terminal.input")} value={tool.input} /> : null}
          {tool.output !== undefined ? <DataBlock label={t("sessionTabs.terminal.output")} value={tool.output} /> : null}
        </article>
      ))}
    </div>
  );
}

function DataBlock({ label, value }: { label: string; value: unknown }) {
  return (
    <div className="mt-3">
      <p className="mb-1 text-xs font-medium text-muted-foreground">{label}</p>
      <pre className="max-h-48 overflow-auto whitespace-pre-wrap break-all rounded-md border border-border bg-background p-2 text-xs">
        {typeof value === "string" ? value : JSON.stringify(value, null, 2)}
      </pre>
    </div>
  );
}

