import { useState } from "react";
import { useTranslation } from "react-i18next";
import { ApplicationDialog } from "../../../components/ui/application-dialog";
import { Button } from "../../../components/ui/button";
import type { SaveSshConnectionInput, SshAuthMode, SshConnection } from "../../../types/ssh-connection";

const emptyForm: SaveSshConnectionInput = {
  name: "",
  host: "",
  port: 22,
  user: "",
  defaultPath: "",
  authMode: "key",
  keyPath: "",
  password: "",
};

export function SshConnectionForm({
  connection,
  saving,
  onCancel,
  onSave,
}: {
  connection: SshConnection | null;
  saving: boolean;
  onCancel: () => void;
  onSave: (input: SaveSshConnectionInput) => void;
}) {
  const { t } = useTranslation();
  const [form, setForm] = useState<SaveSshConnectionInput>(() =>
    connection
      ? {
          name: connection.name,
          host: connection.host,
          port: connection.port,
          user: connection.user,
          defaultPath: connection.defaultPath,
          authMode: connection.authMode,
          keyPath: connection.keyPath ?? "",
          password: "",
        }
      : emptyForm,
  );
  const [submitted, setSubmitted] = useState(false);
  const validationErrors = {
    name: form.name.trim() ? "" : t("sshConnections.validation.name"),
    host: form.host.trim() ? "" : t("sshConnections.validation.host"),
    port:
      Number.isInteger(form.port) && form.port >= 1 && form.port <= 65535
        ? ""
        : t("sshConnections.validation.port"),
    user: form.user.trim() ? "" : t("sshConnections.validation.user"),
    defaultPath: form.defaultPath.trim() ? "" : t("sshConnections.validation.defaultPath"),
    keyPath:
      form.authMode !== "key" || form.keyPath?.trim()
        ? ""
        : t("sshConnections.validation.keyPath"),
    password:
      form.authMode !== "password" || connection?.hasPassword || form.password?.trim()
        ? ""
        : t("sshConnections.validation.password"),
  };
  const invalid = Object.values(validationErrors).some(Boolean);

  function update(field: keyof SaveSshConnectionInput, value: string | number | SshAuthMode) {
    setForm((current) => ({ ...current, [field]: value }));
  }

  function showError(field: keyof typeof validationErrors) {
    return submitted ? validationErrors[field] : "";
  }

  return (
    <ApplicationDialog
      closeDisabled={saving}
      description={t("sshConnections.form.description")}
      maxWidth="max-w-xl"
      onClose={onCancel}
      title={connection ? t("sshConnections.form.editTitle") : t("sshConnections.form.addTitle")}
    >
      <form className="grid gap-4" onSubmit={(event) => { event.preventDefault(); setSubmitted(true); if (!invalid) onSave(form); }}>
        <div className="grid gap-3 md:grid-cols-2">
          <Field autoFocus error={showError("name")} label={t("sshConnections.fields.name")} value={form.name} onChange={(value) => update("name", value)} />
          <Field error={showError("host")} label={t("sshConnections.fields.host")} value={form.host} onChange={(value) => update("host", value)} />
          <Field error={showError("port")} label={t("sshConnections.fields.port")} type="number" value={String(form.port)} onChange={(value) => update("port", Number(value))} />
          <Field error={showError("user")} label={t("sshConnections.fields.user")} value={form.user} onChange={(value) => update("user", value)} />
        </div>
        <Field error={showError("defaultPath")} label={t("sshConnections.fields.defaultPath")} value={form.defaultPath} onChange={(value) => update("defaultPath", value)} />
        <label className="grid gap-1">
          <span className="text-xs font-medium text-muted-foreground">{t("sshConnections.fields.authMode")}</span>
          <select className="ucd-input h-9 rounded px-2 text-sm outline-hidden focus-visible:ring-2 focus-visible:ring-ring" value={form.authMode} onChange={(event) => update("authMode", event.target.value as SshAuthMode)}>
            <option value="key">{t("sshConnections.auth.key")}</option>
            <option value="password">{t("sshConnections.auth.password")}</option>
          </select>
        </label>
        {form.authMode === "key" ? (
          <Field error={showError("keyPath")} label={t("sshConnections.fields.keyPath")} value={form.keyPath ?? ""} onChange={(value) => update("keyPath", value)} />
        ) : (
          <Field error={showError("password")} label={connection?.hasPassword ? t("sshConnections.fields.passwordReplace") : t("sshConnections.fields.password")} type="password" value={form.password ?? ""} onChange={(value) => update("password", value)} placeholder={connection?.hasPassword ? t("sshConnections.credentials.configured") : undefined} />
        )}
        <div className="flex justify-end gap-2">
          <Button className="h-8 px-3 text-xs" disabled={saving} onClick={onCancel} type="button" variant="outline">{t("sshConnections.cancel")}</Button>
          <Button className="h-8 px-3 text-xs" disabled={(submitted && invalid) || saving} type="submit">{saving ? t("sshConnections.saving") : t("sshConnections.save")}</Button>
        </div>
      </form>
    </ApplicationDialog>
  );
}

function Field({ label, value, onChange, type = "text", placeholder, error, autoFocus }: { label: string; value: string; onChange: (value: string) => void; type?: string; placeholder?: string; error?: string; autoFocus?: boolean }) {
  return (
    <label className="grid gap-1">
      <span className="text-xs font-medium text-muted-foreground">{label}</span>
      <input aria-invalid={Boolean(error)} className="ucd-input h-9 rounded px-2 text-sm outline-hidden focus-visible:ring-2 focus-visible:ring-ring" data-dialog-autofocus={autoFocus || undefined} onChange={(event) => onChange(event.target.value)} placeholder={placeholder} type={type} value={value} />
      {error ? <span className="text-xs text-destructive">{error}</span> : null}
    </label>
  );
}
