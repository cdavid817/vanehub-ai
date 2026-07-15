# CLAUDE.md

@AGENTS.md

<!--
  本文件是 Claude Code 的项目入口文件。
  所有通用规则(技术栈、代码规范、文件结构、变更流程)统一维护在根目录 AGENTS.md 中,
  上面这行 @AGENTS.md 会让 Claude Code 启动时自动导入其内容,请不要在这里重复书写规则。

  如果你的系统不支持 @import 语法(或希望用符号链接代替),
  可以删除本文件后执行:
      ln -s AGENTS.md CLAUDE.md
  两种方式效果等价,导入方式在 Windows 上无需管理员权限/开发者模式,更推荐。
-->

## Claude Code 专属补充(仅本工具适用,不放进 AGENTS.md)

- 深度实现示例、单个功能域的详细 pattern 放在 `.claude/skills/` 下,按需触发,不要塞进本文件或 AGENTS.md
- 涉及 OpenSpec 工作流时(`/opsx:propose`、`/opsx:apply` 等),优先阅读 `openspec/project.md` 和对应 `openspec/changes/<change-id>/` 下的 proposal
- 本地临时性、个人化的指令(不希望提交进仓库)请写在 `CLAUDE.local.md`,并确认其已加入 `.gitignore`
