// One-shot helper: adds the strict-cleanup refusal copy to every registered locale, in the same
// position everywhere so the parity test compares like with like.
import { readFileSync, writeFileSync } from "node:fs";

const strings = {
  en: {
    "layout.archiveBlockedByShellCleanup": "This session cannot be archived yet",
    "layout.deleteBlockedByShellCleanup": "This session cannot be deleted yet",
    "layout.shellCleanupStillFinishing":
      "A terminal in this session has not confirmed it stopped. The session is kept so you can still reach it; try again in a moment.",
  },
  "zh-CN": {
    "layout.archiveBlockedByShellCleanup": "该会话暂时无法归档",
    "layout.deleteBlockedByShellCleanup": "该会话暂时无法删除",
    "layout.shellCleanupStillFinishing":
      "该会话中有终端尚未确认已停止。会话被保留，你仍然可以进入它；请稍后重试。",
  },
  "zh-TW": {
    "layout.archiveBlockedByShellCleanup": "該工作階段暫時無法封存",
    "layout.deleteBlockedByShellCleanup": "該工作階段暫時無法刪除",
    "layout.shellCleanupStillFinishing":
      "該工作階段中有終端機尚未確認已停止。工作階段被保留，你仍然可以進入它；請稍後重試。",
  },
  ja: {
    "layout.archiveBlockedByShellCleanup": "このセッションはまだアーカイブできません",
    "layout.deleteBlockedByShellCleanup": "このセッションはまだ削除できません",
    "layout.shellCleanupStillFinishing":
      "このセッションのターミナルが停止を確認していません。セッションは保持されているのでまだ開けます。しばらくしてからもう一度お試しください。",
  },
  ko: {
    "layout.archiveBlockedByShellCleanup": "이 세션은 아직 보관할 수 없습니다",
    "layout.deleteBlockedByShellCleanup": "이 세션은 아직 삭제할 수 없습니다",
    "layout.shellCleanupStillFinishing":
      "이 세션의 터미널이 아직 중지를 확인하지 않았습니다. 세션은 유지되어 계속 열 수 있습니다. 잠시 후 다시 시도하세요.",
  },
};

const anchor = "layout.batchDeleteFailedMessage";

for (const [locale, values] of Object.entries(strings)) {
  const path = `src/i18n/locales/${locale}.json`;
  const data = JSON.parse(readFileSync(path, "utf8"));
  if (!(anchor in data)) throw new Error(`${locale}: anchor ${anchor} not found`);
  const out = {};
  for (const [key, value] of Object.entries(data)) {
    out[key] = value;
    if (key === anchor) for (const [k, v] of Object.entries(values)) out[k] = v;
  }
  writeFileSync(path, `${JSON.stringify(out, null, 2)}\n`, "utf8");
  console.log(locale, "->", Object.keys(out).length);
}
