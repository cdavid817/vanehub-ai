# OnePiece（原生 Agent）

## 功能概述

**OnePiece 是唯一不依赖外部 CLI 的内置 Agent**。它直接通过 HTTP 调用模型 provider，因此没装任何 CLI 也能开始使用。

它还承担一项幕后职责：**为其他 CLI Agent 代做记忆提取**——这意味着即便你主力用 Claude Code，也需要配好 OnePiece 才能有记忆功能。

## 配置 provider

在**设置 → Agent 配置**中打开 OnePiece 配置面板：

1. 在 provider 目录中选择厂商，或配置自定义兼容端点。
2. 填入 API Key。**保存前会实际调用一次做凭据校验**，不通过不会保存。
3. 校验通过后会拉取该 provider 的可用模型列表。
4. 选定模型。目录条目已提供默认模型与备选列表。

**可选的 provider 有 25 家**，分两类：

| 类别 | 条目 |
| --- | --- |
| 官方 | Anthropic、OpenAI |
| 常用 | OpenRouter、DeepSeek、Zhipu GLM、Kimi / Moonshot、SiliconFlow、阿里百炼、火山方舟、Groq、xAI、Mistral、Together AI、Fireworks、NVIDIA NIM、Cerebras、MiniMax（国内 / 全球）、StepFun、百川、PPIO、七牛、ModelScope、小米 MiMo、Z.AI |

每个条目都带**申请 API Key 的链接**与**官方文档链接**，界面上可以直接点。

## 在会话中使用

创建会话时选择 Agent **OnePiece**。未完成 provider 配置时它显示为不可用，提示「OnePiece requires provider configuration.」。

## 记忆检索（recall）

OnePiece 可以对积累的记忆做**混合检索**——同时用向量与关键词两路召回，再融合排序。

在同一配置面板的检索区配置嵌入模型与索引设置。

### 检索的对象是记忆，不是项目代码

这是最容易误解的一点：

> **recall 检索的是你积累下来的记忆，不是仓库文件**。它不会索引项目源码或文档。

如果某一路不可用（例如嵌入服务故障），检索会**自动退化**为只用另一路，并明确标记降级状态，而不是整体失败。

## Notebook 编辑

OnePiece 可以**按单元格**读写 Jupyter Notebook（`.ipynb`），而不是把整个文件当成一大团 JSON 塞进上下文。

**读取返回的是单元格，不是 notebook JSON**。每个单元格的输出会被摘要化：

| 输出类型 | 读取结果里有什么 |
| --- | --- |
| 图片等二进制 | **只有媒体类型和大小** |
| 错误 | 保留错误名与错误值 |
| 文本 | 文本内容，有长度上限 |

**输出的字节永远不会进入读取结果**，也不会以任何编码形式混进去。一个几 MB 的内嵌图片不会把上下文撑爆。

编辑时**不需要拼装 notebook JSON**，只改指定单元格，**其余部分原样不动**。

有一条行为需要特别知道：**改动代码单元格的源码会清空该单元格的输出和执行计数**。这是刻意的——否则文件会继续展示一个当前源码已经产生不出来的结果。Markdown 单元格不带执行状态，不受影响。

目标文件不是合法 notebook 时（JSON 不合法、没有单元格序列、或声明了不支持的 notebook 格式），**操作被拒绝并说明是哪一种，文件保持不变**。

Notebook 访问同样遵守工作区边界和 Plan 模式——**Plan 模式下只读**。

## 与外部 CLI 的差异

| 维度 | 外部 CLI Agent | OnePiece |
| --- | --- | --- |
| 运行形态 | 独立进程 | 应用内 |
| 需要预装 | 是 | 否 |
| 执行追踪可见度 | 只能看到边界（不可见） | **原生保真度，可展开** |
| 工具调用 | CLI 自己处理 | 应用内可观测 |

**想细看一次执行到底做了什么，OnePiece 的追踪信息量明显更高。**

## Plan 模式与 Agent 模式

OnePiece 的输入区会始终显示带图标和文字的模式标签：

- `Plan · 只读`：在当前项目范围内读取、搜索和分析代码，不修改文件。shell 执行、文件写入、有副作用的 MCP 工具和委派工作均不可用。
- `Agent · 可写`：在当前会话配置的工作区与策略内修改文件并执行受控校验。

OnePiece 准备开始操作时会请求 `exit_plan_mode`。批准只让后续轮次进入 Agent 模式；拒绝后仍停留在 Plan 模式。这个转换只修改会话配置，不会创建 PlanRun、任务图或 worktree。

## 注意事项与限制

- **仅桌面端可用。**
- **必须先配 provider**，否则 Agent 不可用，且 CLI Agent 的记忆提取也一并失效。
- **检索需要可用的嵌入服务**；不可用时退化为纯关键词检索。
- **很长的记忆只有前面一段参与向量检索**，尾部仍可被关键词命中。
- **模型目录是静态的**，provider 新增模型可能需要目录更新，或靠模型发现动态拉取。
- **OnePiece 的会话不能迁移到 CLI Agent**，反之亦然。

## 相关

- provider 配置与凭据保管 → [工具与扩展](tooling.md#agent-配置)
- 记忆提取与上下文压缩 → [记忆与上下文](memory-and-context.md)
- 工具调用技术本身：调用循环、约束解码、并行调用与跨 Provider 适配 → [Function Calling 技术架构](../../../agent-infrastructure/function-calling-architecture.md)
