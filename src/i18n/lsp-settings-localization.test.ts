import { describe, expect, it } from "vitest";
import en from "./locales/en.json";
import ja from "./locales/ja.json";
import ko from "./locales/ko.json";
import zhCN from "./locales/zh-CN.json";
import zhTW from "./locales/zh-TW.json";
import type { AppLanguage, TranslationResource } from "./supported-locales";

const resources: Record<AppLanguage, TranslationResource> = {
  "zh-CN": zhCN,
  en,
  "zh-TW": zhTW,
  ja,
  ko,
};

const requiredKeys = [
  "lspSettings.title", "lspSettings.description", "lspSettings.loading",
  "lspSettings.loadError", "lspSettings.retry",
  "lspSettings.configuration.title", "lspSettings.configuration.master",
  "lspSettings.configuration.masterHint", "lspSettings.configuration.save",
  "lspSettings.configuration.saving", "lspSettings.configuration.saved",
  "lspSettings.configuration.saveError", "lspSettings.language.rust",
  "lspSettings.language.typescript_javascript", "lspSettings.language.enabled",
  "lspSettings.language.go", "lspSettings.language.python", "lspSettings.language.cpp",
  "lspSettings.language.java", "lspSettings.language.prerequisite",
  "lspSettings.language.unsupportedOnHost",
  "lspSettings.startupArguments.title", "lspSettings.startupArguments.description",
  "lspSettings.startupArguments.none", "lspSettings.startupArguments.placeholder",
  "lspSettings.startupArguments.tooMany", "lspSettings.startupArguments.tooLarge",
  "lspSettings.discovery.title", "lspSettings.discovery.description",
  "lspSettings.discovery.automatic", "lspSettings.discovery.manual",
  "lspSettings.discovery.available", "lspSettings.discovery.unavailable",
  "lspSettings.discovery.refresh", "lspSettings.discovery.override",
  "lspSettings.discovery.overridePlaceholder",
  "lspSettings.discovery.installDirectory",
  "lspSettings.discovery.installDirectoryPlaceholder",
  "lspSettings.initialization.title",
  "lspSettings.initialization.description", "lspSettings.initialization.placeholder",
  "lspSettings.initialization.invalidJson", "lspSettings.initialization.objectRequired",
  "lspSettings.initialization.tooLarge", "lspSettings.trust.title",
  "lspSettings.trust.description", "lspSettings.trust.explanation",
  "lspSettings.trust.notSandboxed", "lspSettings.trust.rootPlaceholder",
  "lspSettings.trust.grant", "lspSettings.trust.revoke", "lspSettings.trust.empty",
  "lspSettings.trust.trusted", "lspSettings.trust.untrusted",
  "lspSettings.trust.updating", "lspSettings.trust.updateError",
  "lspSettings.test.title", "lspSettings.test.description", "lspSettings.test.run",
  "lspSettings.test.running", "lspSettings.test.succeeded", "lspSettings.test.failed",
  "lspSettings.test.phase.discovery", "lspSettings.test.phase.spawn",
  "lspSettings.test.phase.initialize", "lspSettings.test.phase.cleanup",
  "lspSettings.test.status.succeeded", "lspSettings.test.status.failed",
  "lspSettings.test.status.skipped", "lspSettings.runtime.title",
  "lspSettings.runtime.description", "lspSettings.runtime.empty",
  "lspSettings.runtime.refresh", "lspSettings.runtime.relativeProjectRoot",
  "lspSettings.runtime.restartCount", "lspSettings.runtime.lastResponse",
  "lspSettings.runtime.diagnostics", "lspSettings.runtime.capabilities",
  "lspSettings.runtime.unsupportedMetrics", "lspSettings.runtime.never",
  "lspSettings.state.absent", "lspSettings.state.starting",
  "lspSettings.state.initializing", "lspSettings.state.ready",
  "lspSettings.state.stopping", "lspSettings.state.backoff", "lspSettings.state.failed",
  "lspSettings.capability.positionEncoding", "lspSettings.capability.documentSync",
  "lspSettings.capability.definition", "lspSettings.capability.references",
  "lspSettings.capability.hover", "lspSettings.capability.diagnostics",
  "lspSettings.capability.type_definition", "lspSettings.capability.implementation",
  "lspSettings.capability.workspace_symbols", "lspSettings.capability.document_symbols",
  "lspSettings.capability.call_hierarchy",
  "lspSettings.capability.enabled", "lspSettings.capability.disabled",
  "lspSettings.reason.executable_not_found", "lspSettings.reason.override_missing",
  "lspSettings.reason.override_not_executable", "lspSettings.reason.executable_unavailable",
  "lspSettings.reason.minimal_project_failed", "lspSettings.reason.spawn_failed",
  "lspSettings.reason.initialize_failed", "lspSettings.reason.initialize_timed_out",
  "lspSettings.reason.forced_termination", "lspSettings.reason.cleanup_failed",
  "lspSettings.reason.invalid_deadline", "lspSettings.reason.restart_exhausted",
  "lspSettings.reason.protocol_limit", "lspSettings.reason.request_timeout",
  "lspSettings.reason.cancelled", "lspSettings.reason.untrusted",
  "lspSettings.reason.unsupported_method", "lspSettings.reason.invalid_configuration",
  "lspSettings.reason.unsupported_on_this_platform",
  "lspSettings.reason.prerequisite_missing",
  "lspSettings.reason.install_directory_not_set",
  "lspSettings.reason.launcher_not_found",
  "lspSettings.reason.ambiguous_install",
  "lspSettings.reason.install_refused", "lspSettings.reason.install_failed",
  "lspSettings.reason.checksum_mismatch",
  "lspSettings.install.title", "lspSettings.install.installed",
  "lspSettings.install.notInstalled", "lspSettings.install.unverified",
  "lspSettings.install.action", "lspSettings.install.remove",
  "lspSettings.install.working",
] as const;

describe("LSP settings localization", () => {
  it.each(Object.entries(resources))("provides complete %s settings copy", (language, resource) => {
    for (const key of requiredKeys) {
      const value = resource[key];
      expect(value, `${language}:${key}`).toEqual(expect.any(String));
      expect(value?.trim(), `${language}:${key}`).not.toBe("");
    }
  });

  it.each([
    ["en", "operating-system permissions", "not an operating-system sandbox"],
    ["zh-CN", "操作系统权限", "不是操作系统沙箱"],
    ["zh-TW", "作業系統權限", "不是作業系統沙箱"],
    ["ja", "OS 権限", "OS サンドボックスではありません"],
    ["ko", "운영 체제 권한", "운영 체제 샌드박스가 아닙니다"],
  ] as const)("states the trust boundary explicitly in %s", (language, permission, sandbox) => {
    expect(resources[language]["lspSettings.trust.explanation"]).toContain(permission);
    expect(resources[language]["lspSettings.trust.notSandboxed"]).toContain(sandbox);
  });

  // A translation that drops the negation reads as a reassurance about a download nobody verified.
  it.each([
    ["en", "not checksum-verified"],
    ["zh-CN", "未做校验和验证"],
    ["zh-TW", "未做總和檢查碼驗證"],
    ["ja", "検証されていません"],
    ["ko", "검증되지 않습니다"],
  ] as const)("says the managed download is unverified in %s", (language, negation) => {
    expect(resources[language]["lspSettings.install.unverified"]).toContain(negation);
  });
});
