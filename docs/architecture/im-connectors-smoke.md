# IM Connectors Packaged-App Smoke Test

Run this checklist against a packaged desktop build. Do not place credentials in source files, shell history, screenshots, issue text, or logs.

## Opt-in environment checklist

- Set `VANEHUB_IM_LIVE_SMOKE=1` only in the terminal used to record the test run. This is an operator acknowledgement; live tests are never run by the normal automated suite.
- Set `VANEHUB_IM_SMOKE_CONNECTORS=feishu,telegram,dingtalk,wecom,weixin`, or a comma-separated subset, to record the intended scope.
- Install a packaged VaneHub build and use a disposable test project with a non-production CLI Agent account.
- Enter platform credentials only through IM Settings so the operating-system credential store receives them. Do not put tokens in environment variables.
- Use dedicated platform test applications/bots and direct-message test accounts. Remove or rotate their credentials after the run.
- Keep unified logging at `info` unless diagnosing a failure, and inspect the resulting logs for redaction before retaining them.
- Record app version, OS, connector, UTC timestamp, pass/fail, and safe error code only. Do not record external user ids, message text, QR data, tokens, or protocol frames.

The live check is intentionally operator-driven: the platform-side test account must send and receive the direct message, which cannot be safely automated without storing another account's credentials.

## Tray lifecycle

1. Start VaneHub AI and configure a default CLI Agent and an existing project.
2. Close the main window and confirm the localized first-close notice appears.
3. Confirm the process remains alive and the tray menu provides Show, Hide, and Quit in the configured language.
4. Restore the existing window from the tray icon, then hide it from the tray menu.
5. Enable one connector, close the window, and confirm a direct text message is still received while the window is hidden.
6. Choose Quit and confirm the process exits within eight seconds and no connector keeps running.

## Connector round trips

Live checks are opt-in and require credentials supplied through the settings page:

| Connector | Required setup | Expected check |
| --- | --- | --- |
| Feishu | App ID and App Secret; application event long connection enabled | One private text produces one final-only reply |
| Telegram | Bot token; no active webhook | One private text produces one final-only reply |
| DingTalk | App Key, App Secret, and optional Robot Code; Stream enabled | One private text produces one final-only reply |
| WeCom | Intelligent Bot ID and Secret | One private text produces one final-only reply |
| Personal WeChat | Complete in-app QR authorization | One private text produces one final-only reply; expired authorization requests reauthorization |

For every connector, confirm group and non-text messages do not start Agent work, the session card shows only the localized platform label, and the unified log contains no credential, QR payload, external identity, prompt, or response content.

Record one result row per requested connector:

| App version | OS | Connector | UTC time | Result | Safe error code |
| --- | --- | --- | --- | --- | --- |
| | | | | pass / fail | none / redacted code |
