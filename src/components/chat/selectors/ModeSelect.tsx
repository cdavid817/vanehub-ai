import { CheckCircle2, ListChecks, Play } from "lucide-react";
import type { ReactElement } from "react";
import { useTranslation } from "react-i18next";
import type { SessionExecutionMode } from "../../../types/chat";
import { SelectorButton, SelectorDropdown } from "./SelectorDropdown";

const modeIcons: Record<SessionExecutionMode, ReactElement> = {
  inherit: <CheckCircle2 className="h-3.5 w-3.5" aria-hidden="true" />,
  plan: <ListChecks className="h-3.5 w-3.5" aria-hidden="true" />,
  execute: <Play className="h-3.5 w-3.5" aria-hidden="true" />,
};

export function ModeSelect({
  availableModes,
  emphasizeCapabilities = false,
  onChange,
  onClose,
  onOpen,
  open,
  value,
}: {
  availableModes: SessionExecutionMode[];
  emphasizeCapabilities?: boolean;
  onChange: (value: SessionExecutionMode) => void;
  onClose: () => void;
  onOpen: () => void;
  open: boolean;
  value: SessionExecutionMode;
}) {
  const { t } = useTranslation();
  const executionModes = availableModes.concat(["inherit", "plan", "execute"] as SessionExecutionMode[])
    .filter((mode, index, modes) => modes.indexOf(mode) === index);
  const onePieceKey = (mode: SessionExecutionMode) => mode === "execute" ? "agent" : mode;
  const labelFor = (mode: SessionExecutionMode) => emphasizeCapabilities && (mode === "plan" || mode === "execute")
    ? t(`chat.config.permission.onepiece.${onePieceKey(mode)}`)
    : t(`chat.config.execution.${mode}`);
  const descriptionFor = (mode: SessionExecutionMode) => emphasizeCapabilities && (mode === "plan" || mode === "execute")
    ? t(`chat.config.permission.onepiece.${onePieceKey(mode)}Desc`)
    : t(`chat.config.execution.${mode}Desc`);
  return (
    <div className="relative">
      <SelectorButton icon={modeIcons[value]} label={labelFor(value)} onClick={onOpen} open={open} title={t("chat.config.modeTitle", { label: labelFor(value) })} />
      {open ? (
        <SelectorDropdown
          onClose={onClose}
          onSelect={onChange}
          options={executionModes.map((mode) => ({
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
