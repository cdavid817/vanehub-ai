import { useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Download, PackageOpen, Upload } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Badge } from "../components/ui/badge";
import { Button } from "../components/ui/button";
import type { AgentService } from "../services/agent-service";
import { agentService as defaultAgentService } from "../services/runtime-agent-client";

export function ArtifactPanel({
  service = defaultAgentService,
  sessionId,
}: {
  service?: AgentService;
  sessionId: string;
}) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [acknowledged, setAcknowledged] = useState(false);
  const [notice, setNotice] = useState<string | null>(null);
  const artifacts = useQuery({
    queryKey: ["sessions", sessionId, "artifacts"],
    queryFn: () => service.listArtifacts({ sessionId, limit: 50 }),
  });
  const detail = useQuery({
    enabled: Boolean(selectedId),
    queryKey: ["artifacts", selectedId],
    queryFn: () => service.getArtifact(selectedId ?? ""),
  });
  const preview = useQuery({
    enabled: Boolean(detail.data && isTextMedia(detail.data.mediaType)),
    queryKey: ["artifacts", selectedId, "preview"],
    queryFn: () => service.readArtifact({ artifactId: selectedId ?? "", offset: 0, length: 65_536 }),
  });
  const publication = useMutation({
    mutationFn: () => service.publishArtifact({
      artifactId: detail.data?.id ?? "",
      expectedContentHash: detail.data?.contentHash ?? "",
      acknowledgement: acknowledged,
    }),
    onSuccess: (published) => {
      queryClient.setQueryData(["artifacts", published.id], published);
      setAcknowledged(false);
      setNotice(t("sessionTabs.artifacts.published"));
    },
  });
  const download = useMutation({
    mutationFn: () => service.downloadArtifact({
      artifactId: detail.data?.id ?? "",
      expectedContentHash: detail.data?.contentHash ?? "",
    }),
    onSuccess: () => setNotice(t("sessionTabs.artifacts.downloaded")),
  });

  if (!artifacts.isLoading && !artifacts.isError && !artifacts.data?.items.length) return null;

  return (
    <section aria-labelledby="artifact-panel-heading" className="mb-4 rounded-lg border border-border bg-background p-3">
      <h3 className="flex items-center gap-2 text-sm font-semibold" id="artifact-panel-heading">
        <PackageOpen aria-hidden="true" className="h-4 w-4 text-primary" />
        {t("sessionTabs.artifacts.title")}
      </h3>
      {artifacts.isLoading ? <p aria-live="polite" className="mt-2 text-xs text-muted-foreground">{t("sessionTabs.artifacts.loading")}</p> : null}
      {artifacts.isError ? <p className="mt-2 text-xs text-destructive" role="alert">{t("sessionTabs.artifacts.loadError")}</p> : null}
      {artifacts.data?.items.length ? (
        <div className="mt-3 grid min-h-0 gap-3 lg:grid-cols-[minmax(12rem,0.7fr)_minmax(0,1.3fr)]">
          <ul aria-label={t("sessionTabs.artifacts.list")} className="max-h-72 space-y-1 overflow-y-auto">
            {artifacts.data.items.map((artifact) => (
              <li key={artifact.id}>
                <button
                  aria-pressed={selectedId === artifact.id}
                  className="w-full rounded-md border border-border/70 p-2 text-left hover:bg-muted focus-visible:outline-hidden focus-visible:ring-2 focus-visible:ring-ring"
                  onClick={() => {
                    setSelectedId(artifact.id);
                    setAcknowledged(false);
                    setNotice(null);
                  }}
                  type="button"
                >
                  <span className="block truncate text-sm font-medium">{artifact.displayName}</span>
                  <span className="mt-1 flex items-center justify-between gap-2 text-xs text-muted-foreground">
                    <span>{artifact.mediaType}</span>
                    <Badge tone={artifact.integrity === "verified" ? "success" : "warning"}>
                      {t(`sessionTabs.artifacts.integrity.${artifact.integrity}`)}
                    </Badge>
                  </span>
                </button>
              </li>
            ))}
          </ul>
          <ArtifactDetail
            acknowledged={acknowledged}
            detail={detail.data}
            downloadPending={download.isPending}
            error={detail.isError || preview.isError || publication.isError || download.isError}
            notice={notice}
            onAcknowledged={setAcknowledged}
            onDownload={() => download.mutate()}
            onPublish={() => publication.mutate()}
            preview={preview.data ? decodeBase64Utf8(preview.data.bytesBase64) : null}
            publicationPending={publication.isPending}
          />
        </div>
      ) : null}
    </section>
  );
}

function ArtifactDetail({
  acknowledged,
  detail,
  downloadPending,
  error,
  notice,
  onAcknowledged,
  onDownload,
  onPublish,
  preview,
  publicationPending,
}: {
  acknowledged: boolean;
  detail: Awaited<ReturnType<AgentService["getArtifact"]>> | undefined;
  downloadPending: boolean;
  error: boolean;
  notice: string | null;
  onAcknowledged: (value: boolean) => void;
  onDownload: () => void;
  onPublish: () => void;
  preview: string | null;
  publicationPending: boolean;
}) {
  const { t } = useTranslation();
  if (!detail) return <p className="text-sm text-muted-foreground">{t("sessionTabs.artifacts.select")}</p>;
  return (
    <article aria-label={detail.displayName} className="min-w-0 rounded-md border border-border/70 p-3">
      <div className="flex flex-wrap items-center gap-2">
        <h4 className="min-w-0 flex-1 truncate text-sm font-semibold">{detail.displayName}</h4>
        <Badge tone={detail.integrity === "verified" ? "success" : "warning"}>
          {t(`sessionTabs.artifacts.integrity.${detail.integrity}`)}
        </Badge>
      </div>
      {detail.integrity !== "verified" ? <p className="mt-2 text-xs text-destructive" role="alert">{t("sessionTabs.artifacts.integrityWarning")}</p> : null}
      <dl className="mt-3 grid gap-2 text-xs sm:grid-cols-2">
        <Metadata label={t("sessionTabs.artifacts.hash")} value={detail.contentHash} />
        <Metadata label={t("sessionTabs.artifacts.expiry")} value={detail.expiresAt ?? t("sessionTabs.artifacts.noExpiry")} />
        <Metadata label={t("sessionTabs.artifacts.provenance")} value={detail.provenance.join(" · ")} />
        <Metadata label={t("sessionTabs.artifacts.publication")} value={detail.publishedAt ? t("sessionTabs.artifacts.isPublished") : t("sessionTabs.artifacts.private")} />
      </dl>
      {preview !== null ? <pre className="mt-3 max-h-48 overflow-auto whitespace-pre-wrap break-words rounded-md bg-muted p-2 text-xs">{preview}</pre> : null}
      <label className="mt-3 flex items-start gap-2 text-xs text-muted-foreground">
        <input checked={acknowledged} className="mt-0.5" onChange={(event) => onAcknowledged(event.target.checked)} type="checkbox" />
        <span>{t("sessionTabs.artifacts.publishAcknowledgement")}</span>
      </label>
      <div className="mt-3 flex flex-wrap gap-2">
        <Button disabled={!acknowledged || publicationPending || detail.integrity !== "verified"} onClick={onPublish} size="sm">
          <Upload aria-hidden="true" className="h-3.5 w-3.5" />{t("sessionTabs.artifacts.publish")}
        </Button>
        <Button disabled={downloadPending || detail.integrity !== "verified"} onClick={onDownload} size="sm" variant="outline">
          <Download aria-hidden="true" className="h-3.5 w-3.5" />{t("sessionTabs.artifacts.download")}
        </Button>
      </div>
      {error ? <p className="mt-2 text-xs text-destructive" role="alert">{t("sessionTabs.artifacts.safeError")}</p> : null}
      {notice ? <p className="mt-2 text-xs text-primary" role="status">{notice}</p> : null}
    </article>
  );
}

function Metadata({ label, value }: { label: string; value: string }) {
  return <div className="min-w-0"><dt className="text-muted-foreground">{label}</dt><dd className="break-all font-mono">{value}</dd></div>;
}

function isTextMedia(mediaType: string) {
  return mediaType.startsWith("text/") || mediaType === "application/json";
}

function decodeBase64Utf8(value: string) {
  try {
    const bytes = Uint8Array.from(atob(value), (character) => character.charCodeAt(0));
    return new TextDecoder().decode(bytes);
  } catch {
    return "";
  }
}
