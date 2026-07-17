import { Boxes } from "lucide-react";
import { useTranslation } from "react-i18next";
import type { ModelInfo } from "../../../types/chat";
import { SelectorButton, SelectorDropdown } from "./SelectorDropdown";

export function ModelSelect({
  models,
  onChange,
  onClose,
  onOpen,
  open,
  value,
}: {
  models: ModelInfo[];
  onChange: (value: string) => void;
  onClose: () => void;
  onOpen: () => void;
  open: boolean;
  value: string;
}) {
  const { t } = useTranslation();
  const model = models.find((candidate) => candidate.id === value) ?? models[0];
  return (
    <div className="relative">
      <SelectorButton
        icon={<Boxes className="h-3.5 w-3.5" aria-hidden="true" />}
        label={model?.label ?? t("chat.config.model")}
        onClick={onOpen}
        open={open}
        title={t("chat.config.modelTitle", { label: model?.label ?? value })}
      />
      {open ? (
        <SelectorDropdown
          onClose={onClose}
          onSelect={onChange}
          options={models.map((option) => ({
            value: option.id,
            label: option.label,
            description: option.description,
            icon: <Boxes className="h-3.5 w-3.5" aria-hidden="true" />,
          }))}
          value={value}
        />
      ) : null}
    </div>
  );
}
