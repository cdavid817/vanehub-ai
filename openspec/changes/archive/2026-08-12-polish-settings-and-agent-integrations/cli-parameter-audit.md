# CLI Parameter Audit

Audit date: 2026-08-12.

The managed catalog intentionally covers reusable launch defaults and excludes one-shot prompt, resume/session, output formatting, diagnostic logging, and policy-governed approval/sandbox flags. Sources were each installed CLI's `--help` output plus the upstream CLI references linked below.

| CLI | User-editable parameters | Policy-owned parameters | Audit note |
| --- | --- | --- | --- |
| Claude Code | `--model`, `--effort`, interactive `--chrome` | `--permission-mode` | Model remains custom text; documented aliases include sonnet, opus, and haiku. |
| Codex CLI | `--model`, `--config model_reasoning_effort`, chat `--ephemeral`, `--strict-config` | `--sandbox`, `--ask-for-approval` | Reasoning effort is rendered as a TOML config override. |
| OpenCode | `--model`, chat `--variant`, chat `--thinking` | `--agent`, `--auto` | Model uses `provider/model`; variant is provider-specific and therefore accepts custom text. |
| Antigravity CLI | `--model`, `--effort`, `--agent` | settings-backed mode and sandbox | No bypass flag is exposed; approval remains profile/policy controlled. |
| Gemini CLI | `--model` | `--approval-mode`, `--sandbox` | Known model aliases are `auto`, `pro`, `flash`, and `flash-lite`; arbitrary model ids remain valid. |

Primary references:

- Anthropic Claude Code CLI reference: https://docs.anthropic.com/en/docs/claude-code/cli-usage
- OpenAI Codex CLI reference: https://developers.openai.com/codex/cli/reference
- OpenCode CLI reference: https://opencode.ai/docs/cli/
- Gemini CLI reference: https://geminicli.com/docs/cli/cli-reference/
