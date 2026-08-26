import { keepPreviousData, useQuery } from "@tanstack/react-query";
import { Database, LoaderCircle } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Badge } from "../../../components/ui/badge";
import { Button } from "../../../components/ui/button";
import { formatAppDateTime } from "../../../i18n/format";
import type { AgentService } from "../../../services/agent-service";
import { agentService as defaultAgentService } from "../../../services/runtime-agent-client";
import type { MemoryQuery, MemorySummary } from "../../../types/personalization-memory";
import { SectionPanel } from "../page-parts";
import { MemoryCreateForm } from "./memory-create-form";
import { MemoryDetailPanel } from "./memory-detail-panel";
import { MemoryResetDialog } from "./memory-reset-dialog";
import { MemoryFilters } from "./memory-filters";
import { useScopeOptions } from "./use-scope-options";

const PAGE_SIZE = 25;

/**
 * A page of summaries, never a list of bodies.
 *
 * The list this replaces read every memory in full to render a column of names, so the cost of
 * opening the page grew with everything the user had ever saved. A summary carries no body at all;
 * the detail call exists for the one row a user opens.
 */
export function MemoryListSection({ service = defaultAgentService }: { service?: AgentService }) {
  const { t, i18n } = useTranslation();
  const [query, setQuery] = useState<MemoryQuery>({ limit: PAGE_SIZE });
  const [pageStack, setPageStack] = useState<(string | undefined)[]>([undefined]);
  const [openId, setOpenId] = useState<string | null>(null);
  const [creating, setCreating] = useState(false);
  const [resetting, setResetting] = useState(false);
  const { agents, workspaces } = useScopeOptions(service);

  const cursor = pageStack[pageStack.length - 1];
  const pageQuery = useQuery({
    queryKey: ["personalization", "memories", { ...query, cursor }] as const,
    queryFn: () => service.queryPersonalizationMemories({ ...query, cursor }),
    // The previous page stays on screen while the next one loads. Blanking the list on every
    // keystroke of the search box makes the page flicker through empty states the user's data
    // never had.
    placeholderData: keepPreviousData,
  });

  const page = pageQuery.data;
  const items = page?.items ?? [];
  // `isPlaceholderData` is the honest signal: it is exactly the window in which the rows on screen
  // answer a query the user is no longer asking. Showing stale rows without saying so is its own
  // failure -- the user reads them as the result of the filter they just set.
  const refreshing = pageQuery.isPlaceholderData || (pageQuery.isFetching && items.length > 0);

  function applyFilter(patch: Omit<MemoryQuery, "cursor">) {
    setQuery((current) => ({ ...current, ...patch }));
    // A cursor names a position in one filtered ordering; carrying it into another resumes from a
    // row that is no longer in the set, which reads as a page of missing results.
    setPageStack([undefined]);
  }

  return (
    <SectionPanel
      description={t("personalization.memoryList.description")}
      icon={Database}
      title={t("personalization.memoryList.title")}
    >
      <div className="mb-3 flex justify-end gap-2">
        <Button data-testid="personalization-create-open" onClick={() => setCreating((open) => !open)} size="sm">
          {t("personalization.create.open")}
        </Button>
        <Button data-testid="personalization-reset-open" onClick={() => setResetting(true)} size="sm" variant="outline">
          {t("personalization.reset.open")}
        </Button>
      </div>
      {resetting ? (
        <MemoryResetDialog onClose={() => setResetting(false)} service={service} workspaces={workspaces} />
      ) : null}
      {creating ? (
        <div className="mb-4 rounded-md border border-border/70 p-3">
          <h4 className="mb-1 text-sm font-semibold">{t("personalization.create.title")}</h4>
          <p className="mb-3 text-xs text-muted-foreground">{t("personalization.create.description")}</p>
          <MemoryCreateForm onCreated={() => setCreating(false)} service={service} workspaces={workspaces} />
        </div>
      ) : null}

      <MemoryFilters agents={agents} onChange={applyFilter} query={query} workspaces={workspaces} />

      <p
        aria-live="polite"
        className="mt-3 min-h-4 text-xs text-muted-foreground"
        data-testid="personalization-memory-refresh-status"
      >
        {refreshing ? t("personalization.memoryList.refreshing") : ""}
      </p>

      <div className="mt-1">
        {pageQuery.error ? (
          <p className="rounded-md border p-3 text-sm ucd-status-danger" data-testid="personalization-memory-error" role="alert">
            {t("personalization.memoryList.loadFailed")}
          </p>
        ) : pageQuery.isPending ? (
          <p className="flex items-center gap-2 text-sm text-muted-foreground">
            <LoaderCircle className="h-4 w-4 animate-spin" />
            {t("personalization.memory.loading")}
          </p>
        ) : items.length === 0 ? (
          <p className="text-sm text-muted-foreground" data-testid="personalization-memory-empty">
            {t("personalization.memoryList.noMatches")}
          </p>
        ) : (
          <ul className="grid gap-2" data-testid="personalization-memory-list">
            {items.map((memory) => (
              <MemoryRow
                key={memory.id}
                language={i18n.language}
                memory={memory}
                onOpen={() => setOpenId(memory.id)}
                open={openId === memory.id}
              />
            ))}
          </ul>
        )}
      </div>

      <div className="mt-4 flex flex-wrap items-center gap-3">
        <Button
          data-testid="personalization-memory-previous"
          disabled={pageStack.length === 1}
          onClick={() => setPageStack((stack) => stack.slice(0, -1))}
          size="sm"
          variant="outline"
        >
          {t("personalization.memoryList.previous")}
        </Button>
        <Button
          data-testid="personalization-memory-next"
          disabled={!page?.nextCursor}
          onClick={() => setPageStack((stack) => [...stack, page?.nextCursor ?? undefined])}
          size="sm"
          variant="outline"
        >
          {t("personalization.memoryList.next")}
        </Button>
        <span className="text-xs text-muted-foreground" data-testid="personalization-memory-count">
          {/* `totalMatched` is present only when the store can produce it cheaply, so the shown
              count falls back to this page rather than claiming a total nobody counted. */}
          {page?.totalMatched === null || page?.totalMatched === undefined
            ? t("personalization.memoryList.shown", { count: items.length })
            : t("personalization.memoryList.matched", { count: page.totalMatched })}
        </span>
      </div>

      <div
        aria-label={t("personalization.detail.title")}
        className="mt-5 border-t border-border/70 pt-4"
        role="region"
      >
        <h4 className="mb-3 text-sm font-semibold">{t("personalization.detail.title")}</h4>
        <MemoryDetailPanel memoryId={openId} onClose={() => setOpenId(null)} service={service} />
      </div>
    </SectionPanel>
  );
}

function MemoryRow({
  language,
  memory,
  onOpen,
  open,
}: {
  language: string;
  memory: MemorySummary;
  onOpen: () => void;
  open: boolean;
}) {
  const { t } = useTranslation();
  return (
    <li
      className={`ucd-panel rounded-md ${open ? "ring-1 ring-primary" : ""}`}
      data-testid={`personalization-memory-row-${memory.id}`}
    >
      {/* The whole row opens it. A row that carried a separate "open" control would make the
          obvious click do nothing, and a summary has no other action of its own. */}
      <button
        aria-expanded={open}
        className="w-full p-3 text-left"
        data-testid={`personalization-memory-open-${memory.id}`}
        onClick={onOpen}
        type="button"
      >
      <p className="wrap-break-word text-sm font-medium leading-5">{memory.name}</p>
      <p className="mt-0.5 wrap-break-word text-xs text-muted-foreground">{memory.description}</p>
      <div className="mt-2 flex flex-wrap items-center gap-2 text-xs text-muted-foreground">
        <Badge tone={memory.status === "archived" ? "muted" : "default"}>
          {t(`personalization.memoryList.status.${memory.status}`)}
        </Badge>
        <Badge tone="muted">{t(`personalization.memory.type.${memory.memoryType}`)}</Badge>
        <Badge tone="muted">{t(`personalization.overview.source.${memory.scopeKind}`)}</Badge>
        {memory.sensitivity === "restricted" ? (
          <Badge tone="warning">{t("personalization.memoryList.restricted")}</Badge>
        ) : null}
        <span>{t(`personalization.memoryList.source.${memory.source}`)}</span>
        <span>{formatAppDateTime(memory.updatedAt, language, { dateStyle: "medium", timeStyle: "short" })}</span>
      </div>
      </button>
    </li>
  );
}
