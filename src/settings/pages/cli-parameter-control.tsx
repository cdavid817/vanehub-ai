import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "../../components/ui/button";
import type { CliParameterDefinition, CliParameterValue } from "../../types/agent";

export function CliParameterControl({ definition, value, onChange }: {
  definition: CliParameterDefinition;
  value: CliParameterValue;
  onChange: (value: CliParameterValue) => void;
}) {
  const { t } = useTranslation();
  const [customText, setCustomText] = useState("");

  if (definition.control === "boolean") {
    const checked = value === true;
    return (
      <Button aria-checked={checked} aria-label={t(definition.labelKey)} onClick={() => onChange(!checked)} size="sm" type="button" role="switch" variant={checked ? "default" : "outline"}>
        {t(checked ? "cliParameters.common.enabled" : "cliParameters.common.disabled")}
      </Button>
    );
  }

  if (definition.control === "custom-text") {
    const strValue = typeof value === "string" ? value : "default";
    const isKnown = definition.options.some((option) => option.value === strValue);
    const isCustom = !isKnown && strValue !== "default";
    const selectValue = isCustom ? "__custom__" : strValue;
    return (
      <div className="space-y-2">
        <select aria-label={t(definition.labelKey)} className="min-h-9 w-full rounded-md border border-border bg-background px-3 py-2 text-sm focus-visible:outline-hidden focus-visible:ring-2 focus-visible:ring-ring" onChange={(event) => {
          const next = event.currentTarget.value;
          onChange(next === "__custom__" ? customText : next);
        }} value={selectValue}>
          {definition.options.map((option) => <option key={option.value} value={option.value}>{t(option.labelKey)}</option>)}
          <option value="__custom__">{isCustom ? strValue : t("cliParameters.custom.option")}</option>
        </select>
        {selectValue === "__custom__" ? (
          <input aria-label={t("cliParameters.custom.placeholder")} className="min-h-9 w-full rounded-md border border-border bg-background px-3 py-2 text-sm focus-visible:outline-hidden focus-visible:ring-2 focus-visible:ring-ring" onChange={(event) => {
            setCustomText(event.currentTarget.value);
            onChange(event.currentTarget.value);
          }} placeholder={t("cliParameters.custom.placeholder")} type="text" value={isCustom ? strValue : customText} />
        ) : null}
        {!isCustom ? <p className="text-xs leading-5 text-muted-foreground">{t(definition.options.find((option) => option.value === strValue)?.descriptionKey ?? "cliParameters.values.default.description")}</p> : null}
      </div>
    );
  }

  const multiple = definition.control === "multi-enum";
  const selectValue = multiple ? (Array.isArray(value) ? value : []) : typeof value === "string" ? value : "default";
  return (
    <div className="space-y-2">
      <select aria-label={t(definition.labelKey)} className="min-h-9 w-full rounded-md border border-border bg-background px-3 py-2 text-sm focus-visible:outline-hidden focus-visible:ring-2 focus-visible:ring-ring" multiple={multiple} onChange={(event) => {
        onChange(multiple ? Array.from(event.currentTarget.selectedOptions, (option) => option.value) : event.currentTarget.value);
      }} value={selectValue}>
        {definition.options.map((option) => <option key={option.value} value={option.value}>{t(option.labelKey)}</option>)}
      </select>
      {!multiple && typeof selectValue === "string" ? <p className="text-xs leading-5 text-muted-foreground">{t(definition.options.find((option) => option.value === selectValue)?.descriptionKey ?? "cliParameters.values.default.description")}</p> : null}
    </div>
  );
}
