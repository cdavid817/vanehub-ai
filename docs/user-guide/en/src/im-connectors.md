# IM connectors

Connecting VaneHub AI to Feishu, Telegram, DingTalk, WeCom, or personal WeChat so that a chat message can start a session.

**Only a text direct message triggers execution in this first release** — group messages and non-text content are acknowledged, but create no session and start no Agent generation.

For SSH remote workspaces see [Remote workspaces and SSH](remote-workspaces.md).

## Supported platforms

Five connectors can be configured under **Settings → IM Connectors**:

| Connector | How it connects | Status |
| --- | --- | --- |
| **Feishu** | Feishu app long connection | — |
| **Telegram** | Telegram Bot API long polling | — |
| **DingTalk** | DingTalk Stream bot | — |
| **WeCom** | WeCom smart bot WebSocket | — |
| **Personal WeChat** | iLink QR authorization | **Experimental** |

Before configuring, you need to create an application on the corresponding open platform and obtain its credentials.

![The IM Connectors settings page with the default route and five connectors](assets/screenshots/im-en.png)

## Configure the default route first

**This is a precondition for enabling any connector**, and the interface says so directly: **"This must be configured before any connector can be enabled."**

Set two values in the **default route** section and save:

| Field | Notes |
| --- | --- |
| **Default Agent** | Chosen from the available CLI Agents |
| **Default project** | The project directory |

New external chats create their own sessions using these two defaults; **existing bindings are unaffected**. Try to enable a connector before configuring it and the interface tells you to configure and save first.

## Configure a connector

Enter the application credentials. **Fields marked secret are not echoed back after saving** — they live in the system keychain, and the interface keeps only a reference.

WeChat goes through a QR authorization flow, with the interface showing a QR code and polling the state. The states are: awaiting scan → scanned → confirmed. **"Scanned" and "confirmed" are separate**, and the interface tells you when you have scanned the code but not confirmed on your phone.

## Connection state

Once enabled successfully, a connector shows a **connected** badge and an update time.

Connectors have seven states, two of which are easy to confuse:

- **Authorization expired** — an expected, normal condition; re-authorize
- **Error** — a network or configuration fault

They are displayed separately and never conflated.

![The Feishu connector shown in a connected state on the IM page](assets/screenshots/im-connected-en.png)

## Sessions triggered from IM

**Only a text direct message triggers execution in this first release.** Each connector accepts text direct messages only; group messages and non-text content are acknowledged or consumed but create no session and start no Agent generation. Mentioning the bot in a group will not put it to work — send it a text message in a direct chat.

A session created by a connector is marked with its source, distinguishing it from sessions you created by hand on the desktop. The execution trace also records which connector triggered it.

## Notes and limits

- **Desktop only**, and it depends on the native network stack and the system credential store.
- **Default routing must be configured first**, or no connector can be enabled.
- **Personal WeChat is marked experimental** — it uses iLink QR authorization and is expected to be less stable than the other four.
- **A connector needs that platform's application credentials**, so create the application on its open platform first.
- **What a chat app can show is limited by its form** — desktop-only views such as file browsing and terminal interaction are unavailable there.
- A connector session **cannot be activated as the current desktop session**.
