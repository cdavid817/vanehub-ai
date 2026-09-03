import { Check, ChevronDown, Settings } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { FolderOpenerIcon } from "../components/folder-opener-icon";
import { agentService } from "../services/runtime-agent-client";
import type { Session } from "../types/agent";
import type { FolderOpenerAvailability, FolderOpenerId, FolderOpenerPreferences } from "../types/folder-opener";
import { useMenuList } from "../ui/actions/use-menu-list";

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

  const enabled = (preferences?.enabledOpenerIds ?? [])
    .map((id) => openers.find((item) => item.id === id))
    .filter((item): item is FolderOpenerAvailability => item !== undefined && item.status === "available");
  // +1 for the trailing "manage" item, which is not itself in `enabled`.
  const { activeIndex, handleMenuKeyDown, setActiveIndex } = useMenuList(Array.from({ length: enabled.length + 1 }));
  const itemRefs = useRef<(HTMLButtonElement | null)[]>([]);

  useEffect(() => {
    if (!menuOpen) return;
    // Resets the roving index every time the menu opens -- this component persists across opens,
    // so without this a stale index from a previous session would carry over instead of starting
    // back at the first item.
    setActiveIndex(0);
  }, [menuOpen, setActiveIndex]);

  // Follows the roving index while open: fires once on open (focusing the first item) and again
  // on every Arrow/Home/End press (moving real DOM focus along with it).
  useEffect(() => {
    if (menuOpen) itemRefs.current[activeIndex]?.focus();
  }, [menuOpen, activeIndex]);

  useEffect(() => {
    if (!menuOpen) return;
    function closeOnOutsidePointer(event: PointerEvent) {
      const target = event.target;
      if (target instanceof Node && menuRef.current?.contains(target)) return;
      setMenuOpen(false);
    }
    window.addEventListener("pointerdown", closeOnOutsidePointer);
    return () => window.removeEventListener("pointerdown", closeOnOutsidePointer);
  }, [menuOpen]);

  useEffect(() => {
    // Unmounting before the subscription resolves must still release it, otherwise the
    // listener outlives the component with nothing left holding its unsubscribe handle.
    let active = true;
    let unsubscribe: (() => void) | undefined;
    void agentService.subscribeFolderOpenerEvents(() => {
      void Promise.all([agentService.listFolderOpeners(), agentService.getFolderOpenerPreferences()]).then(([nextOpeners, nextPreferences]) => {
        if (active) { setOpeners(nextOpeners); setPreferences(nextPreferences); }
      }).catch((cause) => { if (active) setError(cause instanceof Error ? cause.message : String(cause)); });
    }).then((cleanup) => { if (active) unsubscribe = cleanup; else cleanup(); });
    return () => { active = false; unsubscribe?.(); };
  }, []);

  const targetAvailable = Boolean(session && !session.remoteWorkspace && (session.worktreePath || session.folder || session.projectPath));
  const effective = preferences?.effectiveDefaultOpenerId ?? null;

  async function launch(id: FolderOpenerId) {
    if (!session) return;
    setBusy(true); setError(null); setMenuOpen(false);
    try {
      const result = await agentService.openSessionFolder(session.id, id);
      if (result.status !== "opened") setError(t("folderOpeners.webUnavailable"));
    } catch (cause) { setError(cause instanceof Error ? cause.message : String(cause)); }
    finally { setBusy(false); }
  }

  async function selectDefault(id: FolderOpenerId) {
    if (!preferences) return;
    setBusy(true); setError(null); setMenuOpen(false);
    try {
      setPreferences(await agentService.saveFolderOpenerPreferences({
        configuredDefaultOpenerId: id,
        enabledOpenerIds: preferences.enabledOpenerIds,
      }));
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
      handleMenuKeyDown(event);
      if (event.key !== "Escape") return;
      event.preventDefault();
      setMenuOpen(false);
      menuButtonRef.current?.focus();
    }} role="menu">
      {preferences?.fallbackActive ? <p className="px-2 py-1 text-xs text-muted-foreground">{t("folderOpeners.fallbackActive")}</p> : null}
      {enabled.map((opener, index) => <button className="flex w-full items-center gap-2 rounded px-2 py-2 text-left text-sm hover:bg-muted" key={opener.id} onClick={() => void selectDefault(opener.id)} onFocus={() => setActiveIndex(index)} ref={(element) => { itemRefs.current[index] = element; }} role="menuitem" tabIndex={index === activeIndex ? 0 : -1} type="button"><FolderOpenerIcon id={opener.id} /><span className="flex-1">{t(`folderOpeners.name.${opener.id}`)}</span>{effective === opener.id ? <Check className="h-4 w-4" /> : null}</button>)}
      <div className="my-1 border-t border-border" />
      <button className="flex w-full items-center gap-2 rounded px-2 py-2 text-left text-sm hover:bg-muted" onClick={() => { setMenuOpen(false); onOpenSettings(); }} onFocus={() => setActiveIndex(enabled.length)} ref={(element) => { itemRefs.current[enabled.length] = element; }} role="menuitem" tabIndex={enabled.length === activeIndex ? 0 : -1} type="button"><Settings className="h-4 w-4" />{t("folderOpeners.manage")}</button>
    </div> : null}
  </div>;
}
