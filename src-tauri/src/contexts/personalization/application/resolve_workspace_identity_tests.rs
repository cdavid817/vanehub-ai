use super::models::WorkspaceIdentityRequest;
use super::ports::WorkspaceIdentityPort;
use super::resolve_workspace_identity::WorkspaceIdentityResolver;
use crate::contexts::personalization::domain::WorkspaceKind;

fn resolver() -> WorkspaceIdentityResolver {
    WorkspaceIdentityResolver::with_case_folding(false)
}

fn resolve(request: WorkspaceIdentityRequest) -> Option<(String, String, WorkspaceKind)> {
    resolver()
        .resolve(&request)
        .expect("resolve")
        .map(|identity| {
            (
                identity.key().as_str().to_string(),
                identity.display_path().to_string(),
                identity.kind(),
            )
        })
}

#[test]
fn no_workspace_information_resolves_to_no_workspace() {
    assert!(resolve(WorkspaceIdentityRequest::default()).is_none());
    assert!(resolve(WorkspaceIdentityRequest {
        project_path: Some("   ".to_string()),
        ..Default::default()
    })
    .is_none());
}

#[test]
fn an_existing_stable_id_wins_over_every_derivable_input() {
    let (key, _, kind) = resolve(WorkspaceIdentityRequest {
        stable_id: Some("proj_01K2ABCDEF".to_string()),
        project_path: Some(r"D:\work\app".to_string()),
        worktree_path: Some(r"D:\work\app-feature".to_string()),
        remote_uri: Some("ssh://deploy@alpha.example/srv/app".to_string()),
    })
    .expect("identity");
    assert_eq!(key, "proj_01K2ABCDEF");
    assert_eq!(kind, WorkspaceKind::Local);
}

#[test]
fn a_worktree_is_its_own_workspace_rather_than_its_parent_project() {
    // Two worktrees of one repository are different working directories with different state.
    // Merging their memories would surface one branch's notes while working on another.
    let worktree = resolve(WorkspaceIdentityRequest {
        project_path: Some(r"D:\work\app".to_string()),
        worktree_path: Some(r"D:\work\app-feature".to_string()),
        ..Default::default()
    })
    .expect("identity");
    let project = resolve(WorkspaceIdentityRequest {
        project_path: Some(r"D:\work\app".to_string()),
        ..Default::default()
    })
    .expect("identity");

    assert_ne!(worktree.0, project.0);
    assert_eq!(worktree.1, r"D:\work\app-feature");
}

#[test]
fn a_remote_uri_resolves_to_its_connection_identity() {
    let (key, display, kind) = resolve(WorkspaceIdentityRequest {
        remote_uri: Some("ssh://deploy@alpha.example:2222/srv/app".to_string()),
        ..Default::default()
    })
    .expect("identity");
    assert!(key.starts_with("ws_"));
    assert_eq!(display, "deploy@alpha.example:/srv/app");
    assert_eq!(kind, WorkspaceKind::Remote);
}

#[test]
fn the_same_remote_path_on_two_hosts_resolves_to_two_workspaces() {
    let left = resolve(WorkspaceIdentityRequest {
        remote_uri: Some("ssh://deploy@alpha.example/srv/app".to_string()),
        ..Default::default()
    })
    .expect("identity");
    let right = resolve(WorkspaceIdentityRequest {
        remote_uri: Some("ssh://deploy@beta.example/srv/app".to_string()),
        ..Default::default()
    })
    .expect("identity");
    assert_ne!(left.0, right.0);
}

#[test]
fn a_password_component_in_a_uri_never_reaches_the_identity() {
    // An identity derived from a secret changes when the secret rotates, and would put recoverable
    // material into a value that appears in diagnostics.
    let with_password = resolve(WorkspaceIdentityRequest {
        remote_uri: Some("ssh://deploy:hunter2@alpha.example/srv/app".to_string()),
        ..Default::default()
    })
    .expect("identity");
    let without = resolve(WorkspaceIdentityRequest {
        remote_uri: Some("ssh://deploy@alpha.example/srv/app".to_string()),
        ..Default::default()
    })
    .expect("identity");

    assert_eq!(
        with_password.0, without.0,
        "the password must not participate in the key"
    );
    assert!(!with_password.0.contains("hunter"));
    assert!(!with_password.1.contains("hunter2"));
}

#[test]
fn a_remote_uri_without_a_port_uses_the_default() {
    let implicit = resolve(WorkspaceIdentityRequest {
        remote_uri: Some("ssh://deploy@alpha.example/srv/app".to_string()),
        ..Default::default()
    })
    .expect("identity");
    let explicit = resolve(WorkspaceIdentityRequest {
        remote_uri: Some("ssh://deploy@alpha.example:22/srv/app".to_string()),
        ..Default::default()
    })
    .expect("identity");
    assert_eq!(implicit.0, explicit.0);
}

#[test]
fn a_remote_uri_beats_a_local_path_when_both_are_present() {
    let (_, display, kind) = resolve(WorkspaceIdentityRequest {
        project_path: Some(r"D:\work\app".to_string()),
        remote_uri: Some("ssh://alpha.example/srv/app".to_string()),
        ..Default::default()
    })
    .expect("identity");
    assert_eq!(kind, WorkspaceKind::Remote);
    assert_eq!(display, "alpha.example:/srv/app");
}

#[test]
fn case_folding_follows_the_injected_platform_rule() {
    let folding = WorkspaceIdentityResolver::with_case_folding(true);
    let sensitive = WorkspaceIdentityResolver::with_case_folding(false);
    let upper = WorkspaceIdentityRequest {
        project_path: Some(r"D:\Work\App".to_string()),
        ..Default::default()
    };
    let lower = WorkspaceIdentityRequest {
        project_path: Some(r"d:\work\app".to_string()),
        ..Default::default()
    };

    let key = |resolver: &WorkspaceIdentityResolver, request: &WorkspaceIdentityRequest| {
        resolver
            .resolve(request)
            .expect("resolve")
            .expect("identity")
            .key()
            .as_str()
            .to_string()
    };
    assert_eq!(key(&folding, &upper), key(&folding, &lower));
    assert_ne!(key(&sensitive, &upper), key(&sensitive, &lower));
}

#[test]
fn an_unusable_remote_uri_resolves_to_no_workspace() {
    assert!(resolve(WorkspaceIdentityRequest {
        remote_uri: Some("ssh://".to_string()),
        ..Default::default()
    })
    .is_none());
}
