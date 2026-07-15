import { Bot, CheckCircle2, ListChecks, Wand2 } from "lucide-react";
import type { PermissionMode } from "../../../types/chat";
import { PERMISSION_MODES } from "../models";
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
  const current = PERMISSION_MODES.find((mode) => mode.id === value) ?? PERMISSION_MODES[0];
  return (
    <div className="relative">
      <SelectorButton icon={modeIcons[current.id]} label={current.label} onClick={onOpen} open={open} title={`模式: ${current.label}`} />
      {open ? (
        <SelectorDropdown
          onClose={onClose}
          onSelect={onChange}
          options={PERMISSION_MODES.map((mode) => ({
            value: mode.id,
            label: mode.label,
            description: mode.description,
            icon: modeIcons[mode.id],
            disabled: !availableModes.includes(mode.id),
          }))}
          value={value}
        />
      ) : null}
    </div>
  );
}
