import { useTranslation } from "react-i18next";
import type { InstructionMergeMode, PersonalizationPolicyRef } from "../../../types/personalization";
import type { InstructionValues } from "./instruction-drafts";
import {
  agentLayerVaries,
  hasInstructionText,
  mergeOutcome,
  type InheritedLayer,
} from "./inheritance-model";

function LayerText({ label, text }: { label: string; text: string }) {
  const { t } = useTranslation();
  return (
    <div className="min-w-0">
      <div className="text-xs font-medium text-muted-foreground">{label}</div>
      {text ? (
        <p className="wrap-break-word whitespace-pre-wrap text-sm">{text}</p>
      ) : (
        <p className="text-sm text-muted-foreground">{t("personalization.inheritance.emptyField")}</p>
      )}
    </div>
  );
}

/**
 * What the layers underneath currently say, and what saving will do to them.
 *
 * Both are stated before the save, because the difference between append and replace is invisible
 * afterwards: the field looks the same either way, and the only way to find out which happened is
 * to start a session.
 */
export function InheritancePanel({
  layers,
  scope,
  values,
}: {
  layers: readonly InheritedLayer[];
  scope: PersonalizationPolicyRef;
  values: InstructionValues;
}) {
  const { t } = useTranslation();
  const outcome = mergeOutcome(values.instructionMergeMode, hasInstructionText(values));

  return (
    <div className="flex flex-col gap-4" data-testid="personalization-inheritance">
      <p className="text-sm" data-testid="personalization-merge-outcome">
        {t(`personalization.inheritance.outcome.${outcome}`, { count: layers.length })}
      </p>

      {agentLayerVaries(scope) ? (
        <p className="text-xs text-muted-foreground" data-testid="personalization-inheritance-varies">
          {t("personalization.inheritance.agentLayerVaries")}
        </p>
      ) : null}

      {layers.length === 0 ? (
        <p className="text-sm text-muted-foreground" data-testid="personalization-inheritance-none">
          {scope.scopeKind === "global"
            ? t("personalization.inheritance.bottomLayer")
            : t("personalization.inheritance.nothingBelow")}
        </p>
      ) : (
        <ol className="flex flex-col gap-3">
          {layers.map((layer) => (
            <li
              className="rounded-md border border-border/70 p-3"
              data-testid={`personalization-inherited-${layer.scopeKind}`}
              key={`${layer.scopeKind}:${layer.scopeKey}`}
            >
              <div className="mb-2 flex flex-wrap items-baseline gap-2">
                <span className="text-sm font-medium">
                  {layer.scopeKey
                    ? `${t(`personalization.overview.source.${layer.scopeKind}`)} (${layer.scopeKey})`
                    : t(`personalization.overview.source.${layer.scopeKind}`)}
                </span>
                <span className="text-xs text-muted-foreground">
                  {t("personalization.inheritance.revision", { revision: layer.revision })}
                </span>
                <span className="text-xs text-muted-foreground">
                  {t(`personalization.editor.merge.${layer.mergeMode satisfies InstructionMergeMode}`)}
                </span>
              </div>
              <div className="grid gap-2 sm:grid-cols-2">
                <LayerText label={t("personalization.editor.aboutUser")} text={layer.aboutUser} />
                <LayerText label={t("personalization.editor.styleRules")} text={layer.styleRules} />
              </div>
            </li>
          ))}
        </ol>
      )}
    </div>
  );
}
