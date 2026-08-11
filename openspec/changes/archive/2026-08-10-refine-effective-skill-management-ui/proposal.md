## Why

有效 Skill 运行时元数据已经完整进入设置页，但当前列表行同时展开过多标签和细节，削弱了名称、状态与主操作的视觉优先级，也让遮蔽关系、只读原因和 Utility 可用性更难快速理解。现在需要在不改变运行时语义的前提下，建立可扫描、可深入检查且适配窄屏的 Skill 管理体验。

## What Changes

- 将 Skill 列表重构为渐进式信息层级：默认行聚焦名称、启用状态、有效层级、类型和主操作，其余运行时元数据进入详情检查器。
- 在宽屏设置页提供与选中行联动的详情检查器，在窄屏使用具备焦点管理的应用内详情面板，避免行内展开造成列表跳动和横向滚动。
- 在详情检查器中集中展示来源、信任、交付方式、版本、使用统计、兼容性、资源摘要及有效定义信息。
- 将遮蔽定义改为按优先级排列的只读时间线，明确当前生效定义和各层被遮蔽定义的关系。
- 简化 System 只读与 Utility 不可委托状态的提示，使用图标、文字和语义色共同表达，不依赖颜色或大量状态徽标。
- 在 Agent 视图中保持 Assign/Remove 为主操作，将详情和预览作为次级操作，并保留行级待处理与错误反馈。
- 补充键盘导航、焦点恢复、缩放、减少动态效果以及 375px 到桌面宽度的响应式行为测试。
- 本变更同时影响 Tauri 桌面端和 Web/mock 端的 React UI；两端继续通过同一前端服务接口消费等价数据。

## Capabilities

### New Capabilities

无。

### Modified Capabilities

- `settings-skill-management-ui`: 修改 Skill 列表的信息层级、详情检查器、遮蔽定义呈现、Agent 操作层级及响应式与可访问性要求。

## Impact

- 前端：`src/settings/pages/skills-page.tsx`、`src/settings/pages/skills/` 下的列表、详情与对话框组件，以及相关本地化文案。
- 测试：Skill 设置页 Vitest 组件/交互测试与 Playwright 响应式、键盘交互测试。
- 服务边界：不新增或修改 Tauri command、Rust 数据模型、SQLite、Skill 运行时或 adapter 接口；组件仍仅通过现有前端服务边界读取和变更数据。
- 依赖：继续使用 React 内置 state、Tailwind CSS、现有 shadcn-style 组件和 lucide-react，不引入新的状态管理、组件库、字体或动画依赖。
- 范围外：Skill Overlay、自进化候选、Curator 审批与治理策略页面不属于本变更。
