# 管理 Skill

**状态：已交付——桌面端与 Web/mock 界面。** 桌面端会执行本地持久化、CLI Skill 挂载和 API Agent 提示词绑定；Web/mock 只模拟相同界面和状态变化，不会修改本机文件或运行时配置。

![设置中的 Skill 管理页面，左侧按 Agent 分组，右侧 Skill 列表](../assets/screenshots/skills-zh-CN.png)

## 理解列表与状态

“是否启用”和“分配给哪个 Agent”是两个相互独立的维度。

| 界面或状态 | 含义 |
| --- | --- |
| 全部 Skill | 当前全局 Skill 库中的所有 Skill，无论它是否启用、是否已分配 |
| 未分配 | 没有分配给任何 CLI Agent 或 API Agent 的 Skill；启用状态不影响这一判断 |
| Agent 页面 | 管理所选 Agent 的“已分配”和“可分配”Skill |
| 已启用 | Skill 级总开关，允许已经分配该 Skill 的 Agent 使用它 |
| 已分配 | 该 Skill 与一个具体 Agent 的关系；每个 Agent 的分配相互独立 |

“已启用”不等于“已分配给所有 CLI”。启用一个尚未分配的 Skill 不会让任何 Agent 自动使用它，也不会让它离开“未分配”列表。

关闭 Skill 会让所有已分配 Agent 暂停使用它，但不会删除这些分配关系。重新启用后，只有原来已经分配的 Agent 恢复使用；未分配的 Agent 不受影响。

## 典型结果

| 配置 | 结果 |
| --- | --- |
| Skill A 已启用，但未分配 | 没有 Agent 使用它，并继续显示在“未分配”中 |
| Skill B 已启用，只分配给 Codex | Codex 可以使用，其他 Agent 不可以 |
| Skill C 已关闭，已分配给 Codex 和 Claude | 两个 Agent 都暂停使用，但分配关系保留 |
| 重新启用 Skill C | Codex 和 Claude 恢复使用，其他 Agent 不会自动获得它 |

## 启用并分配 Skill

1. 打开“设置”中的“Skill”。
2. 在“全部 Skill”中找到目标 Skill，并确认“已启用”处于开启状态。
3. 在左侧选择一个 CLI Agent 或 API Agent。
4. 在“可分配”面板找到该 Skill，然后选择“分配”。
5. 确认该 Skill 移到所选 Agent 的“已分配”面板。

如果多个 Agent 都需要使用同一个 Skill，请逐个进入对应 Agent 页面完成分配。关闭“全部 Skill”中的“已启用”会暂停所有这些 Agent；它不是单个 Agent 的开关。

## Runtime 差异

- **桌面端：** 为 CLI Agent 分配 Skill 可能会在其 Skill 挂载目录执行文件系统操作；API Agent 分配会保存提示词绑定。操作失败时，错误会留在对应 Skill 行，分配状态不会提前变化。
- **Web/mock：** 可以验证筛选、分配、移除和响应式界面，但所有结果都是内存模拟，不能作为本机文件或 native 配置已经改变的证据。
