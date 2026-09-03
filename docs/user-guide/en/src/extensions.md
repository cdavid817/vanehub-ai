# Local extensions

Installing, enabling, and disabling local extensions, plus the built-in product integrations and their readiness checks.

Configured in the same settings centre but documented in their own chapters: [MCP servers](mcp.md), [Prompt Hooks](prompt-hooks.md), [Skill management](skill-management.md), and [Agent and CLI configuration](agent-configuration.md).

## Extension capabilities

What **Settings → Extension Capabilities** installs is **local multimodal AI capability**, not general-purpose plugins. The first release provides one built-in allowlisted framework per capability:

| Capability | Framework | Runtime | Local port | Estimated disk |
| --- | --- | --- | --- | --- |
| **OCR** | PaddleOCR | Python 3.10+ | 9875 | **~1800 MB** |
| **Speech Recognition** | faster-whisper | Python 3.10+ | 9876 | **~900 MB** |
| **Speech Synthesis** | sherpa-onnx | Python 3.10+ | — | — |

**Check two things before installing**: you need Python 3.10+ on the machine, and **the disk footprint is not small** — PaddleOCR is close to 1.8 GB. Every framework card has an expandable "installation requirements" section.

The top of the page has three counters, **Installed / Running / Errors**; when something errors, check the operation logs for the reason.

![The Extension Capabilities settings page with the PaddleOCR and faster-whisper framework cards](assets/screenshots/extensions-en.png)

## Plugin integration

**Settings → Plugin Integration** manages built-in product integrations and their readiness checks — note that it **does not install third-party plugin packages**. The first release ships one built-in integration, GitHub, which checks the local `gh` CLI's authentication status. The five statuses, how to enable it, and the Web-mode limitation are all in [Plugin integration](plugin-integration.md).

## Notes and limits

- **Desktop only.**
- **Extension capabilities do not rewrite a CLI's own configuration file**; binding works through launch parameters and the relay.
