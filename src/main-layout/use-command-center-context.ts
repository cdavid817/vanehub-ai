import { useState } from "react";
import { useCommandCenterShortcut } from "../command-center/use-command-center-shortcut";
import type { WorkbenchCommandContext } from "../command-center/command-center-types";
import type { SettingsPageId } from "../settings/settings-pages";
import type { WorkbenchLocation } from "./workbench-route";

/**
 * Owns the Command Center's open state, the global Ctrl/Cmd+K listener, and the
 * `WorkbenchCommandContext` built from `MainLayout`'s own state setters — extracted so wiring it in
 * costs that file one hook call instead of a dozen lines inline (it is already at its raised
 * `max-lines` budget, `eslint.config.js`'s technical-debt exemption list).
 */
export function useCommandCenterContext(args: {
  location: WorkbenchLocation;
  navigate: (next: WorkbenchLocation, options?: { replace?: boolean; returnTo?: WorkbenchLocation }) => void;
  onNewSession: () => void;
  onOpenSettings: (pageId?: SettingsPageId) => void;
  onToggleFocusMode: () => void;
  onToggleInspector: () => void;
  onToggleNavigation: () => void;
}) {
  const [open, setOpen] = useState(false);
  useCommandCenterShortcut(() => setOpen(true));
  const context: WorkbenchCommandContext = { ...args };
  return { close: () => setOpen(false), context, open };
}
