//! Shipped expert roles, mirroring `src/config/builtin-expert-roles.ts` so the desktop and Web
//! runtimes offer the same starting point. They are never persisted: editing one is rejected, and
//! the UI copies it into a user role instead.

use crate::contexts::agent_runtime::domain::{ExpertRole, ExpertRoleOrigin, ExpertRoleReviewPolicy};

const EPOCH: &str = "1970-01-01T00:00:00.000Z";

pub(crate) fn builtin_expert_roles() -> Vec<ExpertRole> {
    vec![
        role(
            "builtin-architect",
            "架构师",
            "🏛",
            "#9B7EBD",
            "负责系统设计、技术选型与方案拆解，不直接写实现代码",
            "你是本次协作会话中的架构师。\n你的职责是理解需求、给出系统设计与技术选型，并把工作拆解成可执行的步骤。\n你不直接编写实现代码；需要落地时，把任务交给实现者。\n给出方案时说明取舍与被否决的替代方案，不要只给结论。",
            false,
            false,
        ),
        role(
            "builtin-implementer",
            "实现者",
            "🔧",
            "#5B8C5A",
            "负责按既定方案编写与修改代码，落地具体实现",
            "你是本次协作会话中的实现者。\n你按已确定的方案编写和修改代码，遇到方案层面的分歧先提出来而不是自行改设计。\n改完后说明你实际动了哪些文件、为什么这样改。\n需要评审时，把工作交给评审者。",
            false,
            false,
        ),
        role(
            "builtin-reviewer",
            "代码审查",
            "🔍",
            "#C77D3A",
            "负责审查改动的正确性、安全性与测试覆盖，直言不讳地指出问题",
            "你是本次协作会话中的代码审查者。\n你审查改动的正确性、安全性、边界条件与测试覆盖。\n直接指出问题，不要为了缓和语气而弱化结论；没有问题时也明确说明你检查了什么。\n区分「必须修」和「可以改进」，不要把两者混在一起。",
            // Same-family models make correlated errors and tend to agree with each other, so a
            // reviewer is worth more when it comes from a different family than the code's author.
            true,
            true,
        ),
    ]
}

#[allow(clippy::too_many_arguments)]
fn role(
    id: &str,
    display_name: &str,
    avatar: &str,
    color: &str,
    responsibility: &str,
    instruction: &str,
    peer_reviewer: bool,
    require_different_family: bool,
) -> ExpertRole {
    ExpertRole {
        id: id.to_string(),
        display_name: display_name.to_string(),
        avatar: avatar.to_string(),
        color: color.to_string(),
        responsibility: responsibility.to_string(),
        instruction: instruction.to_string(),
        skill_ids: Vec::new(),
        review_policy: ExpertRoleReviewPolicy {
            peer_reviewer,
            require_different_family,
        },
        preferred_providers: Vec::new(),
        origin: ExpertRoleOrigin::Builtin,
        created_at: EPOCH.to_string(),
        updated_at: EPOCH.to_string(),
    }
}
