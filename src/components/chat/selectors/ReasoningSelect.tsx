import { Lightbulb } from "lucide-react";
import { useTranslation } from "react-i18next";
import type { ReasoningDepth } from "../../../types/chat";
import { SelectorButton, SelectorDropdown } from "./SelectorDropdown";

export function ReasoningSelect({
  availableReasoning,
  onChange,
  onClose,
  onOpen,
  open,
  value,
}: {
  availableReasoning: ReasoningDepth[];
  onChange: (value: ReasoningDepth) => void;
  onClose: () => void;
  onOpen: () => void;
  open: boolean;
  value: ReasoningDepth;
}) {
  const { t } = useTranslation();
  const labelFor = (depth: ReasoningDepth) => t(`chat.config.reasoning.${depth}`);
  if (availableReasoning.length === 0) return null;
  return (
    <div className="relative">
      <SelectorButton icon={<Lightbulb className="h-3.5 w-3.5" aria-hidden="true" />} label={labelFor(value)} onClick={onOpen} open={open} title={t("chat.config.reasoningTitle", { label: labelFor(value) })} />
      {open ? (
        <SelectorDropdown
          onClose={onClose}
          onSelect={onChange}
          options={availableReasoning.map((depth) => ({
            value: depth,
            label: labelFor(depth),
            description: depth === "max" ? t("chat.config.reasoning.maxDesc") : t("chat.config.reasoning.defaultDesc"),
            icon: <Lightbulb className="h-3.5 w-3.5" aria-hidden="true" />,
          }))}
          value={value}
        />
      ) : null}
    </div>
  );
}
