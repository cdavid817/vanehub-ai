import { useState } from "react";
import { useTranslation } from "react-i18next";
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

  return (
    <div className="fixed inset-0 z-50 grid place-items-center bg-background/70 p-4">
      <form className="ucd-panel grid w-full max-w-xl gap-4 rounded-lg p-4 shadow-xl" onSubmit={(event) => { event.preventDefault(); onSave(form); }}>
        <div>
          <h3 className="text-sm font-semibold">{connection ? t("sshConnections.form.editTitle") : t("sshConnections.form.addTitle")}</h3>
          <p className="mt-1 text-xs text-muted-foreground">{t("sshConnections.form.description")}</p>
        </div>
        <div className="grid gap-3 md:grid-cols-2">
          <Field error={validationErrors.name} label={t("sshConnections.fields.name")} value={form.name} onChange={(value) => update("name", value)} />
          <Field error={validationErrors.host} label={t("sshConnections.fields.host")} value={form.host} onChange={(value) => update("host", value)} />
          <Field error={validationErrors.port} label={t("sshConnections.fields.port")} type="number" value={String(form.port)} onChange={(value) => update("port", Number(value))} />
          <Field error={validationErrors.user} label={t("sshConnections.fields.user")} value={form.user} onChange={(value) => update("user", value)} />
        </div>
        <Field error={validationErrors.defaultPath} label={t("sshConnections.fields.defaultPath")} value={form.defaultPath} onChange={(value) => update("defaultPath", value)} />
        <label className="grid gap-1">
          <span className="text-xs font-medium text-muted-foreground">{t("sshConnections.fields.authMode")}</span>
          <select className="ucd-input h-9 rounded px-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-ring" value={form.authMode} onChange={(event) => update("authMode", event.target.value as SshAuthMode)}>
            <option value="key">{t("sshConnections.auth.key")}</option>
            <option value="password">{t("sshConnections.auth.password")}</option>
          </select>
        </label>
        {form.authMode === "key" ? (
          <Field error={validationErrors.keyPath} label={t("sshConnections.fields.keyPath")} value={form.keyPath ?? ""} onChange={(value) => update("keyPath", value)} />
        ) : (
          <Field error={validationErrors.password} label={connection?.hasPassword ? t("sshConnections.fields.passwordReplace") : t("sshConnections.fields.password")} type="password" value={form.password ?? ""} onChange={(value) => update("password", value)} placeholder={connection?.hasPassword ? t("sshConnections.credentials.configured") : undefined} />
        )}
        <div className="flex justify-end gap-2">
          <Button className="h-8 px-3 text-xs" onClick={onCancel} type="button" variant="outline">{t("sshConnections.cancel")}</Button>
          <Button className="h-8 px-3 text-xs" disabled={invalid || saving} type="submit">{saving ? t("sshConnections.saving") : t("sshConnections.save")}</Button>
        </div>
      </form>
    </div>
  );
}

function Field({ label, value, onChange, type = "text", placeholder, error }: { label: string; value: string; onChange: (value: string) => void; type?: string; placeholder?: string; error?: string }) {
  return (
    <label className="grid gap-1">
      <span className="text-xs font-medium text-muted-foreground">{label}</span>
      <input aria-invalid={Boolean(error)} className="ucd-input h-9 rounded px-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-ring" onChange={(event) => onChange(event.target.value)} placeholder={placeholder} type={type} value={value} />
      {error ? <span className="text-xs text-destructive">{error}</span> : null}
    </label>
  );
}
