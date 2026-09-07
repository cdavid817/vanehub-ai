# Live gate 登录操作指南（Kimi / Qoder / CodeBuddy / Copilot / Cursor）

目的：为 tasks 6.4、7.4、8.5 的 live smoke 解除「未登录」阻塞。登录由你在终端里完成，VaneHub 不代登录、不读取你的凭据文件；登录态由各 CLI 自己存放（系统钥匙串或各自的配置目录），VaneHub 启动 ACP 子进程时原样继承，不需要在应用内再配置。

本机已安装版本（2026-09-07）：Kimi Code 0.41.0、Qoder 1.1.45、CodeBuddy 2.146.0、Copilot 1.0.83、Cursor Agent 2026.09.02。

## 0. 前置

本会话的 shell 里 `~/.npm-global/bin` 不在 PATH，你的终端可能也一样。先执行：

```bash
export PATH="$HOME/.npm-global/bin:$HOME/.local/bin:$PATH"
```

五家都是"设备码 / 浏览器 OAuth"式登录，需要能打开浏览器或复制链接到能登录的浏览器。全程不会产生模型调用；登录后我跑的 live gate 会产生少量付费调用（每家约 5–8 轮短 prompt）。

## 1. Kimi Code CLI（`kimi`）

```bash
kimi login --region mainland-cn   # 国内账号（kimi.com）
# 或
kimi login --region global        # 国际账号（kimi.ai）
```

按提示打开链接、输入设备码完成授权。验证（不调模型）：

```bash
kimi doctor
```

官方文档：<https://www.kimi.com/code/docs/en/kimi-code-cli/guides/getting-started.html>

## 2. Qoder CLI（`qoder` / `qodercli`）

```bash
qoder login
```

按提示完成浏览器授权。验证：

```bash
qoder status
```

官方文档：<https://docs.qoder.com/cli/installation>

## 3. CodeBuddy Code（`codebuddy` / `cbc`）

没有独立的 `login` 子命令。直接启动交互式会话，首次进入会提示登录（腾讯云 / CodeBuddy 账号，浏览器授权）；已进入会话时可在输入框输入 `/login`：

```bash
codebuddy
```

看到欢迎界面且没有登录提示即完成；用 `/status`（会话内）或退出后再次启动确认。

官方文档：<https://www.codebuddy.ai/docs/cli/quickstart>

## 4. GitHub Copilot CLI（`copilot`）

```bash
copilot login
```

本机是桌面环境，默认走浏览器回调；如浏览器打不开，改用设备码：

```bash
copilot login --device-code
```

需要 GitHub 账号拥有 Copilot 订阅（个人版或组织版）。令牌会存进系统凭据库。验证：再次执行 `copilot login`，已登录会直接提示当前账号，或启动 `copilot` 看是否要求登录。

官方文档：<https://docs.github.com/en/copilot/how-tos/copilot-cli/set-up-copilot-cli/install-copilot-cli>

## 5. Cursor Agent CLI（`agent` / `cursor-agent`）

```bash
agent login
```

会打开浏览器；无法打开时：

```bash
NO_OPEN_BROWSER=1 agent login
```

然后手动打开输出的链接。验证：

```bash
agent status
agent models
```

需要 Cursor 账号（Pro 或以上才能调用模型）。官方文档：<https://cursor.com/docs/cli/installation>

## 6. 登录完成后告诉我，我会做的事

1. 先用不调模型的探针确认五家的登录态（每家只做 `initialize` + `session/new`，不发 prompt）：

   ```bash
   cd src-tauri && VANEHUB_LIVE_CLI=1 cargo test --lib -- live_session_new_without_sign_in_is_classified_not_guessed --nocapture
   ```

   输出里每家一行 `LIVE-SESSION <agent>: session/new ACCEPTED …` 表示登录生效；`REFUSED: acp-authentication-required` 表示仍未登录。

2. 对登录成功的每一家跑完整 live gate（文本轮次、工具审批、策略拒绝、取消、重启后恢复），例如：

   ```bash
   VANEHUB_LIVE_CLI=1 VANEHUB_LIVE_PROMPT_AGENT=kimi-cli cargo test --lib -- live_prompt_turn_completes_through_the_acp_adapter live_tool_approval_and_cancel_through_the_acp_adapter live_resume_across_a_host_restart_through_the_acp_adapter --nocapture
   ```

   Agent id 依次为 `kimi-cli`、`qoder-cli`、`codebuddy-code`、`copilot-cli`、`cursor-agent-cli`。

3. 把每家的版本、结果与耗时记入 `results.md` 与 `provider-matrix.md`，并按实际结果勾选或保留 6.4 / 7.4 / 8.5。

## 7. 注意

- 登录只发生在你的终端；VaneHub 不会修改、备份或读取这些凭据。
- 五家的 live gate 只在 Linux 本机执行；Windows / macOS 仍记 NOT RUN。
- 想撤销：Cursor 用 `agent logout`；Copilot 本机版本没有 `logout` 子命令，凭据在系统凭据库里按其文档删除；其余各 CLI 配置目录内的凭据文件由其自身管理，VaneHub 不会碰。
