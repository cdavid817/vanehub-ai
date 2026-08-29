import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "../../../components/ui/button";
import type { AgentService } from "../../../services/agent-service";
import type { MemoryScopeKind, MemoryType } from "../../../types/personalization";
import type { CreateMemoryInput } from "../../../types/personalization-memory";
import type { WorkspaceOption } from "./use-scope-options";

const TYPES: Exclude<MemoryType, "untyped">[] = ["user", "feedback", "project", "reference"];

function blank(): CreateMemoryInput {
  return {
    name: "",
    description: "",
    memoryType: "user",
    content: "",
    scopeKind: "global",
  };
}

/**
 * A memory the user writes themselves.
 *
 * Deliberately not a proposal: this path records a person as the author, which is what makes it
 * the one write that reaches active memory without passing through review.
 */
export function MemoryCreateForm({
  onCreated,
  service,
  workspaces,
}: {
  onCreated: () => void;
  service: AgentService;
  workspaces: readonly WorkspaceOption[];
}) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [input, setInput] = useState<CreateMemoryInput>(blank());
  const [failed, setFailed] = useState(false);

  const createMutation = useMutation({
    mutationFn: (value: CreateMemoryInput) => service.createPersonalizationMemory(value),
    onSuccess: () => {
      setInput(blank());
      setFailed(false);
      void queryClient.invalidateQueries({ queryKey: ["personalization", "memories"] });
      void queryClient.invalidateQueries({ queryKey: ["personalization", "overview"] });
      onCreated();
    },
    onError: () => setFailed(true),
  });

  const needsWorkspace = input.scopeKind === "workspace" && !input.workspaceKey;
  // Whitespace is not content. The store refuses it, and refusing here is what stops the user
  // finding that out only after pressing Save.
  const blocked = !input.name.trim() || !input.content.trim() || needsWorkspace || createMutation.isPending;

  function patch(next: Partial<CreateMemoryInput>) {
    setInput((current) => ({ ...current, ...next }));
    setFailed(false);
  }

  return (
    <div className="grid gap-3" data-testid="personalization-create-form">
      <label className="flex flex-col gap-1 text-xs font-medium">
        {t("personalization.detail.name")}
        <input
          className="ucd-input h-9 rounded-md px-2 text-sm"
          data-testid="personalization-create-name"
          onChange={(event) => patch({ name: event.target.value })}
          value={input.name}
        />
      </label>
      <label className="flex flex-col gap-1 text-xs font-medium">
        {t("personalization.detail.description_field")}
        <input
          className="ucd-input h-9 rounded-md px-2 text-sm"
          data-testid="personalization-create-description"
          onChange={(event) => patch({ description: event.target.value })}
          value={input.description}
        />
      </label>
      <label className="flex flex-col gap-1 text-xs font-medium">
        {t("personalization.detail.body")}
        <textarea
          className="ucd-input min-h-24 rounded-md p-2 text-sm"
          data-testid="personalization-create-content"
          onChange={(event) => patch({ content: event.target.value })}
          value={input.content}
        />
      </label>

      <div className="grid gap-3 sm:grid-cols-2">
        <label className="flex flex-col gap-1 text-xs font-medium">
          {t("personalization.memoryList.filters.type")}
          <select
            className="ucd-input h-9 rounded-md px-2 text-sm"
            data-testid="personalization-create-type"
            onChange={(event) => patch({ memoryType: event.target.value as (typeof TYPES)[number] })}
            value={input.memoryType}
          >
            {TYPES.map((type) => (
              <option key={type} value={type}>
                {t(`personalization.memory.type.${type}`)}
              </option>
            ))}
          </select>
        </label>
        <label className="flex flex-col gap-1 text-xs font-medium">
          {t("personalization.scope.title")}
          <select
            className="ucd-input h-9 rounded-md px-2 text-sm"
            data-testid="personalization-create-scope"
            onChange={(event) =>
              patch({
                scopeKind: event.target.value as MemoryScopeKind,
                workspaceKey:
                  event.target.value === "workspace" ? workspaces[0]?.workspaceKey : undefined,
              })
            }
            value={input.scopeKind}
          >
            <option value="global">{t("personalization.overview.source.global")}</option>
            <option value="workspace">{t("personalization.overview.source.workspace")}</option>
          </select>
        </label>
      </div>

      {input.scopeKind === "workspace" ? (
        <label className="flex flex-col gap-1 text-xs font-medium">
          {t("personalization.scope.workspace")}
          <select
            className="ucd-input h-9 rounded-md px-2 text-sm"
            data-testid="personalization-create-workspace"
            onChange={(event) => patch({ workspaceKey: event.target.value || undefined })}
            value={input.workspaceKey ?? ""}
          >
            <option value="">{t("personalization.scope.chooseWorkspace")}</option>
            {workspaces.map((workspace) => (
              <option key={workspace.workspaceKey} value={workspace.workspaceKey}>
                {workspace.displayName}
              </option>
            ))}
          </select>
        </label>
      ) : null}

      {failed ? (
        <p className="text-sm ucd-status-danger" data-testid="personalization-create-error" role="alert">
          {t("personalization.create.failed")}
        </p>
      ) : null}

      <div className="flex gap-3">
        <Button
          data-testid="personalization-create-save"
          disabled={blocked}
          onClick={() => createMutation.mutate(input)}
        >
          {t("personalization.create.save")}
        </Button>
        <Button data-testid="personalization-create-cancel" onClick={onCreated} variant="outline">
          {t("personalization.detail.cancel")}
        </Button>
      </div>
    </div>
  );
}
