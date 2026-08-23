import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "../../components/ui/button";
import type { CliParameterDefinition, CliParameterSelection } from "../../types/cli-parameter";

const inheritOption = "__inherit__";
const customOption = "__custom__";

const selectClassName =
  "min-h-9 w-full rounded-md border border-border bg-background px-3 py-2 text-sm focus-visible:outline-hidden focus-visible:ring-2 focus-visible:ring-ring";

function selectedText(selection: CliParameterSelection): string | undefined {
  return selection.state === "value" && typeof selection.value === "string"
    ? selection.value
    : undefined;
}

function selectedList(selection: CliParameterSelection): string[] {
  return selection.state === "value" && Array.isArray(selection.value) ? selection.value : [];
}

/** Inheritance is its own choice, not a value named "default". That distinction is the whole point
 * of the explicit selection envelope: a provider whose real option is literally `default` stays
 * expressible. */
export function CliParameterControl({
  definition,
  value,
  onChange,
}: {
  definition: CliParameterDefinition;
  value: CliParameterSelection;
  onChange: (value: CliParameterSelection) => void;
}) {
  const { t } = useTranslation();
  const [customText, setCustomText] = useState("");

  if (definition.control === "boolean-flag" || definition.control === "tri-state") {
    const checked = value.state === "value" && value.value === true;
    return (
      <Button
        aria-checked={checked}
        aria-label={t(definition.labelKey)}
        onClick={() => onChange(checked ? { state: "inherit" } : { state: "value", value: true })}
        role="switch"
        size="sm"
        type="button"
        variant={checked ? "default" : "outline"}
      >
        {t(checked ? "cliParameters.common.enabled" : "cliParameters.common.disabled")}
      </Button>
    );
  }

  if (definition.control === "custom-text") {
    const text = selectedText(value);
    const isKnown = text !== undefined && definition.options.some((option) => option.value === text);
    const isCustom = text !== undefined && !isKnown;
    const selectValue = text === undefined ? inheritOption : isCustom ? customOption : text;
    return (
      <div className="space-y-2">
        <select
          aria-label={t(definition.labelKey)}
          className={selectClassName}
          onChange={(event) => {
            const next = event.currentTarget.value;
            if (next === inheritOption) onChange({ state: "inherit" });
            else if (next === customOption) onChange({ state: "value", value: customText });
            else onChange({ state: "value", value: next });
          }}
          value={selectValue}
        >
          <option value={inheritOption}>{t("cliParameters.values.default.label")}</option>
          {definition.options.map((option) => (
            <option key={option.value} value={option.value}>
              {t(option.labelKey)}
            </option>
          ))}
          <option value={customOption}>{isCustom ? text : t("cliParameters.custom.option")}</option>
        </select>
        {selectValue === customOption ? (
          <input
            aria-label={t("cliParameters.custom.placeholder")}
            className={selectClassName}
            onChange={(event) => {
              setCustomText(event.currentTarget.value);
              onChange({ state: "value", value: event.currentTarget.value });
            }}
            placeholder={t("cliParameters.custom.placeholder")}
            type="text"
            value={isCustom ? text : customText}
          />
        ) : null}
        {!isCustom ? (
          <p className="text-xs leading-5 text-muted-foreground">
            {t(
              definition.options.find((option) => option.value === text)?.descriptionKey ??
                "cliParameters.values.default.description",
            )}
          </p>
        ) : null}
      </div>
    );
  }

  if (definition.control === "multi-enum") {
    const entries = selectedList(value);
    return (
      <select
        aria-label={t(definition.labelKey)}
        className={selectClassName}
        multiple
        onChange={(event) => {
          const next = Array.from(event.currentTarget.selectedOptions, (option) => option.value);
          onChange(next.length === 0 ? { state: "inherit" } : { state: "value", value: next });
        }}
        value={entries}
      >
        {definition.options.map((option) => (
          <option key={option.value} value={option.value}>
            {t(option.labelKey)}
          </option>
        ))}
      </select>
    );
  }

  if (definition.control === "ordered-string-list" || definition.control === "path-list") {
    const entries = selectedList(value);
    return (
      <textarea
        aria-label={t(definition.labelKey)}
        className={selectClassName}
        onChange={(event) => {
          const next = event.currentTarget.value
            .split("\n")
            .map((line) => line.trim())
            .filter((line) => line.length > 0);
          onChange(next.length === 0 ? { state: "inherit" } : { state: "value", value: next });
        }}
        placeholder={t("cliParameters.list.placeholder")}
        rows={3}
        value={entries.join("\n")}
      />
    );
  }

  const text = selectedText(value);
  return (
    <div className="space-y-2">
      <select
        aria-label={t(definition.labelKey)}
        className={selectClassName}
        onChange={(event) => {
          const next = event.currentTarget.value;
          onChange(next === inheritOption ? { state: "inherit" } : { state: "value", value: next });
        }}
        value={text ?? inheritOption}
      >
        <option value={inheritOption}>{t("cliParameters.values.default.label")}</option>
        {definition.options.map((option) => (
          <option key={option.value} value={option.value}>
            {t(option.labelKey)}
          </option>
        ))}
      </select>
      <p className="text-xs leading-5 text-muted-foreground">
        {t(
          definition.options.find((option) => option.value === text)?.descriptionKey ??
            "cliParameters.values.default.description",
        )}
      </p>
    </div>
  );
}
