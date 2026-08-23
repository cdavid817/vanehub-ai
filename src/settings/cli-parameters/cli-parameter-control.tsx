import { useTranslation } from "react-i18next";
import { Button } from "../../components/ui/button";
import type { CliParameterDefinition, CliParameterSelection } from "../../types/cli-parameter";
import { CliParameterListControl } from "./cli-parameter-list-control";

const inheritOption = "__inherit__";
const customOption = "__custom__";

const fieldClassName =
  "min-h-9 w-full rounded-md border border-border bg-background px-3 py-2 text-sm focus-visible:outline-hidden focus-visible:ring-2 focus-visible:ring-ring";

function selectedText(selection: CliParameterSelection): string | undefined {
  return selection.state === "value" && typeof selection.value === "string"
    ? selection.value
    : undefined;
}

function selectedList(selection: CliParameterSelection): string[] {
  return selection.state === "value" && Array.isArray(selection.value) ? selection.value : [];
}

export interface CliParameterControlProps {
  definition: CliParameterDefinition;
  value: CliParameterSelection;
  customMode: boolean;
  customInput: string;
  invalid: boolean;
  disabled: boolean;
  onSelect: (selection: CliParameterSelection) => void;
  onOpenCustom: (seed: string) => void;
  onTypeCustom: (text: string) => void;
}

/**
 * Fully controlled. Inheritance is its own choice, not a value named "default", and choosing Custom
 * only switches the editor — it never writes a value, so opening Custom and changing your mind
 * leaves the profile untouched.
 */
export function CliParameterControl({
  definition,
  value,
  customMode,
  customInput,
  invalid,
  disabled,
  onSelect,
  onOpenCustom,
  onTypeCustom,
}: CliParameterControlProps) {
  const { t } = useTranslation();
  const describedBy = `${definition.id}-help`;

  if (definition.control === "boolean-flag" || definition.control === "tri-state") {
    const checked = value.state === "value" && value.value === true;
    return (
      <Button
        aria-checked={checked}
        aria-label={t(definition.labelKey)}
        disabled={disabled}
        onClick={() => onSelect(checked ? { state: "inherit" } : { state: "value", value: true })}
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
    const selectValue = customMode ? customOption : text === undefined ? inheritOption : isKnown ? text : customOption;
    return (
      <div className="space-y-2">
        <select
          aria-label={t(definition.labelKey)}
          className={fieldClassName}
          disabled={disabled}
          onChange={(event) => {
            const next = event.currentTarget.value;
            if (next === inheritOption) onSelect({ state: "inherit" });
            else if (next === customOption) onOpenCustom(isKnown || text === undefined ? "" : text);
            else onSelect({ state: "value", value: next });
          }}
          value={selectValue}
        >
          <option value={inheritOption}>{t("cliParameters.values.inherit.label")}</option>
          {definition.options.map((option) => (
            <option key={option.value} value={option.value}>
              {t(option.labelKey)}
            </option>
          ))}
          <option value={customOption}>{t("cliParameters.custom.option")}</option>
        </select>
        {selectValue === customOption ? (
          <input
            aria-describedby={describedBy}
            aria-invalid={invalid}
            aria-label={t("cliParameters.custom.placeholder")}
            className={fieldClassName}
            disabled={disabled}
            onChange={(event) => onTypeCustom(event.currentTarget.value)}
            placeholder={t("cliParameters.custom.placeholder")}
            type="text"
            value={customInput}
          />
        ) : (
          <p className="text-xs leading-5 text-muted-foreground" id={describedBy}>
            {t(
              definition.options.find((option) => option.value === text)?.descriptionKey ??
                "cliParameters.values.inherit.description",
            )}
          </p>
        )}
      </div>
    );
  }

  if (definition.control === "multi-enum") {
    const entries = selectedList(value);
    return (
      <select
        aria-label={t(definition.labelKey)}
        className={fieldClassName}
        disabled={disabled}
        multiple
        onChange={(event) => {
          const next = Array.from(event.currentTarget.selectedOptions, (option) => option.value);
          onSelect(next.length === 0 ? { state: "inherit" } : { state: "value", value: next });
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
    return (
      <CliParameterListControl
        definition={definition}
        disabled={disabled}
        entries={selectedList(value)}
        onChange={onSelect}
      />
    );
  }

  const text = selectedText(value);
  return (
    <div className="space-y-2">
      <select
        aria-label={t(definition.labelKey)}
        className={fieldClassName}
        disabled={disabled}
        onChange={(event) => {
          const next = event.currentTarget.value;
          onSelect(next === inheritOption ? { state: "inherit" } : { state: "value", value: next });
        }}
        value={text ?? inheritOption}
      >
        <option value={inheritOption}>{t("cliParameters.values.inherit.label")}</option>
        {definition.options.map((option) => (
          <option key={option.value} value={option.value}>
            {t(option.labelKey)}
          </option>
        ))}
      </select>
      <p className="text-xs leading-5 text-muted-foreground" id={describedBy}>
        {t(
          definition.options.find((option) => option.value === text)?.descriptionKey ??
            "cliParameters.values.inherit.description",
        )}
      </p>
    </div>
  );
}
