import { useTranslation } from "react-i18next";
import { ApplicationDialog } from "../../../components/ui/application-dialog";
import { Button } from "../../../components/ui/button";
import { normalizeDisplayPath } from "../../../lib/session-path";
import type { ExtensionInstallPreview } from "../../../types/extension";
import { TagList } from "../page-parts";

export function ExtensionInstallPreviewDialog({
  nativeAvailable,
  onClose,
  onInstall,
  preview,
}: {
  nativeAvailable: boolean;
  onClose: () => void;
  onInstall: () => void;
  preview: ExtensionInstallPreview;
}) {
  const { t } = useTranslation();
  return (
    <ApplicationDialog description={t("extensions.preview.description")} maxWidth="max-w-xl" onClose={onClose} title={t("extensions.preview.title")}>
      <dl className="grid gap-3 text-sm md:grid-cols-2">
        <div>
          <dt className="text-muted-foreground">{t("extensions.preview.path")}</dt>
          <dd className="break-all font-medium">{normalizeDisplayPath(preview.installPath)}</dd>
        </div>
        <div>
          <dt className="text-muted-foreground">{t("extensions.preview.download")}</dt>
          <dd className="font-medium">~{preview.estimatedDownloadMb} MB</dd>
        </div>
        <div>
          <dt className="text-muted-foreground">{t("extensions.preview.disk")}</dt>
          <dd className="font-medium">~{preview.estimatedDiskMb} MB</dd>
        </div>
        <div>
          <dt className="text-muted-foreground">{t("extensions.preview.network")}</dt>
          <dd className="font-medium">{t("extensions.preview.installOnly")}</dd>
        </div>
      </dl>
      <div className="mt-4">
        <TagList tags={preview.packages} />
      </div>
      {preview.reason ? <div className="mt-4 rounded border p-3 text-sm ucd-status-warning">{t(preview.reason)}</div> : null}
      <div className="mt-5 flex justify-end gap-2">
        <Button onClick={onClose} variant="outline">{t("extensions.action.cancel")}</Button>
        <Button disabled={!nativeAvailable || !preview.supported} onClick={onInstall}>{t("extensions.action.confirmInstall")}</Button>
      </div>
    </ApplicationDialog>
  );
}
