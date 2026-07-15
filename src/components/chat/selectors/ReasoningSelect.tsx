import { Lightbulb } from "lucide-react";
import type { ReasoningDepth } from "../../../types/chat";
import { SelectorButton, SelectorDropdown } from "./SelectorDropdown";

const labels: Record<ReasoningDepth, string> = {
  low: "Low",
  medium: "Medium",
  high: "High",
  max: "Max",
};

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
  if (availableReasoning.length === 0) return null;
  return (
    <div className="relative">
      <SelectorButton icon={<Lightbulb className="h-3.5 w-3.5" aria-hidden="true" />} label={labels[value]} onClick={onOpen} open={open} title={`思考深度: ${labels[value]}`} />
      {open ? (
        <SelectorDropdown
          onClose={onClose}
          onSelect={onChange}
          options={availableReasoning.map((depth) => ({
            value: depth,
            label: labels[depth],
            description: depth === "max" ? "最深推理" : "控制响应前的推理投入",
            icon: <Lightbulb className="h-3.5 w-3.5" aria-hidden="true" />,
          }))}
          value={value}
        />
      ) : null}
    </div>
  );
}
