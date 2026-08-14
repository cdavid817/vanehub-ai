use super::OnePieceToolFeatureGates;
use std::collections::BTreeMap;

#[test]
fn rollout_defaults_expose_only_artifact_inspection() {
    let gates = OnePieceToolFeatureGates::rollout_defaults();

    assert!(gates.enabled("artifact", "read"));
    for (capability, mode) in [
        ("browser", "read"),
        ("web", "read"),
        ("code_execution", "execute"),
        ("ocr", "read"),
        ("artifact", "publish"),
        ("artifact", "download"),
        ("delegation", "read"),
        ("delegation", "write"),
        ("delegation", "apply"),
    ] {
        assert!(!gates.enabled(capability, mode), "{capability}/{mode}");
    }
}

#[test]
fn gates_require_exact_opt_in_and_remain_independent() {
    let values = BTreeMap::from([
        ("VANEHUB_ONEPIECE_WEB_ENABLED", "1"),
        ("VANEHUB_ONEPIECE_DELEGATION_EDIT_ENABLED", "true"),
        ("VANEHUB_ONEPIECE_ARTIFACT_READ_ENABLED", "0"),
    ]);
    let gates = OnePieceToolFeatureGates::from_lookup(|name| {
        values.get(name).map(|value| (*value).to_owned())
    });

    assert!(gates.enabled("web", "read"));
    assert!(!gates.enabled("delegation", "write"));
    assert!(!gates.enabled("artifact", "read"));
    assert!(!gates.enabled("browser", "read"));
    assert!(gates.tool_enabled("file"));
    assert!(gates.tool_enabled("shell"));
}
