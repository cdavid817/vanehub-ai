use super::SqlitePlanRepository;
use crate::contexts::task_orchestration::application::{
    GeneratePlanDraftRequest, PlanApplicationError, PlanApplicationService, PlanGenerationPort,
    PlanGenerationRequest, PlanGenerationResponse,
};
use crate::platform::database::NativeDatabase;
use crate::test_support::TempDirectory;
use std::sync::Arc;

struct FixedGenerator(Result<PlanGenerationResponse, String>);

impl PlanGenerationPort for FixedGenerator {
    fn generate(
        &self,
        _request: &PlanGenerationRequest,
    ) -> Result<PlanGenerationResponse, PlanApplicationError> {
        self.0.clone().map_err(PlanApplicationError::Storage)
    }
}

fn request() -> GeneratePlanDraftRequest {
    GeneratePlanDraftRequest {
        plan_id: Some("plan-service-test".to_string()),
        version: 1,
        goal: "Implement a bounded feature".to_string(),
        project_path: "C:\\code\\app".to_string(),
        base_ref: "main".to_string(),
        available_tools: vec!["shell: validation only".to_string()],
    }
}

fn service(
    database: &NativeDatabase,
    response: Result<PlanGenerationResponse, String>,
) -> PlanApplicationService {
    PlanApplicationService::new(
        Arc::new(SqlitePlanRepository::new(database.clone())),
        Arc::new(FixedGenerator(response)),
    )
}

#[test]
fn valid_planner_output_is_strictly_parsed_validated_and_persisted() {
    let directory = TempDirectory::new("plan-service-valid");
    let database = NativeDatabase::new(directory.path().to_path_buf()).expect("database");
    let application = service(
        &database,
        Ok(PlanGenerationResponse {
            active_profile_id: "profile-1".to_string(),
            content: r#"{
                "subtasks":[{
                    "id":"task-1",
                    "title":"Implement",
                    "description":"Implement the feature",
                    "acceptanceCriteria":["Tests pass"]
                }],
                "dependencies":[]
            }"#
            .to_string(),
        }),
    );

    let draft = application.generate_draft(&request()).expect("draft");
    let persisted = application
        .find_draft("plan-service-test")
        .expect("lookup")
        .expect("persisted draft");

    assert_eq!(draft, persisted);
    assert_eq!(draft.planner_profile_id.as_deref(), Some("profile-1"));
}

#[test]
fn invalid_output_records_only_a_safe_action_and_never_persists_a_draft() {
    let directory = TempDirectory::new("plan-service-invalid");
    let database = NativeDatabase::new(directory.path().to_path_buf()).expect("database");
    let application = service(
        &database,
        Ok(PlanGenerationResponse {
            active_profile_id: "profile-secret".to_string(),
            content: "raw-model-output-secret".to_string(),
        }),
    );

    assert!(matches!(
        application.generate_draft(&request()),
        Err(PlanApplicationError::Validation(_))
    ));
    assert!(application
        .find_draft("plan-service-test")
        .expect("lookup")
        .is_none());

    let connection = database.connection().expect("connection");
    let recorded: (String, String) = connection
        .query_row(
            "SELECT failure_class, safe_action FROM plan_generation_failures",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("failure record");
    assert_eq!(recorded.0, "planner_output_invalid");
    assert!(recorded.1.contains("Retry"));
    assert!(!recorded.1.contains("raw-model-output-secret"));
    assert!(!recorded.1.contains("profile-secret"));
}
