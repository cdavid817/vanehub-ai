import {
  BarChart3,
  Bot,
  FileDiff,
  Files,
  Gauge,
  ScrollText,
  TerminalSquare,
  type LucideIcon,
} from "lucide-react";
import { useTranslation } from "react-i18next";
import type { LoopInspectionSurface, LoopInspectionTarget } from "../types/loop";

const icons: Record<LoopInspectionSurface, LucideIcon> = {
  chat: Bot,
  changes: FileDiff,
  files: Files,
  terminal: TerminalSquare,
  logs: ScrollText,
  report: BarChart3,
  usage: Gauge,
};

export const roleInspectionSurfaces: LoopInspectionSurface[] = [
  "chat",
  "changes",
  "files",
  "terminal",
  "logs",
  "report",
  "usage",
];

export function LoopInspectionActions({
  onInspect,
  sessionId,
  surfaces = roleInspectionSurfaces,
}: {
  onInspect?: (target: LoopInspectionTarget) => void;
  sessionId: string | null;
  surfaces?: LoopInspectionSurface[];
}) {
  const { t } = useTranslation();
  if (!onInspect || !sessionId) return null;
  return (
    <div aria-label={t("loops.inspection.actions")} className="mt-2 flex flex-wrap gap-1" role="group">
      {surfaces.map((surface) => {
        const Icon = icons[surface];
        const label = t("loops.inspection.openSurface", { surface: t(`loops.inspection.surface.${surface}`) });
        return (
          <button
            aria-label={label}
            className="grid h-8 w-8 shrink-0 place-items-center rounded-md border border-border text-muted-foreground hover:bg-muted hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
            key={surface}
            onClick={() => onInspect({ sessionId, surface })}
            title={label}
            type="button"
          >
            <Icon aria-hidden="true" className="h-3.5 w-3.5" />
          </button>
        );
      })}
    </div>
  );
}
