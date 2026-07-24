# Runtime 与功能状态标签

| 标签 | 含义 |
| --- | --- |
| 已交付 | 已实现用户可见路径，并有验证证据 |
| 预览 | 支撑 contract 已存在，但正常工作流尚未完成 |
| 仅 Web/mock | 确定性浏览器模拟，不产生 native side effect |
| 仅桌面端 | 使用本地文件系统、CLI、SQLite 或 OS 集成的 Tauri runtime |
| 规划中 | 尚无受支持工作流 |

当一个页面在 Web 预览中看起来可以操作时，应先检查 runtime 标签，再判断它是否真的修改了本机。模拟操作适合 UI 验证，但不是 native 执行证据。
