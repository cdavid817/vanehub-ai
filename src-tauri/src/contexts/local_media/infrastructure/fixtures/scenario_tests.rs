use super::*;
use std::io::Write;

fn scenario_file(body: &str) -> PathBuf {
    let directory = std::env::temp_dir().join(format!(
        "vanehub-scenario-{}",
        uuid::Uuid::new_v4().simple()
    ));
    std::fs::create_dir_all(&directory).expect("create the scenario directory");
    let path = directory.join("scenario.json");
    let mut handle = std::fs::File::create(&path).expect("create the scenario file");
    handle.write_all(body.as_bytes()).expect("write scenario");
    path
}

#[test]
fn an_absent_scenario_path_means_default_success() {
    let scenario = FixtureScenario::load(None);

    assert_eq!(scenario.capture, "success");
    assert_eq!(scenario.playback, "success");
    assert_eq!(scenario.devices, "success");
}

#[test]
fn a_scripted_scenario_is_read_per_section() {
    let path = scenario_file(
        r#"{"capture":{"behaviour":"permission_denied"},"devices":{"behaviour":"no_devices"}}"#,
    );

    let scenario = FixtureScenario::load(Some(&path));

    assert_eq!(scenario.capture, "permission_denied");
    assert_eq!(scenario.devices, "no_devices");
    // An omitted section is success, which is different from a section holding a typo.
    assert_eq!(scenario.playback, "success");
}

#[test]
#[should_panic(expected = "fixture configuration error")]
fn a_configured_but_missing_scenario_file_is_a_configuration_error() {
    // Silently defaulting to success here would let a failure scenario pass as a success test.
    FixtureScenario::load(Some(Path::new("/vanehub/definitely/not/here.json")));
}

#[test]
#[should_panic(expected = "not valid JSON")]
fn malformed_json_is_a_configuration_error() {
    let path = scenario_file("{not json");
    FixtureScenario::load(Some(&path));
}

#[test]
#[should_panic(expected = "is not an object")]
fn a_wrongly_typed_section_is_a_configuration_error() {
    let path = scenario_file(r#"{"capture":"permission_denied"}"#);
    FixtureScenario::load(Some(&path));
}

#[test]
#[should_panic(expected = "expected one of")]
fn an_unknown_behaviour_is_a_configuration_error() {
    let path = scenario_file(r#"{"capture":{"behaviour":"permision_denied"}}"#);
    FixtureScenario::load(Some(&path));
}

#[test]
#[should_panic(expected = "is not a string")]
fn a_non_string_behaviour_is_a_configuration_error() {
    let path = scenario_file(r#"{"playback":{"behaviour":7}}"#);
    FixtureScenario::load(Some(&path));
}

#[test]
fn every_documented_behaviour_is_accepted() {
    for behaviour in CAPTURE_BEHAVIOURS {
        let path = scenario_file(&format!(r#"{{"capture":{{"behaviour":"{behaviour}"}}}}"#));
        assert_eq!(FixtureScenario::load(Some(&path)).capture, behaviour);
    }
    for behaviour in PLAYBACK_BEHAVIOURS {
        let path = scenario_file(&format!(r#"{{"playback":{{"behaviour":"{behaviour}"}}}}"#));
        assert_eq!(FixtureScenario::load(Some(&path)).playback, behaviour);
    }
    for behaviour in DEVICE_BEHAVIOURS {
        let path = scenario_file(&format!(r#"{{"devices":{{"behaviour":"{behaviour}"}}}}"#));
        assert_eq!(FixtureScenario::load(Some(&path)).devices, behaviour);
    }
}

#[test]
fn the_limit_reached_capture_behaviour_is_reachable() {
    // It was accepted by the capture fixture but missing from the allowlist, so scripting it was a
    // configuration error and the branch could never run.
    let path = scenario_file(r#"{"capture":{"behaviour":"limit_reached"}}"#);

    assert_eq!(FixtureScenario::load(Some(&path)).capture, "limit_reached");
}
