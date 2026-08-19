import type { Skill, SkillAgentMountPath } from "../types/skill";
import { mockAgents } from "./mock-agent-data";
import { nowIso } from "./web-mock-clock";

export interface WebBuiltinSkillSeed {
  id: string;
  name: string;
  description: string;
  category: string;
  triggers: string[];
}

/** Read-only fixture data, never mutated, so it is safe to share by value across modules. */
export const webBuiltinSkillSeeds: readonly WebBuiltinSkillSeed[] = [
  {
    id: "tdd-discipline",
    name: "TDD 开发纪律助手",
    description: "引导开发过程遵循测试先行、红绿重构和回归验证纪律。",
    category: "development",
    triggers: ["TDD", "测试先行", "红绿重构"],
  },
  {
    id: "code-review",
    name: "代码审查助手",
    description: "从缺陷、回归风险、可维护性和测试缺口角度审查代码变更。",
    category: "review",
    triggers: ["代码审查", "review"],
  },
  {
    id: "code-security-scan",
    name: "代码安全扫描",
    description: "检查常见安全风险、敏感信息泄漏、命令注入和不安全文件操作。",
    category: "security",
    triggers: ["安全扫描", "security"],
  },
  {
    id: "api-doc-generation",
    name: "API 文档自动生成",
    description: "根据接口、类型和示例生成结构化 API 文档。",
    category: "documentation",
    triggers: ["API 文档", "api docs"],
  },
  {
    id: "unit-test-generation",
    name: "单元测试自动生成",
    description: "为核心函数、边界条件和回归场景生成单元测试。",
    category: "testing",
    triggers: ["单元测试", "unit test"],
  },
  {
    id: "readme-generation",
    name: "README 文档生成",
    description: "生成或改进项目 README，包括安装、使用、开发和验证说明。",
    category: "documentation",
    triggers: ["README", "项目说明"],
  },
];

export function createWebSkillMountPathSeeds(): SkillAgentMountPath[] {
  return mockAgents.map((agent) => ({
    agentId: agent.id,
    mountPath:
      agent.id === "claude-code"
        ? ".claude/skills"
        : agent.id === "codex-cli"
          ? ".codex/skills"
          : agent.id === "gemini-cli"
            ? ".gemini/skills"
            : agent.id === "opencode"
              ? ".opencode/skills"
              : ".skills",
    isDefault: true,
  }));
}

export function createWebSkillSeeds(): Skill[] {
  const skills: Skill[] = webBuiltinSkillSeeds.map((seed) => {
    const timestamp = nowIso();
    const isUserOverride = seed.id === "readme-generation";
    const isUtility = seed.id === "code-security-scan";
    return {
      id: seed.id,
      scope: "global",
      workspacePath: null,
      source: "builtin",
      enabled: true,
      skillDir: `~/.vanehub/skills/${seed.id}`,
      skillMdPath: `~/.vanehub/skills/${seed.id}/SKILL.md`,
      contentHash: `web-${seed.id}`,
      metadata: {
        id: seed.id,
        name: seed.name,
        description: seed.description,
        category: seed.category,
        version: "1.0.0",
        triggers: seed.triggers,
        aliases: seed.id === "readme-generation" ? ["docs"] : [],
        type: isUtility ? "utility" : "role",
        delivery: seed.id === "tdd-discipline" ? "on-demand" : "eager",
        compatibilityDefaults: { skillType: false, delivery: false },
      },
      boundAgentIds: ["claude-code", "codex-cli"],
      bindings: [],
      createdAt: timestamp,
      updatedAt: timestamp,
      layer: isUserOverride ? "user" : "system",
      origin: isUserOverride ? "migrated" : "shipped",
      trust: "trusted",
      availability: isUtility ? "unsupported" : "available",
      delegationCapability: isUtility
        ? { supported: false, reason: "native-runtime-unavailable" }
        : { supported: false, reason: "not-utility" },
      immutable: !isUserOverride,
      shadowedDefinitions: isUserOverride
        ? [{ layer: "system", origin: "shipped", version: "1.0.0", availability: "available" }]
        : [],
      usage: {
        viewCount: seed.id === "tdd-discipline" ? 3 : 0,
        useCount: seed.id === "tdd-discipline" ? 1 : 0,
        lastViewedAt: seed.id === "tdd-discipline" ? timestamp : null,
        lastUsedAt: seed.id === "tdd-discipline" ? timestamp : null,
        revisionWitness: "web-usage-1",
      },
    };
  });
  skills.push({
    ...skills[0],
    id: "project-conventions",
    scope: "workspace",
    workspacePath: "D:/example/project",
    source: "user",
    skillDir: "D:/example/project/.vanehub/skills/project-conventions",
    skillMdPath: "D:/example/project/.vanehub/skills/project-conventions/SKILL.md",
    contentHash: "web-project-conventions",
    metadata: {
      id: "project-conventions",
      name: "Project Conventions",
      description: "Project-specific conventions.",
      category: "development",
      version: "1.0.0",
      triggers: ["project"],
      aliases: [],
      type: "role",
      delivery: "on-demand",
      compatibilityDefaults: { skillType: false, delivery: false },
    },
    boundAgentIds: [],
    layer: "project",
    origin: "created",
    immutable: false,
    shadowedDefinitions: [],
    usage: {
      viewCount: 0,
      useCount: 0,
      lastViewedAt: null,
      lastUsedAt: null,
      revisionWitness: "web-project-usage-1",
    },
  });
  return skills;
}

export function createWebSkillDocumentSeeds(skills: Skill[]): Map<string, string> {
  const documents = new Map<string, string>(
    skills.map((skill) => [
      `${skill.scope}:${skill.workspacePath ?? ""}:${skill.id}`,
      `Built-in instructions for ${skill.metadata.name}.`,
    ]),
  );
  documents.set(
    "global::tdd-discipline",
    `Use {skill_base_dir} for supporting material.\n${"TDD guidance. ".repeat(1_100)}`,
  );
  return documents;
}

export function createWebSkillResourceDocumentSeeds(): Map<string, string> {
  return new Map<string, string>([
    ["skill://tdd-discipline/references/testing-cycle.md", "Red, green, refactor, then run regression tests."],
    ["skill://tdd-discipline/templates/test-plan.md", "# Test plan\n\n- Expected failure\n- Minimal fix\n- Regression"],
    ["skill://project-conventions/references/conventions.md", "Use the project formatting and validation commands."],
  ]);
}
