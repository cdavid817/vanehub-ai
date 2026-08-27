import { ArrowDown, ArrowUp, FolderOpen, Plus, X } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "../../components/ui/button";
import { agentService } from "../../services/runtime-agent-client";
import type { CliParameterDefinition, CliParameterSelection } from "../../types/cli-parameter";

const fieldClassName =
  "min-h-9 w-full rounded-md border border-border bg-background px-3 py-2 text-sm focus-visible:outline-hidden focus-visible:ring-2 focus-visible:ring-ring";

export interface CliParameterListControlProps {
  definition: CliParameterDefinition;
  entries: readonly string[];
  disabled: boolean;
  onChange: (selection: CliParameterSelection) => void;
}

/**
 * Ordered list editor with keyboard-operable reorder and remove.
 *
 * Drag handles would leave keyboard users without a way to reorder at all, so every action here is
 * a button. Ordering matters for these parameters — a fallback model list is tried in order — which
 * is why reordering exists rather than a plain set editor.
 */
export function CliParameterListControl({
  definition,
  entries,
  disabled,
  onChange,
}: CliParameterListControlProps) {
  const { t } = useTranslation();
  const [draft, setDraft] = useState("");
  const isPathList = definition.control === "path-list";

  function commit(next: readonly string[]) {
    const cleaned = next.map((entry) => entry.trim()).filter((entry) => entry.length > 0);
    onChange(cleaned.length === 0 ? { state: "inherit" } : { state: "value", value: cleaned });
  }

  function move(index: number, delta: number) {
    const next = [...entries];
    const target = index + delta;
    if (target < 0 || target >= next.length) return;
    [next[index], next[target]] = [next[target], next[index]];
    commit(next);
  }

  async function addDirectory() {
    const selected = await agentService.selectProjectDirectory();
    if (selected) commit([...entries, selected]);
  }

  return (
    <div className="space-y-2">
      <ul aria-label={t(definition.labelKey)} className="space-y-1">
        {entries.map((entry, index) => (
          <li className="flex items-center gap-1" key={`${entry}-${index}`}>
            <code className="min-w-0 flex-1 truncate rounded border border-border bg-muted px-2 py-1 text-xs">
              {entry}
            </code>
            <Button
              aria-label={t("cliParameters.list.moveUp", { entry })}
              disabled={disabled || index === 0}
              onClick={() => move(index, -1)}
              size="icon"
              type="button"
              variant="ghost"
            >
              <ArrowUp aria-hidden="true" />
            </Button>
            <Button
              aria-label={t("cliParameters.list.moveDown", { entry })}
              disabled={disabled || index === entries.length - 1}
              onClick={() => move(index, 1)}
              size="icon"
              type="button"
              variant="ghost"
            >
              <ArrowDown aria-hidden="true" />
            </Button>
            <Button
              aria-label={t("cliParameters.list.remove", { entry })}
              disabled={disabled}
              onClick={() => commit(entries.filter((_, position) => position !== index))}
              size="icon"
              type="button"
              variant="ghost"
            >
              <X aria-hidden="true" />
            </Button>
          </li>
        ))}
      </ul>
      <div className="flex items-center gap-1">
        <input
          aria-label={t("cliParameters.list.placeholder")}
          className={fieldClassName}
          disabled={disabled}
          onChange={(event) => setDraft(event.currentTarget.value)}
          onKeyDown={(event) => {
            if (event.key !== "Enter" || draft.trim().length === 0) return;
            event.preventDefault();
            commit([...entries, draft]);
            setDraft("");
          }}
          placeholder={t("cliParameters.list.placeholder")}
          type="text"
          value={draft}
        />
        <Button
          aria-label={t("cliParameters.list.add")}
          disabled={disabled || draft.trim().length === 0}
          onClick={() => {
            commit([...entries, draft]);
            setDraft("");
          }}
          size="icon"
          type="button"
          variant="outline"
        >
          <Plus aria-hidden="true" />
        </Button>
        {isPathList ? (
          <Button
            disabled={disabled}
            onClick={() => void addDirectory()}
            size="sm"
            type="button"
            variant="outline"
          >
            <FolderOpen aria-hidden="true" /> {t("cliParameters.list.addDirectory")}
          </Button>
        ) : null}
      </div>
    </div>
  );
}
