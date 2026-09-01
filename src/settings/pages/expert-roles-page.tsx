import { Copy, Pencil, Plus, Trash2, UserRound } from "lucide-react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "../../components/ui/button";
import { useConfirmation } from "../../components/ui/use-confirmation";
import { agentService } from "../../services/runtime-agent-client";
import type { SettingsPageStatus } from "../settings-page-types";
import type { ExpertRole, SaveExpertRoleInput } from "../../types/expert-role";
import { PageHeader, SectionPanel } from "./page-parts";
import { ExpertRoleForm } from "./expert-roles/expert-role-form";

const expertRolesQueryKey = ["expert-roles"] as const;

export function ExpertRolesPage({
  onStatusChange,
  searchTerm,
}: {
  onStatusChange?: (status: SettingsPageStatus | null) => void;
  searchTerm: string;
}) {
  const { t } = useTranslation();
  const { confirm, confirmationDialog } = useConfirmation();
  const queryClient = useQueryClient();
  /** `undefined` closes the form; `null` opens it blank; a role opens it seeded. */
  const [editing, setEditing] = useState<ExpertRole | null | undefined>();
  const [notice, setNotice] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const rolesQuery = useQuery({
    queryKey: expertRolesQueryKey,
    queryFn: () => agentService.listExpertRoles(),
  });

  const saveMutation = useMutation({
    mutationFn: (input: SaveExpertRoleInput) => agentService.saveExpertRole(input),
    onSuccess: async () => {
      setEditing(undefined);
      setError(null);
      setNotice(t("expertRoles.saved"));
      await queryClient.invalidateQueries({ queryKey: expertRolesQueryKey });
    },
    onError: (reason: unknown) => setError(reason instanceof Error ? reason.message : String(reason)),
  });

  const deleteMutation = useMutation({
    mutationFn: (roleId: string) => agentService.deleteExpertRole(roleId),
    onSuccess: async () => {
      setError(null);
      setNotice(t("expertRoles.deleted"));
      await queryClient.invalidateQueries({ queryKey: expertRolesQueryKey });
    },
    onError: (reason: unknown) => setError(reason instanceof Error ? reason.message : String(reason)),
  });

  // Task 12.16: the same persistent error state the page's own <p> already renders from (set by
  // either mutation's onError, cleared only by a later success) -- reported so this page's own
  // nav entry can flag it too.
  useEffect(() => {
    onStatusChange?.(error ? { kind: "error", labelKey: "expertRoles.status.error" } : null);
    return () => onStatusChange?.(null);
  }, [error, onStatusChange]);

  const roles = useMemo(() => {
    const all = rolesQuery.data ?? [];
    const term = searchTerm.trim().toLowerCase();
    if (!term) return all;
    return all.filter((role) =>
      `${role.displayName} ${role.responsibility}`.toLowerCase().includes(term),
    );
  }, [rolesQuery.data, searchTerm]);

  return (
    <div className="grid content-start gap-4">
      {confirmationDialog}
      <PageHeader
        description={t("expertRoles.description")}
        icon={UserRound}
        title={t("expertRoles.title")}
      />

      <SectionPanel title={t("expertRoles.title")}>
        <div className="mb-3 flex items-center justify-between gap-3">
          <span className="text-xs text-muted-foreground">{t("expertRoles.builtinHint")}</span>
          <Button className="h-8 px-3 text-xs" onClick={() => setEditing(null)} type="button">
            <Plus aria-hidden="true" className="h-3.5 w-3.5" />
            {t("expertRoles.create")}
          </Button>
        </div>

        {error ? <p className="mb-2 text-xs text-destructive">{error}</p> : null}
        {notice && !error ? <p className="mb-2 text-xs text-muted-foreground">{notice}</p> : null}

        {editing !== undefined ? (
          <div className="mb-3">
            <ExpertRoleForm
              onCancel={() => setEditing(undefined)}
              onSubmit={(input) => saveMutation.mutate(input)}
              role={editing}
              submitting={saveMutation.isPending}
            />
          </div>
        ) : null}

        {roles.length === 0 ? (
          <p className="rounded-lg border border-dashed border-border p-6 text-center text-sm text-muted-foreground">
            {t("expertRoles.empty")}
          </p>
        ) : (
          <ul className="grid gap-2">
            {roles.map((role) => (
              <li className="ucd-list-row grid gap-2 rounded-lg p-3" key={role.id}>
                <div className="flex items-start gap-3">
                  <span
                    aria-hidden="true"
                    className="flex h-9 w-9 shrink-0 items-center justify-center rounded-md border text-base"
                    style={{ borderColor: role.color }}
                  >
                    {role.avatar}
                  </span>
                  <div className="min-w-0 flex-1">
                    <div className="flex items-center gap-2">
                      <span className="truncate text-sm font-medium">{role.displayName}</span>
                      {role.origin === "builtin" ? (
                        <span className="rounded bg-muted px-1.5 py-0.5 text-[10px] text-muted-foreground">
                          {t("expertRoles.builtin")}
                        </span>
                      ) : null}
                    </div>
                    <p className="mt-0.5 text-xs text-muted-foreground">{role.responsibility}</p>
                  </div>
                  <div className="flex shrink-0 gap-1">
                    <Button
                      className="h-8 px-2 text-xs"
                      onClick={() => setEditing({ ...role, origin: "user", id: "" })}
                      title={t("expertRoles.copy")}
                      type="button"
                      variant="outline"
                    >
                      <Copy aria-hidden="true" className="h-3.5 w-3.5" />
                    </Button>
                    {role.origin === "user" ? (
                      <>
                        <Button
                          className="h-8 px-2 text-xs"
                          onClick={() => setEditing(role)}
                          title={t("expertRoles.edit")}
                          type="button"
                          variant="outline"
                        >
                          <Pencil aria-hidden="true" className="h-3.5 w-3.5" />
                        </Button>
                        <Button
                          className="h-8 px-2 text-xs"
                          onClick={() => {
                            void confirm({ title: t("expertRoles.deleteConfirm"), tone: "danger" })
                              .then((confirmed) => { if (confirmed) deleteMutation.mutate(role.id); });
                          }}
                          title={t("expertRoles.delete")}
                          type="button"
                          variant="destructive"
                        >
                          <Trash2 aria-hidden="true" className="h-3.5 w-3.5" />
                        </Button>
                      </>
                    ) : null}
                  </div>
                </div>
              </li>
            ))}
          </ul>
        )}
      </SectionPanel>
    </div>
  );
}
