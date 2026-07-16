import { ChevronDown, Layers, Plus, Shield, SlidersHorizontal, Users, Wrench, Zap } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Button } from "../components/ui/button";
import { cn } from "../lib/utils";
import type { WorkspaceConversation, WorkspaceTool, WorkspaceToolIconMap } from "../types/workspace";

const workspaceToolIcons: WorkspaceToolIconMap = {
  shield: Shield,
  wrench: Wrench,
  zap: Zap,
  layers: Layers,
  sliders: SlidersHorizontal,
  users: Users,
};

export function ConversationSidebar({
  conversations,
  onOpenSettings,
  tools,
}: {
  conversations: WorkspaceConversation[];
  onOpenSettings: () => void;
  tools: WorkspaceTool[];
}) {
  const { t } = useTranslation();

  return (
    <aside className="ucd-panel flex min-h-0 flex-col rounded-xl p-3">
      <div className="mb-3 flex items-center justify-between gap-3">
        <h2 className="text-sm font-semibold">{t("layout.sessions")}</h2>
        <Button className="h-7 px-2 text-xs">
          <Plus className="h-3.5 w-3.5" aria-hidden="true" />
          {t("layout.new")}
        </Button>
      </div>

      <div className="mb-3 flex gap-1">
        {[t("layout.all"), t("layout.favorite"), t("layout.archive")].map((item, index) => (
          <button
            className={cn(
              "h-7 rounded border border-border px-3 text-xs",
              index === 0 ? "bg-background text-foreground" : "text-muted-foreground hover:bg-muted",
            )}
            key={item}
            type="button"
          >
            {item}
            {index === 0 ? <ChevronDown className="ml-1 inline h-3 w-3" aria-hidden="true" /> : null}
          </button>
        ))}
      </div>

      <div className="grid gap-2">
        {conversations.map((item) => (
          <button
            className={cn(
              "relative rounded-lg border border-border p-3 text-left transition-colors hover:bg-muted",
              item.active && "bg-[hsl(var(--nav-active-soft))]",
            )}
            key={item.title}
            type="button"
          >
            {item.active ? <span className="absolute left-0 top-2 h-10 w-0.5 rounded bg-primary" /> : null}
            <div className={cn("truncate text-sm font-medium", item.archived && "text-muted-foreground")}>{item.title}</div>
            <div className="mt-2 flex items-center gap-2 text-xs text-muted-foreground">
              <span className={cn("h-2 w-2 rounded-full", item.archived ? "bg-muted-foreground" : "bg-[hsl(var(--success))]")} />
              <span>{item.status}</span>
              <span>·</span>
              <span className="font-mono">{item.agents}</span>
              <span className="ml-auto font-mono">{item.date}</span>
            </div>
          </button>
        ))}
      </div>

      <div className="mt-auto border-t border-border pt-3">
        <h3 className="mb-2 text-xs font-semibold">{t("layout.tools")}</h3>
        <div className="grid gap-1.5">
          {tools.map((tool) => {
            const Icon = workspaceToolIcons[tool.iconName];
            return (
              <button className="flex h-8 items-center gap-2 rounded border border-border px-2 text-sm hover:bg-muted" key={tool.label} type="button">
                <Icon className={cn("h-4 w-4", tool.tone)} aria-hidden="true" />
                <span>{tool.label}</span>
              </button>
            );
          })}
        </div>
        <div className="mt-3 grid grid-cols-2 gap-1.5">
          <button className="h-7 rounded border border-border text-xs hover:bg-muted" onClick={onOpenSettings} type="button">
            {t("layout.settings")}
          </button>
          <button className="h-7 rounded border border-border text-xs hover:bg-muted" type="button">
            {t("layout.help")}
          </button>
        </div>
      </div>
    </aside>
  );
}
