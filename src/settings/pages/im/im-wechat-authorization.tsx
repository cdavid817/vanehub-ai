import { QrCode, RefreshCw, X } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Button } from "../../../components/ui/button";
import type { WeChatAuthorization } from "../../../contracts/im";

interface ImWeChatAuthorizationProps {
  authorization: WeChatAuthorization | null;
  pending: boolean;
  onBegin: () => void;
  onPoll: () => void;
  onCancel: () => void;
}

export function ImWeChatAuthorization({ authorization, pending, onBegin, onPoll, onCancel }: ImWeChatAuthorizationProps) {
  const { t } = useTranslation();
  if (!authorization || authorization.status === "expired" || authorization.status === "error" || authorization.status === "confirmed") {
    return (
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div>
          <p className="text-sm font-medium">{t("im.wechat.authorization")}</p>
          <p className="mt-1 text-xs text-muted-foreground">{t(`im.wechat.${authorization?.status ?? "notAuthorized"}`)}</p>
        </div>
        <Button disabled={pending} onClick={onBegin} size="sm"><QrCode aria-hidden="true" />{t(authorization ? "im.wechat.reauthorize" : "im.wechat.start")}</Button>
      </div>
    );
  }

  return (
    <div className="grid gap-4 sm:grid-cols-[160px_minmax(0,1fr)]">
      <div className="flex aspect-square w-40 items-center justify-center rounded-md border border-border bg-white p-2">
        {authorization.imageDataUrl ? <img alt={t("im.wechat.qrAlt")} className="h-full w-full object-contain" src={authorization.imageDataUrl} /> : <QrCode className="h-10 w-10 text-black" aria-hidden="true" />}
      </div>
      <div className="flex min-w-0 flex-col justify-center">
        <p className="font-medium">{t(`im.wechat.${authorization.status}`)}</p>
        <p className="mt-2 text-sm text-muted-foreground">{t("im.wechat.scanHint")}</p>
        {authorization.expiresAt ? <p className="mt-2 text-xs text-muted-foreground">{t("im.wechat.expiresAt", { time: new Date(authorization.expiresAt).toLocaleTimeString() })}</p> : null}
        <div className="mt-4 flex flex-wrap gap-2">
          <Button disabled={pending} onClick={onPoll} size="sm" variant="outline"><RefreshCw aria-hidden="true" />{t("im.wechat.check")}</Button>
          <Button disabled={pending} onClick={onCancel} size="sm" variant="ghost"><X aria-hidden="true" />{t("im.actions.cancel")}</Button>
        </div>
      </div>
    </div>
  );
}
