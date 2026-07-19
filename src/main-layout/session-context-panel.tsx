import { useLayoutEffect, useRef, useState } from "react";
import { RotateCcw } from "lucide-react";
import { useTranslation } from "react-i18next";
import type { Session, SessionCategory, SessionExportFormat } from "../types/agent";

export type ContextPanelState = { session: Session; mode: "menu" | "rename" | "delete"; draftTitle: string; position?: { x: number; y: number } };

export function SessionContextPanel({ categories, onArchive, onAssignCategory, onChange, onCreateCategory, onDelete, onDismiss, onExport, onPin, onRename, value }: {
  categories: SessionCategory[];
  onArchive: (session: Session) => void; onChange: (value: ContextPanelState) => void; onDelete: (session: Session) => void;
  onAssignCategory: (session: Session, categoryId: string | null) => void;
  onCreateCategory: (session: Session) => void;
  onDismiss: () => void;
  onExport: (session: Session, format: SessionExportFormat) => void;
  onPin: (session: Session) => void; onRename: (session: Session, title: string) => void; value: ContextPanelState | null;
}) {
  const { t } = useTranslation();
  const menuRef = useRef<HTMLDivElement | null>(null);
  const rawX = value?.position?.x ?? 224;
  const rawY = value?.position?.y ?? 80;
  const [menuPosition, setMenuPosition] = useState({ x: rawX, y: rawY, ready: false });

  useLayoutEffect(() => {
    if (!value || value.mode !== "menu") return;
    const menu = menuRef.current;
    if (!menu) return;
    const rect = menu.getBoundingClientRect();
    const margin = 8;
    const gap = 4;
    const preferredX = rawX + gap;
    const preferredY = rawY + gap;
    const fallbackX = rawX - rect.width - gap;
    const fallbackY = rawY - rect.height - gap;
    const unclampedX = preferredX + rect.width <= window.innerWidth - margin ? preferredX : fallbackX;
    const unclampedY = preferredY + rect.height <= window.innerHeight - margin ? preferredY : fallbackY;
    const next = {
      x: Math.max(margin, Math.min(unclampedX, window.innerWidth - rect.width - margin)),
      y: Math.max(margin, Math.min(unclampedY, window.innerHeight - rect.height - margin)),
      ready: true,
    };
    setMenuPosition((current) =>
      current.x === next.x && current.y === next.y && current.ready === next.ready ? current : next,
    );
  }, [categories.length, rawX, rawY, value]);

  if (!value) return null;
  if (value.mode === "menu") {
    return <div className="fixed inset-0 z-50" onClick={onDismiss} onContextMenu={(event) => { event.preventDefault(); onDismiss(); }}>
      <div
        className="ucd-panel fixed grid max-h-[70vh] w-56 gap-1 overflow-y-auto rounded-md p-1 text-sm shadow-lg"
        onClick={(event) => event.stopPropagation()}
        ref={menuRef}
        style={{ left: menuPosition.x, top: menuPosition.y, visibility: menuPosition.ready ? "visible" : "hidden" }}
      >
    <button className="rounded px-2 py-1.5 text-left hover:bg-muted" onClick={() => onChange({ ...value, mode: "rename" })} type="button">{t("layout.rename")}</button>
    <button className="rounded px-2 py-1.5 text-left hover:bg-muted" onClick={() => { onPin(value.session); onDismiss(); }} type="button">{value.session.pinned ? t("layout.unpin") : t("layout.pinned")}</button>
    <button className="rounded px-2 py-1.5 text-left hover:bg-muted" onClick={() => { onArchive(value.session); onDismiss(); }} type="button">{value.session.archived ? <><RotateCcw className="mr-1 inline h-3.5 w-3.5" />{t("layout.restore")}</> : t("layout.archive")}</button>
    <div className="my-1 border-t border-border" />
    <p className="px-2 py-1 text-xs text-muted-foreground">{t("layout.moveToCategory")}</p>
    <button className="rounded px-2 py-1.5 text-left hover:bg-muted" onClick={() => { onAssignCategory(value.session, null); onDismiss(); }} type="button">{t("layout.uncategorized")}</button>
    {categories.map((category) => <button className="rounded px-2 py-1.5 text-left hover:bg-muted" key={category.id} onClick={() => { onAssignCategory(value.session, category.id); onDismiss(); }} type="button">{category.name}</button>)}
    <button className="rounded px-2 py-1.5 text-left hover:bg-muted" onClick={() => { onCreateCategory(value.session); onDismiss(); }} type="button">{t("layout.newCategory")}</button>
    <div className="my-1 border-t border-border" />
    <p className="px-2 py-1 text-xs text-muted-foreground">{t("layout.exportSession")}</p>
    <button className="rounded px-2 py-1.5 text-left hover:bg-muted" onClick={() => { onExport(value.session, "json"); onDismiss(); }} type="button">{t("layout.exportJson")}</button>
    <button className="rounded px-2 py-1.5 text-left hover:bg-muted" onClick={() => { onExport(value.session, "markdown"); onDismiss(); }} type="button">{t("layout.exportMarkdown")}</button>
    <div className="my-1 border-t border-border" />
    <button className="rounded px-2 py-1.5 text-left text-destructive hover:bg-muted" onClick={() => onChange({ ...value, mode: "delete" })} type="button">{t("layout.delete")}</button>
      </div>
    </div>;
  }
  if (value.mode === "rename") return <div className="fixed inset-0 z-50 grid place-items-center bg-background/60 p-4"><form className="ucd-panel grid w-full max-w-sm gap-3 rounded-lg p-4 text-sm shadow-xl" onSubmit={(event) => { event.preventDefault(); const title = value.draftTitle.trim(); if (title) onRename(value.session, title); onDismiss(); }}><div><h3 className="font-semibold">{t("layout.renameSession")}</h3><p className="mt-1 text-xs text-muted-foreground">{t("layout.renameDescription")}</p></div><label className="grid gap-1"><span className="text-xs text-muted-foreground">{t("layout.sessionName")}</span><input autoFocus className="ucd-input h-9 rounded px-2" onChange={(event) => onChange({ ...value, draftTitle: event.target.value })} value={value.draftTitle} /></label><div className="grid grid-cols-2 gap-2"><button className="h-8 rounded border border-border text-xs" onClick={onDismiss} type="button">{t("layout.cancel")}</button><button className="h-8 rounded bg-primary text-xs text-primary-foreground" disabled={!value.draftTitle.trim()} type="submit">{t("layout.confirm")}</button></div></form></div>;
  return <div className="fixed inset-0 z-50 grid place-items-center bg-background/60 p-4"><div className="ucd-panel grid w-full max-w-sm gap-3 rounded-lg p-4 text-sm shadow-xl"><div><h3 className="font-semibold">{t("layout.deleteSession")}</h3><p className="mt-1 break-words text-xs text-muted-foreground">“{value.session.title}” {t("layout.deleteDescription")}</p></div><div className="grid grid-cols-2 gap-2"><button className="h-8 rounded border border-border text-xs" onClick={onDismiss} type="button">{t("layout.cancel")}</button><button className="h-8 rounded bg-destructive text-xs text-destructive-foreground" onClick={() => { onDelete(value.session); onDismiss(); }} type="button">{t("layout.delete")}</button></div></div></div>;
}
