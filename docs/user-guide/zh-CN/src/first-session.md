# 创建第一个会话

**状态：已交付——桌面端与 Web/mock 的 side effect 不同。**

1. 打开 VaneHub AI。
2. 选择**新建**。
3. 选择**单 Agent**。
4. 选择一个可用的 Agent。
5. 选择项目目录并填写会话标题。
6. 创建会话，然后使用工作区终端。

![使用合成的 VaneHub 示例项目数据创建中文会话](../assets/screenshots/create-session-zh-CN.png)

桌面端的项目路径对应真实目录，Agent 执行使用已安装的所选 CLI。Web/mock 中的相同流程使用合成状态和模拟终端，不会启动本地进程，也不会修改 SQLite 数据库。

当前禁用的**多 Agent**选项不属于此已交付工作流。请阅读[多 Agent 预览](multi-agent-workflow.md)。
