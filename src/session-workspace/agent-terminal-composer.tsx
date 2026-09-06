import { useState, type KeyboardEvent } from "react";
import { ChevronDown, ChevronUp, Send } from "lucide-react";
import { useTranslation } from "react-i18next";

export const agentTerminalInputClassName =
  "ucd-agent-terminal-input min-h-20 w-full resize-none border-0 px-2 py-1 text-sm outline-hidden disabled:cursor-not-allowed";

// A per-viewer display convenience, not an application setting: browser storage is the right
// home and an unavailable store simply means "expanded".
const collapsedStorageKey = "vanehub.agentTerminal.composerCollapsed";

function readCollapsed() {
  try {
    return window.localStorage.getItem(collapsedStorageKey) === "1";
  } catch {
    return false;
  }
}

function writeCollapsed(collapsed: boolean) {
  try {
    window.localStorage.setItem(collapsedStorageKey, collapsed ? "1" : "0");
  } catch {
    // Nothing to do; the choice just will not be remembered.
  }
}

/**
 * The line composer under the Agent terminal. Owns only the draft text and its folded state; the
 * tab decides where a submitted line goes. The terminal accepts keystrokes directly, so the
 * composer is optional and can be folded to a single bar to give the terminal the height.
 */
export function AgentTerminalComposer({ canSubmit, onSubmit }: { canSubmit: boolean; onSubmit: (content: string) => void }) {
  const { t } = useTranslation();
  const [draft, setDraft] = useState("");
  const [collapsed, setCollapsed] = useState(readCollapsed);

  function toggleCollapsed() {
    setCollapsed((current) => {
      writeCollapsed(!current);
      return !current;
    });
  }

  function submit() {
    const content = draft.trimEnd();
    if (!canSubmit || !content) return;
    onSubmit(content);
    setDraft("");
  }

  function handleKeyDown(event: KeyboardEvent<HTMLTextAreaElement>) {
    if (event.key !== "Enter" || event.shiftKey) return;
    event.preventDefault();
    submit();
  }

  const toggleLabel = collapsed ? t("sessionTabs.agentTerminal.expandComposer") : t("sessionTabs.agentTerminal.collapseComposer");
  const toggle = (
    <button
      aria-expanded={!collapsed}
      aria-label={toggleLabel}
      className="flex h-6 items-center gap-1 rounded px-1.5 text-xs text-muted-foreground hover:bg-muted hover:text-foreground"
      onClick={toggleCollapsed}
      title={toggleLabel}
      type="button"
    >
      {collapsed ? <ChevronUp className="h-3.5 w-3.5" aria-hidden="true" /> : <ChevronDown className="h-3.5 w-3.5" aria-hidden="true" />}
      <span>{toggleLabel}</span>
    </button>
  );

  if (collapsed) {
    return (
      <div className="flex shrink-0 items-center justify-between gap-2 border-t border-border bg-background/80 px-2 py-1">
        <span className="truncate text-xs text-muted-foreground">{t("sessionTabs.agentTerminal.composerCollapsedHint")}</span>
        {toggle}
      </div>
    );
  }

  return (
    <form className="shrink-0 border-t border-border bg-background/80 p-2" onSubmit={(event) => { event.preventDefault(); submit(); }}>
      <div className="rounded-lg border border-border bg-[hsl(var(--panel-muted))] p-2 shadow-xs focus-within:border-primary">
        <textarea
          aria-label={t("sessionTabs.agentTerminal.input")}
          className={agentTerminalInputClassName}
          disabled={!canSubmit}
          onKeyDown={handleKeyDown}
          onChange={(event) => setDraft(event.target.value)}
          placeholder={t("sessionTabs.agentTerminal.inputPlaceholder")}
          rows={3}
          value={draft}
        />
        <div className="mt-2 flex items-center justify-between gap-2">
          {toggle}
          <button
            className="flex h-8 items-center gap-1 rounded border border-border px-3 text-xs text-primary hover:bg-muted disabled:cursor-not-allowed disabled:opacity-50"
            disabled={!canSubmit || draft.trimEnd().length === 0}
            title={t("sessionTabs.agentTerminal.send")}
            type="submit"
          >
            <Send className="h-3.5 w-3.5" />
            {t("sessionTabs.agentTerminal.send")}
          </button>
        </div>
      </div>
    </form>
  );
}
