import { useTranslation } from "react-i18next";
import { Badge } from "../../../components/ui/badge";
import { Button } from "../../../components/ui/button";
import { useConfirmation } from "../../../components/ui/use-confirmation";
import { formatAppDateTime } from "../../../i18n/format";
import type { AgentService } from "../../../services/agent-service";
import type { MemorySensitivity, MemoryType } from "../../../types/personalization";
import type { MemoryDetail } from "../../../types/personalization-memory";
import { useMemoryDetail } from "./use-memory-detail";

const TYPES: Exclude<MemoryType, "untyped">[] = ["user", "feedback", "project", "reference"];
const SENSITIVITIES: MemorySensitivity[] = ["normal", "sensitive"];

function Row({ children, label }: { children: React.ReactNode; label: string }) {
  return (
    <div className="min-w-0">
      <dt className="text-xs font-medium text-muted-foreground">{label}</dt>
      <dd className="wrap-break-word text-sm">{children}</dd>
    </div>
  );
}

/**
 * One memory in full, with the actions that change it.
 *
 * The body is here and nowhere else: the list deliberately carries none, so this is the only place
 * a stored body is read, and it is read for the one record a user opened.
 */
export function MemoryDetailPanel({
  memoryId,
  onClose,
  service,
}: {
  memoryId: string | null;
  onClose: () => void;
  service: AgentService;
}) {
  const { t, i18n } = useTranslation();
  const { confirm, confirmationDialog } = useConfirmation();
  const detail = useMemoryDetail(service, memoryId);

  if (memoryId === null) {
    return (
      <p className="text-sm text-muted-foreground" data-testid="personalization-detail-empty">
        {t("personalization.detail.empty")}
      </p>
    );
  }
  if (detail.deleted) {
    return (
      <p className="text-sm text-muted-foreground" data-testid="personalization-detail-deleted" role="status">
        {t("personalization.detail.deleted")}
      </p>
    );
  }
  if (detail.loadError) {
    return (
      <p className="text-sm ucd-status-danger" data-testid="personalization-detail-error" role="alert">
        {t("personalization.detail.loadFailed")}
      </p>
    );
  }
  if (detail.isLoading || !detail.record || !detail.draft) {
    return <p className="text-sm text-muted-foreground">{t("personalization.memory.loading")}</p>;
  }

  const record = detail.record;
  const draft = detail.draft;

  async function handleDelete() {
    const confirmed = await confirm({ title: t("personalization.detail.confirmDelete"), tone: "danger" });
    if (confirmed) detail.remove();
  }

  return (
    <div className="flex flex-col gap-4" data-testid="personalization-detail">
      {confirmationDialog}

      {detail.conflict ? (
        <div className="rounded-md border p-3 text-sm ucd-status-warning" data-testid="personalization-detail-conflict" role="alert">
          <p>{t("personalization.detail.conflict")}</p>
          <Button className="mt-2" data-testid="personalization-detail-reload" onClick={detail.reload} size="sm" variant="outline">
            {t("personalization.conflict.reload")}
          </Button>
        </div>
      ) : null}

      {detail.failure ? (
        <p className="text-sm ucd-status-danger" data-testid="personalization-detail-failure" role="alert">
          {t(`personalization.detail.${detail.failure}Failed`)}
        </p>
      ) : null}

      {detail.editing ? (
        <Fields draft={draft} onEdit={detail.edit} saving={detail.isSaving} />
      ) : (
        <ReadOnly language={i18n.language} record={record} />
      )}

      <div className="flex flex-wrap items-center gap-3">
        {detail.editing ? (
          <>
            <Button data-testid="personalization-detail-save" disabled={!detail.isDirty || detail.isSaving || detail.conflict} onClick={detail.save}>
              {t("personalization.editor.save")}
            </Button>
            <Button data-testid="personalization-detail-cancel" disabled={detail.isSaving} onClick={detail.cancelEdit} variant="outline">
              {t("personalization.detail.cancel")}
            </Button>
          </>
        ) : (
          <>
            <Button data-testid="personalization-detail-edit" disabled={detail.isSaving} onClick={detail.beginEdit}>
              {t("personalization.detail.edit")}
            </Button>
            <Button
              data-testid="personalization-detail-status"
              disabled={detail.isSaving}
              onClick={() => detail.setStatus(record.status === "archived" ? "active" : "archived")}
              variant="outline"
            >
              {t(record.status === "archived" ? "personalization.detail.reactivate" : "personalization.detail.archive")}
            </Button>
            <Button data-testid="personalization-detail-delete" disabled={detail.isSaving} onClick={handleDelete} variant="outline">
              {t("personalization.detail.delete")}
            </Button>
          </>
        )}
        <Button className="ml-auto" data-testid="personalization-detail-close" onClick={onClose} size="sm" variant="ghost">
          {t("personalization.detail.close")}
        </Button>
      </div>
    </div>
  );
}

function ReadOnly({ language, record }: { language: string; record: MemoryDetail }) {
  const { t } = useTranslation();
  const stamp = (value: string) =>
    formatAppDateTime(value, language, { dateStyle: "medium", timeStyle: "short" });

  return (
    <>
      <div>
        <h4 className="wrap-break-word text-sm font-semibold">{record.name}</h4>
        <p className="mt-0.5 wrap-break-word text-xs text-muted-foreground">{record.description}</p>
      </div>

      <div>
        <div className="text-xs font-medium text-muted-foreground">{t("personalization.detail.body")}</div>
        <p className="wrap-break-word whitespace-pre-wrap text-sm" data-testid="personalization-detail-body">
          {record.content}
        </p>
      </div>

      <dl className="grid gap-3 sm:grid-cols-2" data-testid="personalization-detail-metadata">
        <Row label={t("personalization.memoryList.filters.type")}>
          {t(`personalization.memory.type.${record.memoryType}`)}
        </Row>
        <Row label={t("personalization.memoryList.filters.status")}>
          <Badge tone={record.status === "archived" ? "muted" : "default"}>
            {t(`personalization.memoryList.status.${record.status}`)}
          </Badge>
        </Row>
        <Row label={t("personalization.scope.title")}>
          {record.workspaceKey
            ? `${t(`personalization.overview.source.${record.scopeKind}`)} (${record.workspaceKey})`
            : t(`personalization.overview.source.${record.scopeKind}`)}
        </Row>
        <Row label={t("personalization.detail.audience")}>
          {/* Null means every Agent. Rendering an empty list instead would read as "no Agent can
              see this", which is the opposite of what it means. */}
          {record.audienceAgentIds === null
            ? t("personalization.detail.audienceAll")
            : record.audienceAgentIds.join(", ")}
        </Row>
        <Row label={t("personalization.detail.sensitivity")}>
          {t(`personalization.detail.sensitivityValue.${record.sensitivity}`)}
        </Row>
        <Row label={t("personalization.inheritance.revision", { revision: record.revision })}>
          {t(`personalization.memoryList.source.${record.source}`)}
        </Row>
        <Row label={t("personalization.detail.recordedBy")}>
          {record.sourceAgentId ?? t("personalization.detail.noSourceAgent")}
        </Row>
        <Row label={t("personalization.detail.created")}>{stamp(record.createdAt)}</Row>
        <Row label={t("personalization.detail.updated")}>{stamp(record.updatedAt)}</Row>
      </dl>
    </>
  );
}

function Fields({
  draft,
  onEdit,
  saving,
}: {
  draft: ReturnType<typeof useMemoryDetail>["draft"] & object;
  onEdit: (patch: Partial<typeof draft>) => void;
  saving: boolean;
}) {
  const { t } = useTranslation();
  return (
    <div className="grid gap-3">
      <label className="flex flex-col gap-1 text-xs font-medium">
        {t("personalization.detail.name")}
        <input
          className="ucd-input h-9 rounded-md px-2 text-sm"
          data-testid="personalization-detail-name"
          disabled={saving}
          onChange={(event) => onEdit({ name: event.target.value })}
          value={draft.name}
        />
      </label>
      <label className="flex flex-col gap-1 text-xs font-medium">
        {t("personalization.detail.description_field")}
        <input
          className="ucd-input h-9 rounded-md px-2 text-sm"
          data-testid="personalization-detail-description"
          disabled={saving}
          onChange={(event) => onEdit({ description: event.target.value })}
          value={draft.description}
        />
      </label>
      <label className="flex flex-col gap-1 text-xs font-medium">
        {t("personalization.detail.body")}
        <textarea
          className="ucd-input min-h-32 rounded-md p-2 text-sm"
          data-testid="personalization-detail-content"
          disabled={saving}
          onChange={(event) => onEdit({ content: event.target.value })}
          value={draft.content}
        />
      </label>
      <div className="grid gap-3 sm:grid-cols-2">
        <label className="flex flex-col gap-1 text-xs font-medium">
          {t("personalization.memoryList.filters.type")}
          <select
            className="ucd-input h-9 rounded-md px-2 text-sm"
            data-testid="personalization-detail-type"
            disabled={saving}
            onChange={(event) => onEdit({ memoryType: event.target.value as (typeof TYPES)[number] })}
            value={draft.memoryType}
          >
            {TYPES.map((type) => (
              <option key={type} value={type}>
                {t(`personalization.memory.type.${type}`)}
              </option>
            ))}
          </select>
        </label>
        <label className="flex flex-col gap-1 text-xs font-medium">
          {t("personalization.detail.sensitivity")}
          <select
            className="ucd-input h-9 rounded-md px-2 text-sm"
            data-testid="personalization-detail-sensitivity"
            disabled={saving}
            onChange={(event) => onEdit({ sensitivity: event.target.value as MemorySensitivity })}
            value={draft.sensitivity}
          >
            {SENSITIVITIES.map((value) => (
              <option key={value} value={value}>
                {t(`personalization.detail.sensitivityValue.${value}`)}
              </option>
            ))}
          </select>
        </label>
      </div>
    </div>
  );
}
