export const providers = [
  {
    id: "zhipu",
    name: "Zhipu AI",
    vendor: "Zhipu AI",
    status: "Running",
    models: ["GLM-4.5", "GLM-4"],
    key: "sk-***************3f7a",
    endpoint: "https://open.bigmodel.cn/api/paas/v4",
    latency: "128ms",
    successRate: "99.2%",
  },
  {
    id: "anthropic",
    name: "Anthropic",
    vendor: "Anthropic",
    status: "Idle",
    models: ["Claude-3.5", "Claude-3"],
    key: "sk-ant-**********7c2d",
    endpoint: "https://api.anthropic.com/v1",
    latency: "342ms",
    successRate: "97.8%",
  },
  {
    id: "openai",
    name: "OpenAI",
    vendor: "OpenAI",
    status: "Disabled",
    models: ["GPT-4o", "GPT-4"],
    key: "API key not configured",
    endpoint: "https://api.openai.com/v1",
    latency: "-",
    successRate: "-",
  },
];

export const sdkPackages = [
  { name: "@anthropic/sdk", description: "Official Anthropic SDK", current: "v0.28.0", latest: "v0.28.0", status: "Current", size: "32MB", source: "npm" },
  { name: "openai", description: "Official OpenAI SDK", current: "v4.52.0", latest: "v4.67.0", status: "Update available", size: "45MB", source: "npm" },
  { name: "@zhipu/sdk", description: "Official Zhipu AI SDK", current: "v2.1.4", latest: "v2.1.4", status: "Current", size: "18MB", source: "npm" },
  { name: "@mcp/sdk", description: "MCP protocol SDK", current: "v1.0.3", latest: "v1.2.0", status: "Update available", size: "12MB", source: "npm" },
  { name: "@tauri-apps/api", description: "Tauri desktop API", current: "v2.0.2", latest: "v2.0.2", status: "Current", size: "21MB", source: "npm" },
];

export const mcpServers = [
  { id: "codehub", name: "CodeHub MCP", status: "Running", tools: 12, latency: "42ms", scope: "Code repository" },
  { id: "filesystem", name: "Filesystem MCP", status: "Running", tools: 8, latency: "18ms", scope: "Local files" },
  { id: "browser", name: "Browser MCP", status: "Idle", tools: 6, latency: "86ms", scope: "Browser automation" },
];

export const skills = [
  { id: "code-review", name: "Code Review", category: "Security", trigger: "/review", agent: "Code Reviewer", runs: 32, success: "96.8%" },
  { id: "security-scan", name: "Security Scan", category: "Scan", trigger: "/scan", agent: "Code Reviewer", runs: 18, success: "100%" },
  { id: "quality-score", name: "Quality Score", category: "Analysis", trigger: "/score", agent: "Code Reviewer", runs: 15, success: "93.3%" },
  { id: "unit-test", name: "Unit Test Generation", category: "Testing", trigger: "/test", agent: "Test Engineer", runs: 28, success: "89.2%" },
  { id: "api-doc", name: "API Docs", category: "Docs", trigger: "/apidoc", agent: "Docs Generator", runs: 22, success: "95.4%" },
  { id: "readme", name: "README Generation", category: "Project", trigger: "/readme", agent: "Docs Generator", runs: 14, success: "92.8%" },
  { id: "chart", name: "Data Visualization", category: "Chart", trigger: "/chart", agent: "Data Analyst", runs: 5, success: "80%" },
  { id: "predict", name: "Trend Forecast", category: "Forecast", trigger: "/predict", agent: "Data Analyst", runs: 3, success: "66.7%" },
];
