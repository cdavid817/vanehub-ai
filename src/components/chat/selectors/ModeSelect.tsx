import { Bot, CheckCircle2, ListChecks, Wand2 } from "lucide-react";
import { useTranslation } from "react-i18next";
import type { PermissionMode } from "../../../types/chat";
import { SelectorButton, SelectorDropdown } from "./SelectorDropdown";

const modeIcons: Record<PermissionMode, JSX.Element> = {
  default: <CheckCircle2 className="h-3.5 w-3.5" aria-hidden="true" />,
  plan: <ListChecks className="h-3.5 w-3.5" aria-hidden="true" />,
  agent: <Bot className="h-3.5 w-3.5" aria-hidden="true" />,
  auto: <Wand2 className="h-3.5 w-3.5" aria-hidden="true" />,
};

export function ModeSelect({
  availableModes,
  onChange,
  onClose,
  onOpen,
  open,
  value,
}: {
  availableModes: PermissionMode[];
  onChange: (value: PermissionMode) => void;
  onClose: () => void;
  onOpen: () => void;
  open: boolean;
  value: PermissionMode;
}) {
  const { t } = useTranslation();
  const permissionModes = availableModes.concat(["default", "plan", "agent", "auto"] as PermissionMode[])
    .filter((mode, index, modes) => modes.indexOf(mode) === index);
  const labelFor = (mode: PermissionMode) => t(`chat.config.permission.${mode}`);
  const descriptionFor = (mode: PermissionMode) => t(`chat.config.permission.${mode}Desc`);
  return (
    <div className="relative">
      <SelectorButton icon={modeIcons[value]} label={labelFor(value)} onClick={onOpen} open={open} title={t("chat.config.modeTitle", { label: labelFor(value) })} />
      {open ? (
        <SelectorDropdown
          onClose={onClose}
          onSelect={onChange}
          options={permissionModes.map((mode) => ({
            value: mode,
            label: labelFor(mode),
            description: descriptionFor(mode),
            icon: modeIcons[mode],
            disabled: !availableModes.includes(mode),
          }))}
          value={value}
        />
      ) : null}
    </div>
  );
}
