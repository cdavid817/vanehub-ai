# VaneHub AI Icon

这套图标保留原 Icon 的 `V` 核心识别，并将其重构为“多 Agent 汇聚到智能工作区”的视觉系统。

## 设计语言

- `V`：VaneHub 品牌首字母，也是两个 Agent 流汇聚至中心 Hub 的路径。
- 三个端点：上方两个圆角端点代表并行 Agent，下方高亮节点为统一 Workspace。
- 环形轨道：表达编排、自动化、远程连接与持续运行。
- 深海军蓝、冷白与单一冷青强调色：与应用的 futuristic/minimal 主题保持一致。
- 使用纯色几何与负空间建立层级，不使用玻璃高光、环境光、阴影、内框或复杂纹理。

## 响应式版本

- `source/app-icon.svg`：48px 以上及桌面、商店、移动端主图标。
- `source/app-icon-compact.svg`：16px、24px、32px 与 favicon，进一步省略端点节点细节。
- `source/android-foreground.svg`：Android Adaptive Icon 安全区前景。
- `source/android-monochrome.svg`：Android 13+ themed icon。

运行 `npm run icons:generate` 生成所有平台资产。跨平台 Node 脚本只写入 `icons/generated`、`icons/optical`、`icons/raster` 和 Web 图标。
