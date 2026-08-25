use super::super::test_doubles::{FakeTempStore, SequentialIds};
use super::*;

fn store() -> Arc<FakeTempStore> {
    FakeTempStore::new(SequentialIds::new())
}

fn cleaned(store: &FakeTempStore) -> Vec<String> {
    store
        .log
        .lock()
        .expect("log lock")
        .cleaned_operations
        .clone()
}

#[test]
fn dropping_the_guard_cleans_the_operation() {
    let temp = store();
    {
        let _guard = OperationMediaGuard::new(temp.clone(), "operation-1");
    }
    assert_eq!(cleaned(&temp), vec!["operation-1".to_string()]);
}

#[test]
fn an_early_return_still_cleans() {
    let temp = store();
    fn failing(temp: Arc<FakeTempStore>) -> Result<(), &'static str> {
        let _guard = OperationMediaGuard::new(temp, "operation-2");
        Err("something went wrong before the happy path")
    }
    assert!(failing(temp.clone()).is_err());
    assert_eq!(cleaned(&temp), vec!["operation-2".to_string()]);
}

#[test]
fn an_unwind_still_cleans() {
    let temp = store();
    let handle = temp.clone();
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
        let _guard = OperationMediaGuard::new(handle, "operation-3");
        panic!("worker thread failed unexpectedly");
    }));
    assert!(result.is_err());
    assert_eq!(cleaned(&temp), vec!["operation-3".to_string()]);
}

#[test]
fn a_disarmed_guard_leaves_the_media_for_its_new_owner() {
    // Playback outlives synthesis: the generated WAV must survive the synthesis scope and be
    // deleted once the sink is done with it.
    let temp = store();
    {
        let mut guard = OperationMediaGuard::new(temp.clone(), "operation-4");
        guard.disarm();
    }
    assert!(cleaned(&temp).is_empty());
}

#[test]
fn cleanup_runs_once_per_guard() {
    let temp = store();
    {
        let _first = OperationMediaGuard::new(temp.clone(), "operation-5");
        let _second = OperationMediaGuard::new(temp.clone(), "operation-6");
    }
    let cleaned = cleaned(&temp);
    assert_eq!(cleaned.len(), 2);
    assert!(cleaned.contains(&"operation-5".to_string()));
    assert!(cleaned.contains(&"operation-6".to_string()));
}
