# Live Feishu qualification

Use this runbook only for the opt-in desktop qualification against a dedicated Feishu tenant. It does not apply to deterministic fixture tests and must not reuse a personal or production application.

## Least-privilege application setup

Create a tenant custom app and enable its bot capability. For VaneHub's direct-text receipt and reply scope, grant only these application permissions:

| Purpose | Feishu permission |
| --- | --- |
| Receive messages sent directly to the bot | `im:message.p2p_msg:readonly` |
| Send replies as the application bot | `im:message:send_as_bot` |

Do not grant the broader `im:message` scope when the two narrow permissions are available. Do not add group-message, contact-directory, user-ID, attachment, card, reaction, or user-agent permissions for this qualification. VaneHub uses the `open_id` and `chat_id` already present in the message event and does not need `contact:user.employee_id:readonly`.

The official [receive-message event reference](https://open.feishu.cn/document/server-docs/im-v1/message/events/receive) lists the read-only direct-message permission and event type. The official [send-message API reference](https://open.feishu.cn/document/server-docs/im-v1/message/create) lists `im:message:send_as_bot`, requires bot capability, and requires the recipient to be inside the bot's availability range.

## Operator setup

1. In the Feishu developer console, create or open the dedicated tenant custom app and enable **Bot** capability.
2. Under **Permissions & Scopes**, add the two permissions above. Remove any unrelated scopes from the qualification app.
3. Under **Events & Callbacks**, select **Use long connection to receive events**. VaneHub uses this mode; do not configure a webhook request URL.
4. Add **Receive message v2.0** under **Messages & Groups**. Confirm the event identifier is `im.message.receive_v1` and the subscription identity is the application identity.
5. Create and publish an application version. Permission, event, bot-capability, and availability changes do not qualify until the version is effective in the test tenant.
6. Restrict the application's availability to the smallest dedicated test-user set. Add only the operator who will open the direct chat unless another tester is required.
7. In Feishu, open a one-to-one chat with the application bot and send a harmless setup message. Obtain that p2p event's `event.message.chat_id` through an authorized diagnostic path; treat it as an external identifier and do not commit or paste it into retained evidence.
8. In VaneHub desktop settings, configure the default Agent and project, save the Feishu App ID and App Secret through the normal write-only settings flow, and enable the connector. In the target session's information panel, turn on IM and pair the dedicated chat.

Feishu's [event-subscription overview](https://open.feishu.cn/document/server-docs/event-subscription-guide/overview) documents long connection and requires publishing after adding events. Its retry guidance also means duplicate delivery is expected and must be included in qualification.

## Preflight and execution

Run from the repository worktree in a fresh PowerShell session. Supply values only at runtime; never place them in a file, command argument, screenshot, issue, or committed shell script.

```powershell
$env:VANEHUB_FEISHU_LIVE_QUALIFICATION = "1"
$env:VANEHUB_FEISHU_TEST_TENANT = Read-Host "Dedicated test tenant" -MaskInput
$env:VANEHUB_FEISHU_APP_ID = Read-Host "Feishu App ID" -MaskInput
$env:VANEHUB_FEISHU_APP_SECRET = Read-Host "Feishu App Secret" -MaskInput
$env:VANEHUB_FEISHU_PERMISSIONS_CONFIRMED = "1"
$env:VANEHUB_FEISHU_LONG_CONNECTION_CONFIRMED = "1"
$env:VANEHUB_FEISHU_TEST_CHAT_ID = Read-Host "Dedicated p2p chat_id" -MaskInput
$env:VANEHUB_FEISHU_LIVE_OPERATOR = "1"
npm run test:desktop:feishu-live
```

With `VANEHUB_FEISHU_LIVE_OPERATOR=1`, the terminal prints each one-time pairing code and safe test message in order. Each action waits for up to ten minutes; do not send the next message early. Without this variable, only credential, authentication, connection lifecycle, and invalid-credential checks run, while the human inbound matrix remains `NOT RUN`.

A Feishu retry reuses the stable event ID. Sending the same text again creates a new event and is not deduplication evidence. This scenario is `PASSED` only when the live connection actually observes a platform redelivery; otherwise it is recorded separately as `BLOCKED`. Deterministic fixture deduplication cannot substitute for it.

The entry point reports `NOT RUN` without explicit opt-in and `BLOCKED` when any prerequisite is missing. A live result is independent from fixture evidence. Retained live artifacts contain only safe status codes and timestamps; credentials, tenant identifiers, chat identifiers, prompts, replies, and raw protocol payloads are forbidden.

After the run, close the PowerShell session or remove all eight `VANEHUB_FEISHU_*` variables. The desktop runner also clears its run-owned credential reference during cleanup. If cleanup reports anything other than `CLEARED`, treat the run as failed and remove the dedicated app credential from VaneHub before retrying.

## Qualification checklist

Record each live scenario separately as `PASSED`, `FAILED`, `BLOCKED`, or `NOT RUN`:

- authentication and long-connection lifecycle;
- direct-text receipt and duplicate delivery;
- single-Agent final reply;
- multi-Agent explicit-seat, default-seat, and invalid-seat routing;
- Unicode-safe reply chunking;
- session disable, re-enable, and desktop restart;
- invalid-credential rejection and recovery.

Fixture success never substitutes for a live result. Stop immediately if an artifact contains a credential, external identifier, message content, or raw event payload.
