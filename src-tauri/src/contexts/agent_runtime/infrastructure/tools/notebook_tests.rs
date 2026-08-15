use super::*;
use crate::test_support::TempDirectory;

/// A notebook shaped the way a real one is: unsorted keys inside cells, a plot output carrying
/// base64, an error output, and per-line source arrays. The unsorted keys matter -- they are what
/// proves an untouched cell was spliced rather than re-serialized.
const NOTEBOOK: &str = r##"{
 "cells": [
  {
   "cell_type": "markdown",
   "id": "intro",
   "metadata": {},
   "source": ["# Revenue\n", "Notes.\n"]
  },
  {
   "zzz_writer_field": "kept",
   "cell_type": "code",
   "execution_count": 1,
   "id": "load",
   "metadata": {},
   "outputs": [
    {
     "output_type": "display_data",
     "data": {"image/png": "iVBORw0KGgoAAAAA", "text/plain": ["<Figure size 640x480>"]},
     "metadata": {}
    }
   ],
   "source": ["import pandas as pd\n", "df.head()\n"]
  },
  {
   "cell_type": "code",
   "execution_count": 2,
   "id": "bug",
   "metadata": {},
   "outputs": [
    {
     "output_type": "error",
     "ename": "AttributeError",
     "evalue": "'DataFrame' object has no attribute 'totals'",
     "traceback": ["Traceback...\n"]
    }
   ],
   "source": ["df.totals()\n"]
  }
 ],
 "metadata": {"kernelspec": {"name": "python3", "display_name": "Python 3"}},
 "nbformat": 4,
 "nbformat_minor": 5
}
"##;

fn workspace(name: &str, contents: &str) -> TempDirectory {
    let directory = TempDirectory::new(name);
    std::fs::write(directory.path().join("analysis.ipynb"), contents).expect("write fixture");
    directory
}

fn run(
    directory: &TempDirectory,
    operation: &str,
    cell_id: Option<&str>,
    cell_index: Option<usize>,
    source: Option<&str>,
) -> ToolExecutionOutcome {
    execute_notebook(
        NotebookRequest {
            operation,
            path: "analysis.ipynb",
            cell_id,
            cell_index,
            source,
            ..NotebookRequest::default()
        },
        &directory.path().to_string_lossy(),
    )
}

fn read_back(directory: &TempDirectory) -> String {
    std::fs::read_to_string(directory.path().join("analysis.ipynb")).expect("read back")
}

/// The whole reason this tool exists: a read must carry the code and not the container. The base64
/// in the fixture is short, so the assertion is on its presence, not its size -- a real one is
/// 120,000 characters and would be truncated into 2,000 characters of noise by the file tool.
#[test]
fn a_read_returns_source_and_never_output_bytes() {
    let directory = workspace("notebook-read", NOTEBOOK);
    let outcome = run(&directory, "read", None, None, None);

    assert!(!outcome.is_error, "{}", outcome.output);
    assert!(outcome.output.contains("import pandas as pd"));
    assert!(outcome.output.contains("df.head()"));
    // Cells are addressable by both forms, and the read says which is which.
    assert!(
        outcome.output.contains("[1] id=load code"),
        "{}",
        outcome.output
    );

    assert!(
        !outcome.output.contains("iVBORw0KGgoAAAAA"),
        "output bytes must never reach the read: {}",
        outcome.output
    );
    // The image is named and measured instead.
    assert!(outcome.output.contains("image/png"), "{}", outcome.output);
    assert!(outcome.output.contains("bytes"), "{}", outcome.output);
    assert!(outcome.output.contains("<Figure size 640x480>"));
}

/// A traceback is usually why a model is reading the notebook at all.
#[test]
fn a_read_keeps_what_an_error_output_says() {
    let directory = workspace("notebook-error", NOTEBOOK);
    let outcome = run(&directory, "read", None, None, None);

    assert!(
        outcome.output.contains("AttributeError"),
        "{}",
        outcome.output
    );
    assert!(
        outcome.output.contains("has no attribute 'totals'"),
        "{}",
        outcome.output
    );
}

/// The property that makes this usable on a git-tracked notebook. `serde_json` sorts object keys,
/// so a parse-then-write would move `zzz_writer_field` and reorder every cell in the file; splicing
/// raw text leaves untouched cells exactly as they were stored.
#[test]
fn editing_one_cell_leaves_every_other_cell_byte_identical() {
    let directory = workspace("notebook-fidelity", NOTEBOOK);
    let outcome = run(&directory, "replace", Some("bug"), None, Some("df.sum()\n"));
    assert!(!outcome.is_error, "{}", outcome.output);

    let updated = read_back(&directory);
    // The untouched code cell keeps its writer-specific field in its original position, ahead of
    // `cell_type` -- which alphabetical re-serialization would have moved to the end.
    assert!(
        updated.contains("\"zzz_writer_field\": \"kept\",\n   \"cell_type\": \"code\","),
        "untouched cell was re-serialized: {updated}"
    );
    // The untouched cell's output bytes survive too.
    assert!(updated.contains("iVBORw0KGgoAAAAA"));
    // And the notebook's own metadata keeps its order.
    assert!(
        updated.contains("\"name\": \"python3\", \"display_name\": \"Python 3\""),
        "notebook metadata was re-serialized: {updated}"
    );

    // Still a notebook.
    let reparsed: serde_json::Value = serde_json::from_str(&updated).expect("valid notebook");
    assert_eq!(reparsed["cells"].as_array().expect("cells").len(), 3);
    assert_eq!(reparsed["nbformat"], 4);
}

/// Outputs describe one execution of one source. Nothing here can re-run a cell, so keeping them
/// would leave the file permanently claiming a result its code cannot produce.
#[test]
fn changing_code_source_clears_its_outputs_and_execution_count() {
    let directory = workspace("notebook-clear", NOTEBOOK);
    let outcome = run(
        &directory,
        "replace",
        Some("load"),
        None,
        Some("df.tail()\n"),
    );

    assert!(!outcome.is_error, "{}", outcome.output);
    assert!(
        outcome.output.contains("cleared its outputs"),
        "{}",
        outcome.output
    );

    let reparsed: serde_json::Value = serde_json::from_str(&read_back(&directory)).expect("valid");
    let cell = &reparsed["cells"][1];
    assert_eq!(cell["source"], serde_json::json!(["df.tail()\n"]));
    assert_eq!(cell["outputs"], serde_json::json!([]));
    assert_eq!(cell["execution_count"], serde_json::Value::Null);
}

/// A markdown cell has neither outputs nor an execution count, and must not gain them.
#[test]
fn changing_markdown_source_does_not_add_execution_state() {
    let directory = workspace("notebook-markdown", NOTEBOOK);
    let outcome = run(
        &directory,
        "replace",
        Some("intro"),
        None,
        Some("# Costs\n"),
    );

    assert!(!outcome.is_error, "{}", outcome.output);
    assert!(!outcome.output.contains("cleared"), "{}", outcome.output);

    let reparsed: serde_json::Value = serde_json::from_str(&read_back(&directory)).expect("valid");
    let cell = &reparsed["cells"][0];
    assert_eq!(cell["source"], serde_json::json!(["# Costs\n"]));
    assert!(cell.get("outputs").is_none());
    assert!(cell.get("execution_count").is_none());
}

#[test]
fn inserting_and_deleting_produce_a_notebook_that_still_parses() {
    let directory = workspace("notebook-insert", NOTEBOOK);

    let inserted = execute_notebook(
        NotebookRequest {
            operation: "insert",
            path: "analysis.ipynb",
            cell_id: Some("intro"),
            source: Some("import numpy as np\n"),
            cell_type: Some("code"),
            position: Some("after"),
            ..NotebookRequest::default()
        },
        &directory.path().to_string_lossy(),
    );
    assert!(!inserted.is_error, "{}", inserted.output);

    let reparsed: serde_json::Value = serde_json::from_str(&read_back(&directory)).expect("valid");
    let cells = reparsed["cells"].as_array().expect("cells");
    assert_eq!(cells.len(), 4);
    assert_eq!(
        cells[1]["source"],
        serde_json::json!(["import numpy as np\n"])
    );
    assert_eq!(cells[1]["cell_type"], "code");
    // The notebook uses ids, so the new cell gets one rather than being the only cell without.
    assert!(cells[1]["id"].as_str().is_some_and(|id| !id.is_empty()));

    let deleted = run(&directory, "delete", None, Some(0), None);
    assert!(!deleted.is_error, "{}", deleted.output);
    let reparsed: serde_json::Value = serde_json::from_str(&read_back(&directory)).expect("valid");
    let cells = reparsed["cells"].as_array().expect("cells");
    assert_eq!(cells.len(), 3);
    assert_eq!(
        cells[0]["source"],
        serde_json::json!(["import numpy as np\n"])
    );
}

#[test]
fn the_two_addressing_forms_select_the_same_cell_and_must_not_be_mixed() {
    let by_id = workspace("notebook-by-id", NOTEBOOK);
    let by_index = workspace("notebook-by-index", NOTEBOOK);
    assert!(!run(&by_id, "replace", Some("bug"), None, Some("x\n")).is_error);
    assert!(!run(&by_index, "replace", None, Some(2), Some("x\n")).is_error);
    assert_eq!(read_back(&by_id), read_back(&by_index));

    let directory = workspace("notebook-address", NOTEBOOK);
    let both = run(&directory, "replace", Some("bug"), Some(2), Some("x\n"));
    assert!(both.is_error);
    assert!(both.output.contains("not both"), "{}", both.output);

    let neither = run(&directory, "replace", None, None, Some("x\n"));
    assert!(neither.is_error);
    assert!(
        neither.output.contains("Supply cell_id or cell_index"),
        "{}",
        neither.output
    );

    let missing = run(&directory, "replace", Some("nope"), None, Some("x\n"));
    assert!(missing.is_error);
    assert!(
        missing.output.contains("No cell has id"),
        "{}",
        missing.output
    );

    let out_of_range = run(&directory, "replace", None, Some(99), Some("x\n"));
    assert!(out_of_range.is_error);
    assert!(
        out_of_range.output.contains("out of range"),
        "{}",
        out_of_range.output
    );

    // None of the refusals touched the file.
    assert_eq!(read_back(&directory), NOTEBOOK);
}

/// Partially understanding a notebook and writing a guess back destroys work with no other copy,
/// so anything unrecognized is refused before a write can happen.
#[test]
fn a_file_that_is_not_a_readable_notebook_is_refused_without_writing() {
    for (name, contents, expected) in [
        ("notebook-bad-json", "{not json", "not valid JSON"),
        (
            "notebook-no-cells",
            r#"{"nbformat": 4, "metadata": {}}"#,
            "no cells array",
        ),
        (
            "notebook-v3",
            r#"{"cells": [], "nbformat": 3, "metadata": {}}"#,
            "nbformat 3",
        ),
        (
            "notebook-no-format",
            r#"{"cells": [], "metadata": {}}"#,
            "no nbformat field",
        ),
    ] {
        let directory = workspace(name, contents);
        for operation in ["read", "replace", "delete"] {
            let outcome = run(&directory, operation, None, Some(0), Some("x\n"));
            assert!(outcome.is_error, "{name}/{operation}: {}", outcome.output);
            assert!(
                outcome.output.contains(expected),
                "{name}/{operation}: {}",
                outcome.output
            );
        }
        assert_eq!(read_back(&directory), contents, "{name} was modified");
    }
}

#[test]
fn a_path_that_escapes_the_workspace_is_rejected() {
    let container = TempDirectory::new("notebook-escape");
    let workspace_root = container.path().join("workspace");
    std::fs::create_dir_all(&workspace_root).expect("create workspace");
    let outside = container.write("outside.ipynb", NOTEBOOK);

    let outcome = execute_notebook(
        NotebookRequest {
            operation: "replace",
            path: "../outside.ipynb",
            cell_index: Some(0),
            source: Some("x\n"),
            ..NotebookRequest::default()
        },
        &workspace_root.to_string_lossy(),
    );

    assert!(outcome.is_error);
    assert!(
        outcome.output.contains("path escape is not allowed"),
        "{}",
        outcome.output
    );
    assert_eq!(
        std::fs::read_to_string(&outside).expect("read back"),
        NOTEBOOK
    );
}

#[test]
fn an_unknown_operation_and_a_missing_file_are_reported() {
    let directory = workspace("notebook-unknown", NOTEBOOK);
    let unknown = run(&directory, "execute", None, Some(0), None);
    assert!(unknown.is_error);
    assert!(
        unknown.output.contains("Unknown notebook operation"),
        "{}",
        unknown.output
    );

    let missing = execute_notebook(
        NotebookRequest {
            operation: "read",
            path: "absent.ipynb",
            ..NotebookRequest::default()
        },
        &directory.path().to_string_lossy(),
    );
    assert!(missing.is_error);
    assert!(
        missing.output.contains("does not exist"),
        "{}",
        missing.output
    );
}

/// A notebook whose cells have no ids (nbformat 4.0-4.4) is still fully editable by index, and a
/// cell inserted into it must not be the only one carrying an id.
#[test]
fn a_notebook_without_cell_ids_is_editable_by_index() {
    const NO_IDS: &str = r##"{
 "cells": [
  {"cell_type": "code", "execution_count": null, "metadata": {}, "outputs": [], "source": ["a = 1\n"]}
 ],
 "metadata": {},
 "nbformat": 4,
 "nbformat_minor": 2
}
"##;
    let directory = workspace("notebook-no-ids", NO_IDS);

    let read = run(&directory, "read", None, None, None);
    assert!(read.output.contains("[0] id=- code"), "{}", read.output);

    let inserted = execute_notebook(
        NotebookRequest {
            operation: "insert",
            path: "analysis.ipynb",
            cell_index: Some(0),
            source: Some("b = 2\n"),
            cell_type: Some("markdown"),
            position: Some("after"),
            ..NotebookRequest::default()
        },
        &directory.path().to_string_lossy(),
    );
    assert!(!inserted.is_error, "{}", inserted.output);

    let reparsed: serde_json::Value = serde_json::from_str(&read_back(&directory)).expect("valid");
    let cells = reparsed["cells"].as_array().expect("cells");
    assert_eq!(cells.len(), 2);
    assert!(
        cells[1].get("id").is_none(),
        "a notebook without ids must not gain one: {}",
        cells[1]
    );
    // A markdown cell gets no execution state.
    assert!(cells[1].get("outputs").is_none());
}

#[test]
fn source_is_stored_as_nbformat_stores_it() {
    // One string per line, each keeping its newline except a final unterminated line.
    assert_eq!(source_lines(""), serde_json::json!([]));
    assert_eq!(source_lines("one\n"), serde_json::json!(["one\n"]));
    assert_eq!(
        source_lines("one\ntwo\n"),
        serde_json::json!(["one\n", "two\n"])
    );
    assert_eq!(
        source_lines("one\ntwo"),
        serde_json::json!(["one\n", "two"])
    );
}
