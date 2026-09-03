import { Box, Check, ChevronDown, Copy, Globe2, KeyRound, MoreHorizontal, Pencil, Trash2 } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { Badge } from "../../../components/ui/badge";
import { Button } from "../../../components/ui/button";
import { managedSettingsPath, payloadSupportsCredential, payloadSupportsEndpointOverride, type CliConfigPreset, type CliConfigProfile } from "../../../types/cli-agent-config";
import type { ProviderCredentialValidationResult } from "../../../types/provider-credential-validation";
import { ProviderCredentialValidation } from "../../../components/provider-directory/provider-credential-validation";
import { useMenuList } from "../../../ui/actions/use-menu-list";

const profileActionItems = [0, 1, 2];

/**
 * Extracted so `useMenuList` has one component instance per row to call it on -- hooks cannot be
 * called a variable number of times inside `visibleProfiles.map(...)` in the parent.
 *
 * A plain trigger button + conditional popup (matching `ActionMenu.tsx`'s own shape), not the
 * native `<details>/<summary>` disclosure the other "details" section in this file still uses for
 * its own read-only connection info -- tried that first here too, but confirmed live that React's
 * `onToggle` never actually fires for a real user-driven `<details>` toggle in this test
 * environment (`details.open` itself does flip, the event just never reaches the handler), which
 * would leave this control's roving focus untestable and, given `toggle` is a famously
 * inconsistently-supported DOM event, not obviously reliable in a real browser either.
 */
function ProfileActionsMenu({ busy, label, onDelete, onDuplicate, onEdit }: {
  busy: boolean;
  label: string;
  onDelete: () => void;
  onDuplicate: () => void;
  onEdit: () => void;
}) {
  const { t } = useTranslation();
  const [open, setOpen] = useState(false);
  const { activeIndex, handleMenuKeyDown, setActiveIndex } = useMenuList(profileActionItems);
  const itemRefs = useRef<(HTMLButtonElement | null)[]>([]);
  const rootRef = useRef<HTMLDivElement>(null);
  const triggerRef = useRef<HTMLButtonElement>(null);

  // Resets the roving index in the same click handler that opens the menu, not a `[open]`-keyed
  // effect -- an effect would still observe the previous, possibly non-zero index on the very
  // render where `open` flips true (see the sibling fix in conversation-overflow-menu.tsx for the
  // race this avoids).
  function openMenu() {
    setActiveIndex(0);
    setOpen(true);
  }

  function selectAction(action: () => void) {
    setOpen(false);
    action();
  }

  useEffect(() => {
    if (!open) return;
    itemRefs.current[0]?.focus();
    function dismiss(event: PointerEvent) {
      if (!rootRef.current?.contains(event.target as Node)) setOpen(false);
    }
    function closeOnEscape(event: KeyboardEvent) {
      if (event.key !== "Escape") return;
      setOpen(false);
      triggerRef.current?.focus();
    }
    document.addEventListener("pointerdown", dismiss);
    document.addEventListener("keydown", closeOnEscape);
    return () => {
      document.removeEventListener("pointerdown", dismiss);
      document.removeEventListener("keydown", closeOnEscape);
    };
  }, [open]);

  // Follows the roving index while open, moving real DOM focus along with every Arrow/Home/End
  // press (the effect above already places the initial on-open focus).
  useEffect(() => {
    if (open) itemRefs.current[activeIndex]?.focus();
  }, [open, activeIndex]);

  return (
    <div className="relative" ref={rootRef}>
      <button aria-expanded={open} aria-haspopup="menu" aria-label={label} className="flex h-9 w-9 items-center justify-center rounded-md border border-border hover:bg-muted focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring" onClick={openMenu} ref={triggerRef} type="button"><MoreHorizontal className="h-4 w-4" /></button>
      {open ? (
        <div className="absolute right-0 z-10 mt-1 grid min-w-40 gap-1 rounded-lg border border-border bg-background p-1 shadow-lg" onKeyDown={handleMenuKeyDown} role="menu">
          <Button disabled={busy} onClick={() => selectAction(onEdit)} onFocus={() => setActiveIndex(0)} ref={(element) => { itemRefs.current[0] = element; }} role="menuitem" size="sm" tabIndex={activeIndex === 0 ? 0 : -1} variant="ghost"><Pencil className="h-4 w-4" />{t("agents.globalConfig.editProfile")}</Button>
          <Button disabled={busy} onClick={() => selectAction(onDuplicate)} onFocus={() => setActiveIndex(1)} ref={(element) => { itemRefs.current[1] = element; }} role="menuitem" size="sm" tabIndex={activeIndex === 1 ? 0 : -1} variant="ghost"><Copy className="h-4 w-4" />{t("agents.globalConfig.duplicate")}</Button>
          <Button disabled={busy} onClick={() => selectAction(onDelete)} onFocus={() => setActiveIndex(2)} ref={(element) => { itemRefs.current[2] = element; }} role="menuitem" size="sm" tabIndex={activeIndex === 2 ? 0 : -1} variant="destructive"><Trash2 className="h-4 w-4" />{t("agents.globalConfig.delete")}</Button>
        </div>
      ) : null}
    </div>
  );
}

const avatarTones = [
  "bg-blue-500/15 text-blue-700 dark:text-blue-300",
  "bg-violet-500/15 text-violet-700 dark:text-violet-300",
  "bg-emerald-500/15 text-emerald-700 dark:text-emerald-300",
  "bg-amber-500/15 text-amber-700 dark:text-amber-300",
];

function profileMetadata(profile: CliConfigProfile, presets: CliConfigPreset[]) {
  const preset = presets.find((candidate) => candidate.id === profile.sourcePresetId);
  const payload = profile.payload;
  const provider = preset?.displayName
    ?? (payload.kind === "opencode" ? payload.providerName : payload.kind === "codex-cli" ? payload.providerId : profile.name);
  // A kind that cannot point at a custom endpoint has none to show; the settings file it manages
  // stands in its place. Driven by the capability declaration rather than the kind literal, so a
  // future endpoint-free kind does not have to be remembered here.
  const endpoint = payloadSupportsEndpointOverride(payload) ? payload.baseUrl : managedSettingsPath(payload);
  const model = payload.kind === "opencode" ? payload.defaultModel : payload.model;
  const toneIndex = [...provider].reduce((total, character) => total + character.charCodeAt(0), 0) % avatarTones.length;
  return { endpoint, model, provider, tone: avatarTones[toneIndex] };
}

function profileUsesCredential(profile: CliConfigProfile) {
  const payload = profile.payload;
  if (!payloadSupportsCredential(payload)) return false;
  if (payload.kind === "claude-code") return payload.authMode !== "none";
  if (payload.kind === "codex-cli") return payload.authStrategy !== "preserve-official";
  if (payload.kind === "gemini-cli") return payload.authStrategy !== "preserve-official";
  return true;
}

export function CliConfigProfileList({
  profiles,
  presets,
  busy,
  searchTerms,
  onApply,
  onDelete,
  onDuplicate,
  onEdit,
  onValidate,
}: {
  profiles: CliConfigProfile[];
  presets: CliConfigPreset[];
  busy: boolean;
  searchTerms: string[];
  onApply: (profile: CliConfigProfile) => void;
  onDelete: (profile: CliConfigProfile) => void;
  onDuplicate: (profile: CliConfigProfile) => void;
  onEdit: (profile: CliConfigProfile) => void;
  onValidate: (profile: CliConfigProfile) => Promise<ProviderCredentialValidationResult>;
}) {
  const { t } = useTranslation();
  const queries = searchTerms.map((term) => term.trim().toLowerCase()).filter(Boolean);
  const visibleProfiles = profiles.filter((profile) => {
    const metadata = profileMetadata(profile, presets);
    const searchable = `${profile.name} ${metadata.provider} ${metadata.endpoint} ${metadata.model}`.toLowerCase();
    return queries.every((query) => searchable.includes(query));
  });

  return (
    <section aria-labelledby="saved-profiles-heading">
      <div className="mb-3">
        <h3 className="font-semibold" id="saved-profiles-heading">{t("agents.globalConfig.profiles")}</h3>
        <p className="mt-1 text-sm leading-6 text-muted-foreground">{t("agentConfigurations.profiles.description")}</p>
      </div>
      <div className="grid gap-3">
        {visibleProfiles.map((profile) => {
          const metadata = profileMetadata(profile, presets);
          const applied = profile.appliedState === "applied";
          return (
            <article className={`rounded-lg border p-3 transition-colors sm:p-4 ${applied ? "border-primary/60 bg-primary/[0.04] ring-1 ring-primary/10" : "border-border bg-background hover:border-primary/30"}`} key={profile.id}>
              <div className="flex flex-col gap-3 sm:flex-row sm:items-center">
                <div className="flex min-w-0 flex-1 items-start gap-3">
                  <div aria-hidden="true" className={`flex h-10 w-10 shrink-0 items-center justify-center rounded-lg text-sm font-bold ${metadata.tone}`}>{metadata.provider.slice(0, 1).toUpperCase()}</div>
                  <div className="min-w-0 flex-1">
                    <div className="flex flex-wrap items-center gap-2">
                      <h4 className="truncate font-semibold">{profile.name}</h4>
                      {applied ? <Badge tone="success">{t("agentConfigurations.profiles.currentlyApplied")}</Badge> : null}
                      <Badge tone={profile.validationState === "valid" ? "success" : "warning"}>{t(`agentConfigurations.validation.${profile.validationState}`)}</Badge>
                    </div>
                    <p className="mt-1 flex min-w-0 items-center gap-1.5 text-sm text-muted-foreground"><Box className="h-3.5 w-3.5 shrink-0" /><span className="truncate">{metadata.provider} · {metadata.model || t("agentConfigurations.profiles.notSet")}</span></p>
                  </div>
                </div>
                <div className="flex shrink-0 items-center gap-2 sm:justify-end">
                  <Button disabled={busy || applied} onClick={() => onApply(profile)}><Check className="h-4 w-4" />{applied ? t("agents.globalConfig.applied") : t("agents.globalConfig.apply")}</Button>
                  <ProfileActionsMenu
                    busy={busy}
                    label={`${t("agentConfigurations.profiles.moreActions")}: ${profile.name}`}
                    onDelete={() => onDelete(profile)}
                    onDuplicate={() => onDuplicate(profile)}
                    onEdit={() => onEdit(profile)}
                  />
                </div>
              </div>
              <details className="group mt-2 border-t border-border/60 pt-2 text-xs text-muted-foreground">
                <summary className="flex w-fit cursor-pointer list-none items-center gap-1.5 rounded focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring">{t("agentConfigurations.profiles.details")}<ChevronDown className="h-3.5 w-3.5 transition-transform group-open:rotate-180" /></summary>
                <div className="mt-3 grid min-w-0 gap-2 sm:grid-cols-3">
                  <p className="flex min-w-0 items-center gap-1.5"><Globe2 className="h-3.5 w-3.5 shrink-0" /><span className="truncate" title={metadata.endpoint}>{metadata.endpoint || t("agentConfigurations.profiles.notSet")}</span></p>
                  <p className="flex items-center gap-1.5"><KeyRound className="h-3.5 w-3.5 shrink-0" />{profile.credentialConfigured ? t("agents.globalConfig.credentialConfigured") : t("agents.globalConfig.credentialMissing")}</p>
                  <p>{profile.sourcePresetVersion ? `v${profile.sourcePresetVersion}` : t("agentConfigurations.profiles.notSet")}</p>
                </div>
                <div className="mt-3"><ProviderCredentialValidation disabled={busy || (profileUsesCredential(profile) && !profile.credentialConfigured)} onValidate={() => onValidate(profile)} resetKey={profile.updatedAt} size="sm" /></div>
              </details>
            </article>
          );
        })}
        {visibleProfiles.length === 0 ? <p className="rounded-xl border border-dashed border-border bg-muted/20 p-8 text-center text-sm leading-6 text-muted-foreground">{queries.length ? t("agentConfigurations.profiles.noMatches") : t("agents.globalConfig.empty")}</p> : null}
      </div>
    </section>
  );
}
