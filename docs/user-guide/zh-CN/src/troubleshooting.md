# 故障排查

## CLI 不可用

在普通终端中运行 Provider 命令。如果 shell 无法找到它，请重新安装 CLI 或修正桌面应用可见的 PATH，然后重启 VaneHub AI。

## Agent 要求登录

在 Provider CLI 中完成认证。VaneHub AI 不会保存 Provider 密码。

## 浏览器预览显示操作成功

检查是否带有**仅 Web/mock**标签。浏览器预览使用确定性模拟，不能证明 native 进程、文件操作或 SQLite 写入真实发生。

## 无法选择多 Agent

这是当前预期行为。多 Agent 协调已有 service/runtime 支持，但创建 UI 仍是预览状态并被禁用。

## 本地截图存在差异

文档截图以固定 CI 浏览器环境为准。仅在有意审核 UI 变更时运行 `npm run docs:screenshots:update`。
