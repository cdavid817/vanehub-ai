import { Trash2 } from "lucide-react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { Badge } from "../../../components/ui/badge";
import { Button } from "../../../components/ui/button";
import { formatAppDateTime } from "../../../i18n/format";
import type { AgentService } from "../../../services/agent-service";
import { agentService as defaultAgentService } from "../../../services/runtime-agent-client";
import type { AgentMemory } from "../../../types/agent";
import { SectionPanel } from "../page-parts";

const agentMemoriesQueryKey = (agentId: string) => ["agents", "memories", agentId] as const;

export function AgentMemoryPanel({ agentId, service = defaultAgentService }: { agentId: string | null; service?: AgentService }) {
  const { i18n, t } = useTranslation();
  const queryClient = useQueryClient();

  const memoriesQuery = useQuery({
    queryKey: agentMemoriesQueryKey(agentId ?? ""),
    queryFn: () => service.listAgentMemories(agentId as string),
    enabled: agentId != null,
  });

  const deleteMutation = useMutation({
    mutationFn: (memoryId: string) => service.deleteAgentMemory(memoryId),
    onSuccess: () => (agentId ? queryClient.invalidateQueries({ queryKey: agentMemoriesQueryKey(agentId) }) : undefined),
  });

  async function handleDelete(memory: AgentMemory) {
    if (!window.confirm(t("agents.memory.confirm.delete"))) return;
    await deleteMutation.mutateAsync(memory.id);
  }

  const memories = memoriesQuery.data ?? [];

  return (
    <SectionPanel description={t("agents.memory.description")} title={t("agents.memory.title")}>
      {!agentId ? (
        <p className="text-sm text-muted-foreground">{t("agents.memory.noAgentSelected")}</p>
      ) : memories.length === 0 ? (
        <p className="text-sm text-muted-foreground">{t("agents.memory.empty")}</p>
      ) : (
        <ul className="grid gap-2">
          {memories.map((memory) => (
            <li className="ucd-panel rounded-md p-3" key={memory.id}>
              <div className="flex items-start justify-between gap-3">
                <p className="min-w-0 flex-1 wrap-break-word text-sm">{memory.content}</p>
                <Button
                  className="h-7 shrink-0 px-2 text-xs"
                  disabled={deleteMutation.isPending}
                  onClick={() => void handleDelete(memory)}
                  type="button"
                  variant="outline"
                >
                  <Trash2 className="h-3.5 w-3.5" aria-hidden="true" />
                  {t("agents.memory.delete")}
                </Button>
              </div>
              <div className="mt-2 flex flex-wrap items-center gap-2 text-xs text-muted-foreground">
                <Badge tone={memory.source === "automatic" ? "muted" : "default"}>{t(`agents.memory.source.${memory.source}`)}</Badge>
                <span>{memory.folder ?? t("agents.memory.folder.global")}</span>
                <span>{formatAppDateTime(memory.createdAt, i18n.language, { dateStyle: "short", timeStyle: "medium" })}</span>
              </div>
            </li>
          ))}
        </ul>
      )}
    </SectionPanel>
  );
}
