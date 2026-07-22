import { FolderOpen, Save } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Button } from "../../../components/ui/button";
import { normalizeDisplayPath } from "../../../lib/session-path";
import type { AgentRegistryEntry, KnownProject } from "../../../types/agent";
import { SectionPanel } from "../page-parts";

interface ImRoutingSectionProps {
  agentId: string;
  projectPath: string;
  agents: AgentRegistryEntry[];
  projects: KnownProject[];
  errors: { agentId?: string; projectPath?: string };
  pending: boolean;
  onAgentChange: (value: string) => void;
  onProjectChange: (value: string) => void;
  onBrowse: () => void;
  onSave: () => void;
}

export function ImRoutingSection(props: ImRoutingSectionProps) {
  const { t } = useTranslation();
  return (
    <SectionPanel title={t("im.routing.title")} description={t("im.routing.description")}>
      <div className="grid gap-4 lg:grid-cols-2">
        <label className="grid gap-1.5 text-sm">
          <span className="font-medium">{t("im.routing.agent")}</span>
          <select
            aria-invalid={Boolean(props.errors.agentId)}
            className="ucd-input h-9 min-w-0 rounded px-3 outline-none focus-visible:ring-2 focus-visible:ring-ring"
            onChange={(event) => props.onAgentChange(event.target.value)}
            value={props.agentId}
          >
            <option value="">{t("im.routing.agentPlaceholder")}</option>
            {props.agents.map((agent) => <option key={agent.id} value={agent.id}>{agent.displayName}</option>)}
          </select>
          {props.errors.agentId ? <span className="text-xs text-destructive">{t("im.validation.agentRequired")}</span> : null}
        </label>
        <label className="grid gap-1.5 text-sm">
          <span className="font-medium">{t("im.routing.project")}</span>
          <div className="flex min-w-0 gap-2">
            <select
              aria-invalid={Boolean(props.errors.projectPath)}
              className="ucd-input h-9 min-w-0 flex-1 rounded px-3 outline-none focus-visible:ring-2 focus-visible:ring-ring"
              onChange={(event) => props.onProjectChange(event.target.value)}
              value={props.projectPath}
            >
              <option value="">{t("im.routing.projectPlaceholder")}</option>
              {props.projectPath && !props.projects.some((project) => normalizeDisplayPath(project.path) === normalizeDisplayPath(props.projectPath)) ? (
                <option value={normalizeDisplayPath(props.projectPath)}>{normalizeDisplayPath(props.projectPath)}</option>
              ) : null}
              {props.projects.map((project) => (
                <option key={project.path} value={normalizeDisplayPath(project.path)}>
                  {project.displayName} - {normalizeDisplayPath(project.path)}
                </option>
              ))}
            </select>
            <Button aria-label={t("im.routing.browse")} onClick={props.onBrowse} size="icon" title={t("im.routing.browse")} type="button" variant="outline">
              <FolderOpen aria-hidden="true" />
            </Button>
          </div>
          {props.errors.projectPath ? <span className="text-xs text-destructive">{t("im.validation.projectRequired")}</span> : null}
        </label>
      </div>
      <div className="mt-4 flex flex-wrap items-center justify-between gap-3 border-t border-border/70 pt-3">
        <p className="text-xs text-muted-foreground">{t("im.routing.effect")}</p>
        <Button disabled={props.pending} onClick={props.onSave} type="button">
          <Save aria-hidden="true" />
          {props.pending ? t("im.actions.saving") : t("im.actions.saveRouting")}
        </Button>
      </div>
    </SectionPanel>
  );
}
