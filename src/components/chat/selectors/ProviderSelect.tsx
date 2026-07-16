import { BrainCircuit, Code2, Sparkles, TerminalSquare } from "lucide-react";
import { PROVIDER_LABELS } from "../models";
import { SelectorButton, SelectorDropdown } from "./SelectorDropdown";

const providerIcons = {
  anthropic: <Sparkles className="h-3.5 w-3.5" aria-hidden="true" />,
  openai: <Code2 className="h-3.5 w-3.5" aria-hidden="true" />,
  google: <BrainCircuit className="h-3.5 w-3.5" aria-hidden="true" />,
  opencode: <TerminalSquare className="h-3.5 w-3.5" aria-hidden="true" />,
};

export function ProviderSelect({
  onChange,
  onClose,
  onOpen,
  open,
  value,
}: {
  onChange: (value: string) => void;
  onClose: () => void;
  onOpen: () => void;
  open: boolean;
  value: string;
}) {
  const label = PROVIDER_LABELS[value] ?? value;
  return (
    <div className="relative">
      <SelectorButton compact icon={providerIcons[value as keyof typeof providerIcons]} label={label} onClick={onOpen} open={open} title={`Provider: ${label}`} />
      {open ? (
        <SelectorDropdown
          onClose={onClose}
          onSelect={onChange}
          options={Object.entries(PROVIDER_LABELS).map(([providerId, providerLabel]) => ({
            value: providerId,
            label: providerLabel,
            icon: providerIcons[providerId as keyof typeof providerIcons],
          }))}
          value={value}
        />
      ) : null}
    </div>
  );
}
