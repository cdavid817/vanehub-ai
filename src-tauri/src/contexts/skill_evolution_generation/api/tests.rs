use crate::{
    contexts::skill_evolution_generation::{
        api::{GenerationApiError, GenerationPolicyUpdate, SkillEvolutionGenerationApi},
        domain::GeneratedArtifactKind,
    },
    platform::database::NativeDatabase,
    test_support::TempDirectory,
};

fn api(label: &str) -> (TempDirectory, SkillEvolutionGenerationApi) {
    let directory = TempDirectory::new(label);
    let database = NativeDatabase::new(directory.path().to_path_buf()).expect("database");
    let api = SkillEvolutionGenerationApi::new(database);
    (directory, api)
}

#[test]
fn policy_is_default_off_and_rejects_unready_provider() {
    let (_directory, api) = api("generation-api-policy");
    let policy = api.policy("workspace").expect("default policy");
    assert_eq!(policy["enabled"], false);
    assert_eq!(policy["revision"], 0);
    let result = api.update_policy(
        &GenerationPolicyUpdate {
            workspace_id: "workspace",
            expected_revision: 0,
            enabled: true,
            disclosure_version: "generation-disclosure-v1",
            provider_profile_id: Some("missing-profile"),
            model_id: Some("missing-model"),
            allowed_artifact_kinds: &[GeneratedArtifactKind::OverlayLearnBlock],
        },
        1,
    );
    assert_eq!(result, Err(GenerationApiError::ProviderUnavailable));
}

#[test]
fn disabled_policy_update_and_bounded_empty_queries_are_stable() {
    let (_directory, api) = api("generation-api-query");
    let policy = api
        .update_policy(
            &GenerationPolicyUpdate {
                workspace_id: "workspace",
                expected_revision: 0,
                enabled: false,
                disclosure_version: "generation-disclosure-v1",
                provider_profile_id: None,
                model_id: None,
                allowed_artifact_kinds: &[
                    GeneratedArtifactKind::OverlayLearnBlock,
                    GeneratedArtifactKind::NewSkill,
                ],
            },
            1,
        )
        .expect("updated policy");
    assert_eq!(policy["revision"], 1);
    assert_eq!(policy["enabled"], false);
    let jobs = api
        .jobs(Some("workspace"), None, None, 20, None)
        .expect("jobs");
    assert_eq!(jobs["items"].as_array().map(Vec::len), Some(0));
    assert_eq!(
        api.jobs(None, None, None, 0, None),
        Err(GenerationApiError::InvalidRequest)
    );
    assert_eq!(api.job_detail("missing").expect("detail"), None);
}
