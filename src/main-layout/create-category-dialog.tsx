import { useState, type FormEvent } from "react";
import { useTranslation } from "react-i18next";
import { ApplicationDialog } from "../components/ui/application-dialog";
import { Button } from "../components/ui/button";

/**
 * Replaces a `window.prompt` call. The native prompt could not be themed or localized, and it
 * had nowhere to report that the name was empty or that creation failed.
 */
export function CreateCategoryDialog({
  onClose,
  onCreate,
}: {
  onClose: () => void;
  onCreate: (name: string) => Promise<void>;
}) {
  const { t } = useTranslation();
  const [name, setName] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const trimmed = name.trim();
    if (!trimmed) {
      setError(t("layout.categoryNameRequired"));
      return;
    }
    setSaving(true);
    setError(null);
    void onCreate(trimmed)
      .then(onClose)
      .catch((reason: unknown) => {
        setError(reason instanceof Error ? reason.message : String(reason));
        setSaving(false);
      });
  }

  return (
    <ApplicationDialog
      closeDisabled={saving}
      description={t("layout.newCategoryDescription")}
      maxWidth="max-w-sm"
      onClose={onClose}
      title={t("layout.newCategory")}
    >
      <form className="grid gap-3" onSubmit={submit}>
        <label className="grid gap-1">
          <span className="text-xs font-medium text-muted-foreground">{t("layout.categoryName")}</span>
          <input
            className="ucd-input h-9 rounded px-2 text-sm outline-hidden focus-visible:ring-2 focus-visible:ring-ring"
            data-dialog-autofocus
            onChange={(event) => setName(event.target.value)}
            placeholder={t("layout.categoryNamePlaceholder")}
            value={name}
          />
        </label>
        {error ? <p className="wrap-break-word text-xs leading-5 text-destructive" role="alert">{error}</p> : null}
        <div className="flex justify-end gap-2">
          <Button disabled={saving} onClick={onClose} size="sm" type="button" variant="outline">
            {t("layout.cancel")}
          </Button>
          <Button disabled={saving} size="sm" type="submit">
            {t("layout.createCategory")}
          </Button>
        </div>
      </form>
    </ApplicationDialog>
  );
}
