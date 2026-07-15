import { ArrowDown } from "lucide-react";
import { Button } from "../ui/button";

export function ScrollControl({ onClick, visible }: { onClick: () => void; visible: boolean }) {
  if (!visible) return null;
  return (
    <Button className="absolute bottom-3 right-3 h-8 px-2 text-xs shadow-lg" onClick={onClick} type="button" variant="outline">
      <ArrowDown className="h-3.5 w-3.5" aria-hidden="true" />
      底部
    </Button>
  );
}
