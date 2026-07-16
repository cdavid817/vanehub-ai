import { FolderOpen } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Button } from "../../../components/ui/button";
import type { SkillScope } from "../../../types/skill";

export function SkillScopeTabs({
  scope,
  workspacePath,
  onScopeChange,
  onWorkspacePathChange,
  onBrowse,
}: {
  scope: SkillScope;
  workspacePath: string;
  onScopeChange: (scope: SkillScope) => void;
  onWorkspacePathChange: (path: string) => void;
  onBrowse: () => void;
}) {
  const { t } = useTranslation();

  return (
    <section className="ucd-panel rounded-lg p-3">
      <div className="flex flex-wrap items-center gap-2">
        {(["global", "workspace"] as const).map((value) => (
          <button
            className={`rounded-md px-3 py-2 text-sm font-medium transition ${
              scope === value ? "bg-primary text-primary-foreground" : "bg-muted text-muted-foreground hover:text-foreground"
            }`}
            key={value}
            onClick={() => onScopeChange(value)}
            type="button"
          >
            {t(`skills.scope.${value}`)}
          </button>
        ))}
      </div>
      {scope === "workspace" ? (
        <div className="mt-3 flex flex-col gap-2 md:flex-row">
          <input
            className="min-w-0 flex-1 rounded-md border border-border bg-background px-3 py-2 text-sm"
            onChange={(event) => onWorkspacePathChange(event.target.value)}
            placeholder={t("skills.scope.workspacePlaceholder")}
            value={workspacePath}
          />
          <Button onClick={onBrowse} type="button" variant="outline">
            <FolderOpen className="h-4 w-4" aria-hidden="true" />
            {t("skills.scope.browse")}
          </Button>
        </div>
      ) : null}
    </section>
  );
}
