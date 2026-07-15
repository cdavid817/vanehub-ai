# GEMINI.md

@AGENTS.md

<!--
  本文件是 Gemini CLI 的项目上下文入口文件。
  所有通用规则统一维护在根目录 AGENTS.md 中,上面这行 @AGENTS.md 会在 Gemini CLI
  加载项目上下文时自动导入其内容,请不要在这里重复书写规则。

  如果需要用符号链接代替 @import:
      ln -s AGENTS.md GEMINI.md
-->

## Gemini CLI 专属补充(仅本工具适用,不放进 AGENTS.md)

- 深度实现示例参考 `.codex/skills/` 与 `.claude/skills/` 下的说明(内容对所有 Agent 通用,非 Claude 专属)
- 涉及 OpenSpec 工作流时,优先阅读 `openspec/project.md` 和对应 `openspec/changes/<change-id>/` 下的 proposal
