use super::{SkillId, SkillMetadata};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct BuiltinSkillDefinition {
    pub(crate) id: &'static str,
    pub(crate) name: &'static str,
    pub(crate) description: &'static str,
    pub(crate) category: &'static str,
    pub(crate) triggers: &'static [&'static str],
    pub(crate) body: &'static str,
}

impl BuiltinSkillDefinition {
    pub(crate) fn metadata(self) -> Result<SkillMetadata, super::SkillDomainError> {
        SkillMetadata::new(
            self.id,
            self.name,
            self.description,
            self.category,
            "1.0.0",
            self.triggers
                .iter()
                .map(|trigger| (*trigger).to_string())
                .collect(),
        )
    }
}

const BUILTIN_SKILLS: [BuiltinSkillDefinition; 6] = [
    BuiltinSkillDefinition {
        id: "tdd-discipline",
        name: "TDD 开发纪律助手",
        description: "引导开发过程遵循测试先行、红绿重构和回归验证纪律。",
        category: "development",
        triggers: &["TDD", "测试先行", "红绿重构"],
        body: "Use this skill to keep implementation work aligned with test-first development discipline. Start by identifying the behavior under change, add or update focused tests, implement the minimal code required, then run the relevant verification before broadening scope.",
    },
    BuiltinSkillDefinition {
        id: "code-review",
        name: "代码审查助手",
        description: "从缺陷、回归风险、可维护性和测试缺口角度审查代码变更。",
        category: "review",
        triggers: &["代码审查", "review", "检查变更"],
        body: "Use this skill for code review. Prioritize correctness, regressions, missing tests, data loss, security issues, and maintainability. Lead with concrete findings tied to files and behavior.",
    },
    BuiltinSkillDefinition {
        id: "code-security-scan",
        name: "代码安全扫描",
        description: "检查常见安全风险、敏感信息泄漏、命令注入和不安全文件操作。",
        category: "security",
        triggers: &["安全扫描", "security", "漏洞"],
        body: "Use this skill to review code for security risks. Check trust boundaries, shell/file operations, secrets handling, dependency usage, input validation, and authorization-sensitive paths.",
    },
    BuiltinSkillDefinition {
        id: "api-doc-generation",
        name: "API 文档自动生成",
        description: "根据接口、类型和示例生成结构化 API 文档。",
        category: "documentation",
        triggers: &["API 文档", "接口文档", "api docs"],
        body: "Use this skill to generate API documentation from source interfaces and examples. Include purpose, parameters, response shapes, errors, and practical usage examples.",
    },
    BuiltinSkillDefinition {
        id: "unit-test-generation",
        name: "单元测试自动生成",
        description: "为核心函数、边界条件和回归场景生成单元测试。",
        category: "testing",
        triggers: &["单元测试", "unit test", "测试生成"],
        body: "Use this skill to add focused unit tests. Cover expected behavior, edge cases, invalid input, and regressions. Keep tests close to existing project patterns.",
    },
    BuiltinSkillDefinition {
        id: "readme-generation",
        name: "README 文档生成",
        description: "生成或改进项目 README，包括安装、使用、开发和验证说明。",
        category: "documentation",
        triggers: &["README", "项目说明", "使用文档"],
        body: "Use this skill to create or improve README content. Cover what the project does, setup, common commands, configuration, development workflow, and troubleshooting.",
    },
];

pub(crate) fn builtin_definitions() -> &'static [BuiltinSkillDefinition] {
    &BUILTIN_SKILLS
}

pub(crate) fn builtin_definition(id: &SkillId) -> Option<BuiltinSkillDefinition> {
    BUILTIN_SKILLS
        .iter()
        .copied()
        .find(|definition| definition.id == id.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn catalog_contains_exactly_the_six_stable_builtin_skills() {
        let ids = builtin_definitions()
            .iter()
            .map(|definition| definition.id)
            .collect::<Vec<_>>();
        assert_eq!(
            ids,
            vec![
                "tdd-discipline",
                "code-review",
                "code-security-scan",
                "api-doc-generation",
                "unit-test-generation",
                "readme-generation"
            ]
        );
        assert_eq!(ids.iter().copied().collect::<BTreeSet<_>>().len(), 6);
        for definition in builtin_definitions() {
            let metadata = definition.metadata().expect("valid builtin metadata");
            assert_eq!(metadata.id.as_str(), definition.id);
            assert_eq!(metadata.version, "1.0.0");
            assert!(!definition.body.is_empty());
        }
    }
}
