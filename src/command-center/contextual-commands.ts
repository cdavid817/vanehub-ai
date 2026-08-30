import type { WorkbenchCommand } from "./command-center-types";

const onSessionsDestination: WorkbenchCommand["isAvailable"] = (context) => context.location.destination === "sessions";

/**
 * 6.8/6.9. "Toggle Runtime Panel" (named in 6.8's own task text) is deliberately not here: the
 * Runtime Panel is a Decision 7 concept for Sessions M2 (§7-11) with no implementation anywhere in
 * the repo yet (confirmed by this same reasoning already applied to `workbench-layout-preferences.ts`
 * this session) — nothing exists yet for a command to toggle. Adding one now would be exactly the
 * fabricated-facet pattern this change's own instructions forbid elsewhere (5.1, 5.4).
 *
 * `isAvailable` is how 6.9's "hidden... with an accessible explanation" is satisfied: the three
 * panel-toggle commands only make sense on the Sessions destination (`DestinationLayout`'s
 * navigation/inspector panes and focus mode are Sessions-specific, per `TopBar`'s own
 * `focusModeAvailable={destination === "sessions"}`), so `CommandCenter` filters them out entirely
 * outside Sessions rather than showing a command that would do nothing.
 */
export const CONTEXTUAL_COMMANDS: WorkbenchCommand[] = [
  {
    id: "new-session",
    labelKey: "commandCenter.command.newSession",
    keywords: ["new session", "create session", "新建会话"],
    isAvailable: () => true,
    run: (context) => context.onNewSession(),
  },
  {
    id: "toggle-navigation",
    labelKey: "commandCenter.command.toggleNavigation",
    keywords: ["toggle sidebar", "session list", "会话栏", "会话列表"],
    isAvailable: onSessionsDestination,
    run: (context) => context.onToggleNavigation(),
  },
  {
    id: "toggle-inspector",
    labelKey: "commandCenter.command.toggleInspector",
    keywords: ["toggle inspector", "info panel", "信息面板"],
    isAvailable: onSessionsDestination,
    run: (context) => context.onToggleInspector(),
  },
  {
    id: "toggle-focus-mode",
    labelKey: "commandCenter.command.toggleFocusMode",
    keywords: ["focus mode", "专注模式"],
    isAvailable: onSessionsDestination,
    run: (context) => context.onToggleFocusMode(),
  },
  {
    id: "open-settings",
    labelKey: "commandCenter.command.openSettings",
    keywords: ["settings", "preferences", "设置"],
    isAvailable: () => true,
    run: (context) => context.onOpenSettings(),
  },
];
