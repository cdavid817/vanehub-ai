import { Save } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Button } from "../../../components/ui/button";
import type { AgentRegistryEntry } from "../../../types/agent";
import type { SkillAgentMountPath, SkillMountMigrationReport } from "../../../types/skill";
import { SectionPanel } from "../page-parts";

export function SkillAgentMountPathsPanel({
  agents,
  mountPaths,
  drafts,
  migration,
  savingAgentId,
  onDraftChange,
  onSave,
}: {
  agents: AgentRegistryEntry[];
  mountPaths: SkillAgentMountPath[];
  drafts: Record<string, string>;
  migration: SkillMountMigrationReport | null;
  savingAgentId: string | null;
  onDraftChange: (agentId: string, value: string) => void;
  onSave: (agentId: string) => void;
}) {
  const { t } = useTranslation();
  const pathFor = (agentId: string) => mountPaths.find((path) => path.agentId === agentId);
  return (
    <SectionPanel title={t("skills.mountPaths.title")} description={t("skills.mountPaths.description")}>
      <div className="grid gap-3 lg:grid-cols-2">
        {agents.map((agent) => {
          const saved = pathFor(agent.id);
          const value = drafts[agent.id] ?? saved?.mountPath ?? "";
          return (
            <div className="rounded-md border border-border p-3" key={agent.id}>
              <div className="mb-2 flex items-center justify-between gap-3">
                <div>
                  <div className="text-sm font-semibold">{agent.displayName}</div>
                  <div className="text-xs text-muted-foreground">{agent.id}</div>
                </div>
                {saved?.isDefault ? <span className="text-xs text-muted-foreground">{t("skills.mountPaths.default")}</span> : null}
              </div>
              <div className="flex gap-2">
                <code className="min-w-0 flex-1 rounded bg-muted px-2 py-2 text-xs">
                  <input
                    className="w-full bg-transparent outline-none"
                    onChange={(event) => onDraftChange(agent.id, event.target.value)}
                    value={value}
                  />
                </code>
                <Button disabled={savingAgentId === agent.id} onClick={() => onSave(agent.id)} variant="outline">
                  <Save className="h-4 w-4" aria-hidden="true" />
                </Button>
              </div>
            </div>
          );
        })}
      </div>
      {migration ? (
        <div className="mt-3 rounded-md border border-border bg-muted p-3 text-xs">
          {t("skills.mountPaths.migrated", { count: migration.migrated.length, agentId: migration.agentId })}
          {migration.failed.length > 0 ? ` ${t("skills.mountPaths.failed", { count: migration.failed.length })}` : ""}
        </div>
      ) : null}
    </SectionPanel>
  );
}
