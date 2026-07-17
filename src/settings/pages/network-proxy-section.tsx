import { Eye, EyeOff, Search, TestTube2, X } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "../../components/ui/button";
import { normalizeNetworkProxyBypass } from "../../services/settings-service";
import type { DetectedNetworkProxy } from "../../types/settings";
import { useSettings } from "../settings-provider";
import { SectionPanel } from "./page-parts";

function extractAuth(url: string) {
  if (!url.trim()) return { baseUrl: "", username: "", password: "" };
  try {
    const parsed = new URL(url);
    const username = decodeURIComponent(parsed.username || "");
    const password = decodeURIComponent(parsed.password || "");
    parsed.username = "";
    parsed.password = "";
    return { baseUrl: parsed.toString(), username, password };
  } catch {
    return { baseUrl: url, username: "", password: "" };
  }
}

function mergeAuth(baseUrl: string, username: string, password: string) {
  const trimmedUrl = baseUrl.trim();
  const trimmedUsername = username.trim();
  if (!trimmedUrl || !trimmedUsername) return trimmedUrl;
  try {
    const parsed = new URL(trimmedUrl);
    parsed.username = trimmedUsername;
    if (password) parsed.password = password;
    return parsed.toString();
  } catch {
    return trimmedUrl;
  }
}

export function NetworkProxySection() {
  const { t } = useTranslation();
  const { loading, reportClientLogEvent, saveSetting, scanNetworkProxies, settings, testNetworkProxy } = useSettings();
  const [urlDraft, setUrlDraft] = useState("");
  const [bypassDraft, setBypassDraft] = useState(settings.networkProxyBypass);
  const [usernameDraft, setUsernameDraft] = useState("");
  const [passwordDraft, setPasswordDraft] = useState("");
  const [showPassword, setShowPassword] = useState(false);
  const [detected, setDetected] = useState<DetectedNetworkProxy[]>([]);
  const [status, setStatus] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busyAction, setBusyAction] = useState<"save" | "test" | "scan" | null>(null);

  useEffect(() => {
    const auth = extractAuth(settings.networkProxyUrl);
    setUrlDraft(auth.baseUrl);
    setUsernameDraft(auth.username);
    setPasswordDraft(auth.password);
  }, [settings.networkProxyUrl]);

  useEffect(() => {
    setBypassDraft(settings.networkProxyBypass);
  }, [settings.networkProxyBypass]);

  const fullUrl = useMemo(() => mergeAuth(urlDraft, usernameDraft, passwordDraft), [passwordDraft, urlDraft, usernameDraft]);
  const normalizedBypass = useMemo(() => normalizeNetworkProxyBypass(bypassDraft), [bypassDraft]);
  const dirty = fullUrl !== settings.networkProxyUrl || normalizedBypass !== settings.networkProxyBypass;
  const disabled = loading || busyAction !== null;

  const saveProxy = async () => {
    setBusyAction("save");
    setError(null);
    setStatus(null);
    try {
      if (fullUrl !== settings.networkProxyUrl) {
        await saveSetting("networkProxyUrl", fullUrl);
      }
      if (normalizedBypass !== settings.networkProxyBypass) {
        await saveSetting("networkProxyBypass", normalizedBypass);
      }
      setStatus(t("basic.proxySaved"));
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      setError(message);
      void reportClientLogEvent({
        level: "error",
        kind: "critical-operation-failure",
        message,
        source: "NetworkProxySection.save",
        details: { proxyUrl: fullUrl ? "[configured]" : "[direct]" },
      });
    } finally {
      setBusyAction(null);
    }
  };

  const testProxy = async () => {
    setBusyAction("test");
    setError(null);
    setStatus(null);
    try {
      const result = await testNetworkProxy({ url: fullUrl, bypass: normalizedBypass });
      if (result.success) {
        setStatus(t("basic.proxyTestSuccess", { latency: result.latencyMs }));
      } else {
        setError(t("basic.proxyTestFailed", { error: result.error ?? t("basic.proxyUnknownError") }));
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setBusyAction(null);
    }
  };

  const scanProxies = async () => {
    setBusyAction("scan");
    setError(null);
    setStatus(null);
    try {
      const results = await scanNetworkProxies();
      setDetected(results);
      setStatus(results.length ? t("basic.proxyScanFound", { count: results.length }) : t("basic.proxyScanEmpty"));
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setBusyAction(null);
    }
  };

  return (
    <SectionPanel title={t("basic.proxy")} description={t("basic.proxyDesc")}>
      <div className="grid gap-4">
        {error ? <div className="rounded border p-3 text-xs ucd-status-danger">{error}</div> : null}
        {status ? <div className="rounded border p-3 text-xs ucd-status-success">{status}</div> : null}
        <label className="grid gap-1 text-sm">
          <span className="text-muted-foreground">{t("basic.proxyUrl")}</span>
          <input
            className="ucd-input h-9 rounded px-3 font-mono text-sm outline-none focus-visible:ring-2 focus-visible:ring-ring"
            disabled={disabled}
            onChange={(event) => setUrlDraft(event.target.value)}
            placeholder={t("basic.proxyUrlPlaceholder")}
            value={urlDraft}
          />
        </label>
        <div className="grid gap-4 md:grid-cols-2">
          <label className="grid gap-1 text-sm">
            <span className="text-muted-foreground">{t("basic.proxyUsername")}</span>
            <input
              className="ucd-input h-9 rounded px-3 font-mono text-sm outline-none focus-visible:ring-2 focus-visible:ring-ring"
              disabled={disabled}
              onChange={(event) => setUsernameDraft(event.target.value)}
              placeholder={t("basic.proxyUsernamePlaceholder")}
              value={usernameDraft}
            />
          </label>
          <label className="grid gap-1 text-sm">
            <span className="text-muted-foreground">{t("basic.proxyPassword")}</span>
            <span className="relative">
              <input
                className="ucd-input h-9 w-full rounded px-3 pr-10 font-mono text-sm outline-none focus-visible:ring-2 focus-visible:ring-ring"
                disabled={disabled}
                onChange={(event) => setPasswordDraft(event.target.value)}
                placeholder={t("basic.proxyPasswordPlaceholder")}
                type={showPassword ? "text" : "password"}
                value={passwordDraft}
              />
              <button
                aria-label={showPassword ? t("basic.proxyHidePassword") : t("basic.proxyShowPassword")}
                className="absolute right-2 top-1/2 -translate-y-1/2 text-muted-foreground"
                disabled={disabled}
                onClick={() => setShowPassword((value) => !value)}
                type="button"
              >
                {showPassword ? <EyeOff className="h-4 w-4" aria-hidden="true" /> : <Eye className="h-4 w-4" aria-hidden="true" />}
              </button>
            </span>
          </label>
        </div>
        <label className="grid gap-1 text-sm">
          <span className="text-muted-foreground">{t("basic.proxyBypass")}</span>
          <input
            className="ucd-input h-9 rounded px-3 font-mono text-sm outline-none focus-visible:ring-2 focus-visible:ring-ring"
            disabled={disabled}
            onChange={(event) => setBypassDraft(event.target.value)}
            placeholder={t("basic.proxyBypassPlaceholder")}
            value={bypassDraft}
          />
        </label>
        {detected.length ? (
          <div className="flex flex-wrap gap-2">
            {detected.map((proxy) => (
              <Button
                disabled={disabled}
                key={proxy.url}
                onClick={() => {
                  setUrlDraft(proxy.url);
                  setDetected([]);
                }}
                size="sm"
                type="button"
                variant="outline"
              >
                {proxy.url}
              </Button>
            ))}
          </div>
        ) : null}
        <div className="flex flex-wrap gap-2">
          <Button disabled={disabled || busyAction === "scan"} onClick={() => void scanProxies()} type="button" variant="outline">
            <Search className="h-4 w-4" aria-hidden="true" />
            {busyAction === "scan" ? t("basic.proxyScanning") : t("basic.proxyScan")}
          </Button>
          <Button disabled={disabled || !fullUrl} onClick={() => void testProxy()} type="button" variant="outline">
            <TestTube2 className="h-4 w-4" aria-hidden="true" />
            {busyAction === "test" ? t("basic.proxyTesting") : t("basic.proxyTest")}
          </Button>
          <Button
            disabled={disabled || (!urlDraft && !usernameDraft && !passwordDraft)}
            onClick={() => {
              setUrlDraft("");
              setUsernameDraft("");
              setPasswordDraft("");
              setStatus(null);
              setError(null);
            }}
            type="button"
            variant="outline"
          >
            <X className="h-4 w-4" aria-hidden="true" />
            {t("basic.proxyClear")}
          </Button>
          <Button disabled={disabled || !dirty} onClick={() => void saveProxy()} type="button">
            {busyAction === "save" ? t("basic.saving") : t("basic.proxySave")}
          </Button>
        </div>
      </div>
    </SectionPanel>
  );
}
