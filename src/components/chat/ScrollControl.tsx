import { ArrowDown } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Button } from "../ui/button";

export function ScrollControl({ onClick, visible }: { onClick: () => void; visible: boolean }) {
  const { t } = useTranslation();
  if (!visible) return null;
  return (
    <Button className="absolute bottom-3 right-3 h-8 px-2 text-xs shadow-lg" onClick={onClick} type="button" variant="outline">
      <ArrowDown className="h-3.5 w-3.5" aria-hidden="true" />
      {t("chat.bottom")}
    </Button>
  );
}
