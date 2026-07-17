# Personal WeChat Release Review

Reviewed: 2026-07-17

## Evidence

- Tencent publishes `@tencent-weixin/openclaw-weixin` under the `@tencent-weixin` npm scope. The reviewed current package version is `2.4.6`.
- The package includes a Tencent copyright notice and the MIT license, which permits use, modification, publishing, distribution, sublicensing, and sale subject to retaining the notice and license.
- The official package describes QR authorization, iLink long polling, contextual replies, and application attribution through `bot_agent`; VaneHub identifies its traffic as `VaneHub/0.1.0`.
- The official package is explicitly packaged as an OpenClaw channel and declares an OpenClaw peer/runtime compatibility range. VaneHub reimplements the narrow HTTP protocol and does not redistribute or load that package.
- No separate public iLink service/distribution terms were discoverable from the iLink endpoint or official package at review time.

## Gate Decision

Keep the VaneHub connector visibly **experimental**, disabled by default, and opt-in through explicit QR authorization. Do not describe ordinary personal-account automation, group support, or proactive messaging as supported. Release readiness still requires the opt-in packaged-app live smoke test in `docs/architecture/im-connectors-smoke.md`; a future Tencent terms or protocol change must trigger another review.

Sources:

- <https://www.npmjs.com/package/@tencent-weixin/openclaw-weixin>
- `@tencent-weixin/openclaw-weixin@2.4.6` package `LICENSE`, `README.zh_CN.md`, and `package.json`
