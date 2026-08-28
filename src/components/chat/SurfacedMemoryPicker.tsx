import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import type { AgentService } from "../../services/agent-service";
import type { MemorySummary } from "../../types/personalization-memory";

/**
 * The memories this Agent can read, so one of them can be corrected or forgotten.
 *
 * Scoped by audience rather than by "what this message used": which memories a particular
 * generation surfaced is not on the wire, and listing every memory instead would offer the user
 * records this Agent could never have been influenced by. The audience filter is the closest
 * honest answer, and the search box is what narrows it to the one they recognise.
 */
export function SurfacedMemoryPicker({
  agentId,
  onClose,
  service,
}: {
  agentId: string;
  onClose: () => void;
  service: AgentService;
}) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [text, setText] = useState("");
  const [correcting, setCorrecting] = useState<string | null>(null);
  const [correction, setCorrection] = useState("");
  const [failed, setFailed] = useState(false);

  const listQuery = useQuery({
    queryKey: ["personalization", "memories", { audienceAgentId: agentId, text, limit: 10 }] as const,
    queryFn: () =>
      service.queryPersonalizationMemories({
        audienceAgentId: agentId,
        status: "active",
        text: text || undefined,
        limit: 10,
      }),
  });

  function refresh() {
    void queryClient.invalidateQueries({ queryKey: ["personalization", "memories"] });
  }

  const archiveMutation = useMutation({
    mutationFn: (memory: MemorySummary) =>
      // The revision the row was rendered with, so forgetting cannot land on an edit made since.
      service.updatePersonalizationMemory({
        id: memory.id,
        expectedRevision: memory.revision,
        status: "archived",
      }),
    onSuccess: () => {
      setFailed(false);
      refresh();
    },
    onError: () => setFailed(true),
  });

  const correctMutation = useMutation({
    mutationFn: (input: { memory: MemorySummary; content: string }) =>
      service.updatePersonalizationMemory({
        id: input.memory.id,
        expectedRevision: input.memory.revision,
        content: input.content,
      }),
    onSuccess: () => {
      setCorrecting(null);
      setCorrection("");
      setFailed(false);
      refresh();
    },
    onError: () => setFailed(true),
  });

  const items = listQuery.data?.items ?? [];

  return (
    <div className="rounded-md border border-border p-2" data-testid="message-memory-picker">
      <input
        aria-label={t("chat.memory.searchLabel")}
        className="ucd-input h-8 w-full rounded-md px-2 text-xs"
        data-testid="message-memory-search"
        onChange={(event) => setText(event.target.value)}
        placeholder={t("chat.memory.searchLabel")}
        type="search"
        value={text}
      />

      {failed ? (
        <p className="mt-2 text-xs ucd-status-danger" data-testid="message-memory-picker-failed" role="alert">
          {t("chat.memory.failed")}
        </p>
      ) : null}

      {listQuery.isPending ? (
        <p className="mt-2 text-xs text-muted-foreground">{t("personalization.memory.loading")}</p>
      ) : items.length === 0 ? (
        <p className="mt-2 text-xs text-muted-foreground" data-testid="message-memory-picker-empty">
          {t("chat.memory.noneReadable")}
        </p>
      ) : (
        <ul className="mt-2 grid gap-2">
          {items.map((memory) => (
            <li className="rounded-md border border-border/60 p-2" data-testid={`message-memory-${memory.id}`} key={memory.id}>
              <p className="wrap-break-word text-xs font-medium">{memory.name}</p>
              <p className="wrap-break-word text-xs text-muted-foreground">{memory.description}</p>
              {correcting === memory.id ? (
                <div className="mt-2 flex flex-col gap-2">
                  <textarea
                    aria-label={t("chat.memory.correctionLabel")}
                    className="ucd-input min-h-16 rounded-md p-2 text-xs"
                    data-testid={`message-memory-correction-${memory.id}`}
                    onChange={(event) => setCorrection(event.target.value)}
                    value={correction}
                  />
                  <div className="flex gap-2">
                    <button
                      className="rounded-md border border-border px-2 py-1 text-xs hover:bg-muted"
                      data-testid={`message-memory-correct-save-${memory.id}`}
                      disabled={!correction.trim() || correctMutation.isPending}
                      onClick={() => correctMutation.mutate({ memory, content: correction })}
                      type="button"
                    >
                      {t("chat.memory.saveCorrection")}
                    </button>
                    <button
                      className="rounded-md border border-border px-2 py-1 text-xs hover:bg-muted"
                      data-testid={`message-memory-correct-cancel-${memory.id}`}
                      onClick={() => setCorrecting(null)}
                      type="button"
                    >
                      {t("personalization.detail.cancel")}
                    </button>
                  </div>
                </div>
              ) : (
                <div className="mt-2 flex gap-2">
                  <button
                    className="rounded-md border border-border px-2 py-1 text-xs hover:bg-muted"
                    data-testid={`message-memory-forget-${memory.id}`}
                    disabled={archiveMutation.isPending}
                    onClick={() => archiveMutation.mutate(memory)}
                    type="button"
                  >
                    {t("chat.memory.forget")}
                  </button>
                  <button
                    className="rounded-md border border-border px-2 py-1 text-xs hover:bg-muted"
                    data-testid={`message-memory-correct-${memory.id}`}
                    onClick={() => {
                      setCorrecting(memory.id);
                      setCorrection("");
                    }}
                    type="button"
                  >
                    {t("chat.memory.correct")}
                  </button>
                </div>
              )}
            </li>
          ))}
        </ul>
      )}

      <button
        className="mt-2 rounded-md border border-border px-2 py-1 text-xs hover:bg-muted"
        data-testid="message-memory-picker-close"
        onClick={onClose}
        type="button"
      >
        {t("personalization.detail.close")}
      </button>
    </div>
  );
}
