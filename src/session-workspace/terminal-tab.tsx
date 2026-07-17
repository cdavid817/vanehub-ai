import { useTranslation } from "react-i18next";
import type { ChatMessage } from "../types/chat";
import { cn } from "../lib/utils";
import { PartialNotice, WorkspaceState } from "./workspace-state";

export function toolUseCount(messages: ChatMessage[]) {
  return messages.reduce((total, message) => total + (message.toolUse?.length ?? 0), 0);
}

export function TerminalTab({ messages, partial }: { messages: ChatMessage[]; partial: boolean }) {
  const { i18n, t } = useTranslation();
  const entries = messages.flatMap((message) =>
    (message.toolUse ?? []).map((tool) => ({ message, tool })),
  );
  if (entries.length === 0) return <WorkspaceState kind="empty" message={t("sessionTabs.terminal.empty")} />;
  return (
    <div className="grid gap-3 overflow-y-auto pr-1">
      {partial ? <PartialNotice /> : null}
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

