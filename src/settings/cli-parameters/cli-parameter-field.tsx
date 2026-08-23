import { TriangleAlert } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Badge } from "../../components/ui/badge";
import type { CliParameterSelection, CliParameterSelections } from "../../types/cli-parameter";
import type { CliParameterDiagnostic, CliParameterFieldView } from "../../types/cli-parameter-profile";
import { CliParameterControl } from "./cli-parameter-control";
import {
  cliParameterDiagnosticMessageKey,
  cliParameterDisplayFlag,
  isUnsupported,
  supportMessage,
  unmetDependencies,
} from "./view-model";

export interface CliParameterFieldProps {
  field: CliParameterFieldView;
  value: CliParameterSelection;
  selections: CliParameterSelections;
  dirty: boolean;
  invalid: boolean;
  customMode: boolean;
  customInput: string;
  diagnostics: readonly CliParameterDiagnostic[];
  onSelect: (selection: CliParameterSelection) => void;
  onOpenCustom: (seed: string) => void;
  onTypeCustom: (text: string) => void;
}

export function CliParameterField({
  field,
  value,
  selections,
  dirty,
  invalid,
  customMode,
  customInput,
  diagnostics,
  onSelect,
  onOpenCustom,
  onTypeCustom,
}: CliParameterFieldProps) {
  const { t } = useTranslation();
  const definition = field.definition;
  const unmet = unmetDependencies(definition, selections);
  const conflicts = definition.dependencies.conflictsWith.filter(
    (id) => selections[id] && selections[id].state !== "inherit",
  );
  const unsupported = isUnsupported(field.support);

  return (
    <section className="ucd-panel ucd-interactive rounded-lg p-4">
      <div className="grid gap-4 md:grid-cols-[minmax(0,1fr)_minmax(220px,320px)] md:items-start">
        <div className="min-w-0">
          <div className="flex flex-wrap items-center gap-2">
            <h3 className="text-sm font-semibold">{t(definition.labelKey)}</h3>
            <Badge tone="muted">{cliParameterDisplayFlag(definition)}</Badge>
            {dirty ? <Badge tone="warning">{t("cliParameters.common.unsaved")}</Badge> : null}
            {definition.maturity === "stable" ? null : (
              <Badge tone="muted">{t(`cliParameters.maturity.${definition.maturity}`)}</Badge>
            )}
            {unsupported ? <Badge tone="danger">{supportMessage(field.support, t)}</Badge> : null}
            {definition.risk === "warning" ? (
              <Badge tone="warning">
                <TriangleAlert aria-hidden="true" className="mr-1 h-3 w-3" />
                {t("cliParameters.common.warning")}
              </Badge>
            ) : null}
          </div>
          <p className="mt-2 text-sm leading-6 text-muted-foreground">
            {t(definition.descriptionKey)}
          </p>
          <p className="mt-2 text-xs text-muted-foreground">
            {t("cliParameters.common.scope", {
              scope: definition.launchScopes
                .map((scope) => t(`cliParameters.scope.${scope}`))
                .join(" / "),
            })}
          </p>
          <p className="mt-1 text-xs text-muted-foreground">{t("cliParameters.source")}</p>
          {unmet.length > 0 ? (
            <p className="mt-2 text-xs ucd-status-warning">
              {t("cliParameters.dependency.requires", { parameters: unmet.join(", ") })}
            </p>
          ) : null}
          {conflicts.length > 0 ? (
            <p className="mt-1 text-xs ucd-status-warning">
              {t("cliParameters.dependency.conflictsWith", { parameters: conflicts.join(", ") })}
            </p>
          ) : null}
          {diagnostics.map((diagnostic) => (
            <p
              className={`mt-1 text-xs ${diagnostic.severity === "error" ? "ucd-status-danger" : diagnostic.severity === "warning" ? "ucd-status-warning" : "text-muted-foreground"}`}
              key={`${diagnostic.code}-${diagnostic.parameterId ?? ""}`}
            >
              {t(cliParameterDiagnosticMessageKey(diagnostic))}
              {diagnostic.remediation === "none"
                ? null
                : ` ${t(`cliParameters.remediation.${diagnostic.remediation}`)}`}
            </p>
          ))}
        </div>
        <div className="min-w-0">
          <CliParameterControl
            customInput={customInput}
            customMode={customMode}
            definition={definition}
            disabled={unsupported}
            invalid={invalid}
            onOpenCustom={onOpenCustom}
            onSelect={onSelect}
            onTypeCustom={onTypeCustom}
            value={value}
          />
          {invalid ? (
            <p className="mt-2 text-xs ucd-status-danger" role="alert">
              {t("cliParameters.error.CLI_PARAMETER_INVALID_VALUE", {
                parameter: definition.id,
              })}
            </p>
          ) : null}
        </div>
      </div>
    </section>
  );
}
