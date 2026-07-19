import { Check, ChevronDown, Settings } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { FolderOpenerIcon } from "../components/folder-opener-icon";
import { agentService } from "../services/runtime-agent-client";
import type { Session } from "../types/agent";
import type { FolderOpenerAvailability, FolderOpenerId, FolderOpenerPreferences } from "../types/folder-opener";

export function FolderOpenerControl({ session, onOpenSettings }: { session: Session | null; onOpenSettings: () => void }) {
  const { t } = useTranslation();
  const [openers, setOpeners] = useState<FolderOpenerAvailability[]>([]);
  const [preferences, setPreferences] = useState<FolderOpenerPreferences | null>(null);
  const [menuOpen, setMenuOpen] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const menuRef = useRef<HTMLDivElement>(null);
  const menuButtonRef = useRef<HTMLButtonElement>(null);

  useEffect(() => {
    let active = true;
    void Promise.all([agentService.listFolderOpeners(), agentService.getFolderOpenerPreferences()]).then(([nextOpeners, nextPreferences]) => {
      if (active) { setOpeners(nextOpeners); setPreferences(nextPreferences); }
    }).catch((cause) => { if (active) setError(cause instanceof Error ? cause.message : String(cause)); });
    return () => { active = false; };
  }, []);

  useEffect(() => {
    if (menuOpen) menuRef.current?.querySelector<HTMLButtonElement>('[role="menuitem"]')?.focus();
  }, [menuOpen]);

  useEffect(() => {
    let unsubscribe: (() => void) | undefined;
    void agentService.subscribeFolderOpenerEvents(() => {
      void Promise.all([agentService.listFolderOpeners(), agentService.getFolderOpenerPreferences()]).then(([nextOpeners, nextPreferences]) => {
        setOpeners(nextOpeners); setPreferences(nextPreferences);
      });
    }).then((cleanup) => { unsubscribe = cleanup; });
    return () => unsubscribe?.();
  }, []);

  const targetAvailable = Boolean(session && !session.remoteWorkspace && (session.worktreePath || session.folder || session.projectPath));
  const effective = preferences?.effectiveDefaultOpenerId ?? null;
  const enabled = openers.filter((item) => item.status === "available" && preferences?.enabledOpenerIds.includes(item.id));

  async function launch(id: FolderOpenerId) {
    if (!session) return;
    setBusy(true); setError(null); setMenuOpen(false);
    try {
      const result = await agentService.openSessionFolder(session.id, id);
      if (result.status !== "opened") setError(t("folderOpeners.webUnavailable"));
    } catch (cause) { setError(cause instanceof Error ? cause.message : String(cause)); }
    finally { setBusy(false); }
  }

  const disabled = busy || !targetAvailable || !effective;
  const title = !session ? t("folderOpeners.noSession") : session.remoteWorkspace ? t("folderOpeners.remoteUnavailable") : !targetAvailable ? t("folderOpeners.noFolder") : error ?? (preferences?.fallbackActive ? t("folderOpeners.fallbackActive") : t("folderOpeners.openWith", { app: effective ? t(`folderOpeners.name.${effective}`) : "" }));

  return <div className="relative flex shrink-0" ref={menuRef}>
    <button aria-label={title} className="flex h-10 items-center gap-1.5 rounded-l-md border border-border bg-background px-2 text-xs hover:bg-muted disabled:cursor-not-allowed disabled:opacity-50" disabled={disabled} onClick={() => effective && void launch(effective)} title={title} type="button">
      {effective ? <FolderOpenerIcon id={effective} /> : null}<span className="hidden xl:inline">{effective ? t(`folderOpeners.name.${effective}`) : t("folderOpeners.open")}</span>
    </button>
    <button aria-expanded={menuOpen} aria-haspopup="menu" aria-label={t("folderOpeners.menu")} className="flex h-10 w-7 items-center justify-center rounded-r-md border border-l-0 border-border bg-background hover:bg-muted disabled:opacity-50" disabled={!targetAvailable} onClick={() => setMenuOpen((value) => !value)} ref={menuButtonRef} type="button"><ChevronDown className="h-3.5 w-3.5" /></button>
    {menuOpen ? <div className="absolute right-0 top-11 z-50 min-w-56 rounded-md border border-border bg-background p-1 shadow-xl" onKeyDown={(event) => {
      if (event.key !== "Escape") return;
      event.preventDefault();
      setMenuOpen(false);
      menuButtonRef.current?.focus();
    }} role="menu">
      {preferences?.fallbackActive ? <p className="px-2 py-1 text-xs text-muted-foreground">{t("folderOpeners.fallbackActive")}</p> : null}
      {enabled.map((opener) => <button className="flex w-full items-center gap-2 rounded px-2 py-2 text-left text-sm hover:bg-muted" key={opener.id} onClick={() => void launch(opener.id)} role="menuitem" type="button"><FolderOpenerIcon id={opener.id} /><span className="flex-1">{t(`folderOpeners.name.${opener.id}`)}</span>{effective === opener.id ? <Check className="h-4 w-4" /> : null}</button>)}
      <div className="my-1 border-t border-border" />
      <button className="flex w-full items-center gap-2 rounded px-2 py-2 text-left text-sm hover:bg-muted" onClick={() => { setMenuOpen(false); onOpenSettings(); }} role="menuitem" type="button"><Settings className="h-4 w-4" />{t("folderOpeners.manage")}</button>
    </div> : null}
  </div>;
}
