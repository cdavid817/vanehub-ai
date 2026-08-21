# OnePiece (native Agent)

**Status: Implemented — desktop only.**

## Overview

**OnePiece is the only built-in Agent that does not depend on an external CLI.** It calls a model provider over HTTP directly, so you can start using it with no CLI installed at all.

It also carries one behind-the-scenes responsibility: **it performs memory extraction on behalf of the other CLI Agents** — which means that even if you mainly use Claude Code, you still need OnePiece configured for memory to work.

## Configure a provider

Open the OnePiece configuration panel under **Settings → Agent Configurations**:

1. Choose a vendor from the provider catalog, or configure a custom compatible endpoint.
2. Enter the API key. **A real call is made to validate the credential before saving**; it is not saved if validation fails.
3. Once validated, the provider's available model list is fetched.
4. Select a model. Catalog entries already supply a default model and alternatives.

**There are 25 providers to choose from**, in two groups:

| Group | Entries |
| --- | --- |
| Official | Anthropic, OpenAI |
| Common | OpenRouter, DeepSeek, Zhipu GLM, Kimi / Moonshot, SiliconFlow, Alibaba Bailian, Volcano Ark, Groq, xAI, Mistral, Together AI, Fireworks, NVIDIA NIM, Cerebras, MiniMax (China / global), StepFun, Baichuan, PPIO, Qiniu, ModelScope, Xiaomi MiMo, Z.AI |

Every entry carries a **link to apply for an API key** and a **link to the official documentation**, both selectable in the interface.

## Use it in a session

Choose the Agent **OnePiece** when creating a session. Until provider configuration is complete it shows as unavailable, with the message "OnePiece requires provider configuration."

## Memory recall

OnePiece can perform **hybrid retrieval** over accumulated memories — recalling through both a vector path and a keyword path, then fusing and ranking the results.

Configure the embedding model and index settings in the retrieval section of the same configuration panel.

### What it retrieves is memories, not project code

This is the easiest thing to misread:

> **Recall retrieves the memories you have accumulated, not repository files.** It does not index project source or documentation.

If one path is unavailable — a failing embedding service, for example — retrieval **degrades automatically** to the other path alone and marks the degraded state explicitly, rather than failing outright.

## Notebook editing

OnePiece can read and write Jupyter notebooks (`.ipynb`) **cell by cell**, rather than pushing the whole file into context as one blob of JSON.

**A read returns cells, not notebook JSON.** Each cell's outputs are summarized:

| Output type | What the read result contains |
| --- | --- |
| Image or other binary | **Only the media type and size** |
| Error | The error name and value |
| Text | The text, up to a declared bound |

**Output bytes never enter the read result**, nor any encoding of them. A multi-megabyte embedded image cannot blow up your context.

Editing **does not require composing notebook JSON**; only the named cell changes, and **everything else is left exactly as it was**.

One behavior is worth knowing specifically: **changing a code cell's source clears that cell's outputs and execution count**. This is deliberate — otherwise the file would keep displaying a result its current source can no longer produce. Markdown cells carry no execution state and are unaffected.

When the target file is not a valid notebook — invalid JSON, no cell sequence, or an unsupported declared notebook format — **the operation is refused with the reason stated, and the file is left unchanged**.

Notebook access observes the workspace boundary and Plan mode as well — **Plan mode is read-only**.

## Differences from external CLIs

| Dimension | External CLI Agent | OnePiece |
| --- | --- | --- |
| Form | A separate process | In-application |
| Needs preinstalling | Yes | No |
| Execution-trace visibility | Only the boundary (opaque) | **Native fidelity, expandable** |
| Tool calls | Handled by the CLI itself | Observable in-application |

**If you want to see in detail what one run actually did, OnePiece's trace carries substantially more information.**

## Plan mode and Agent mode

OnePiece's input area always shows a mode label with an icon and text:

- `Plan · Read-only`: inspect the workspace and prepare a reviewable plan without changing files. Shell execution, file writes, effectful MCP tools, and delegated work are unavailable.
- `Agent · Can edit`: edit files and run guarded validation within the current session's configured workspace and policy.

When OnePiece is ready to act, it requests `exit_plan_mode`. Approving applies Agent mode only to a later turn; declining leaves the session in Plan mode. This transition changes the session configuration only and does not create a PlanRun, task graph, or worktree.

## Notes and limits

- **Desktop only.**
- **A provider must be configured first**, or the Agent is unavailable — and CLI Agent memory extraction stops working along with it.
- **Retrieval needs a working embedding service**; without one it degrades to keyword-only retrieval.
- **Only the beginning of a very long memory takes part in vector retrieval**; the tail can still be hit by keywords.
- **The model catalog is static**, so a provider's new models may need a catalog update, or dynamic fetching through model discovery.
- **A OnePiece session cannot be migrated to a CLI Agent**, or the reverse.

## Related

- Provider configuration and credential storage → [Tools and extensions](tooling.md#agent-configurations)
- Memory extraction and context compaction → [Memory and context](memory-and-context.md)
- Tool calling technology itself: the call loop, constrained decoding, parallel calls, and cross-provider adaptation → [Function Calling technical architecture](../../../agent-infrastructure/function-calling-architecture.md) (Simplified Chinese)
