//! The things no double can prove.
//!
//! Everything the client decides — what is sent, what is refused before it leaves, what is done
//! with an answer — is settled against a scripted channel in
//! `contexts::workspaces::infrastructure::remote_helper`, because those decisions are observable
//! there and would be observable against a real host only when that host happened to be configured
//! to trigger them.
//!
//! Two things are left, and they need real machinery rather than a fake:
//!
//! 1. Whether the helper *program* runs at all — whether `python3 -I -S` accepts the bootstrap,
//!    whether the program parses under a real interpreter, and whether its confinement behaves as
//!    written against a real filesystem. That needs an interpreter, not a network, so it runs
//!    wherever one is installed. A fake that pretended to be Python would prove the fake.
//!
//! 2. Whether the same exchange survives an SSH channel. That needs a host, and it is opt-in.
//!
//! Both skip loudly. "0 failures" over a suite that ran nothing is the most reassuring possible way
//! to report that nothing was checked, so each skip prints what it skipped and why.
//!
//! ```text
//! VANEHUB_SSH_INTEGRATION_HOST=build-host
//! VANEHUB_SSH_INTEGRATION_USER=ci
//! VANEHUB_SSH_INTEGRATION_ROOT=/srv/workspace
//! VANEHUB_SSH_INTEGRATION_PORT=22            # optional, defaults to 22
//! ```

use std::env;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// The helper program, read from the same file the client embeds.
///
/// Read rather than duplicated: a copy would be a second program that passes its own tests while
/// the shipped one does not.
const HELPER_PROGRAM: &str =
    include_str!("../src/contexts/workspaces/infrastructure/remote_helper/helper.py");

/// The command the client sends, character for character.
///
/// Duplicated deliberately and asserted below against the client's constant: this file cannot
/// import from the crate's private modules, and a bootstrap that drifted from the one in production
/// would make this suite prove a command nobody sends.
const BOOTSTRAP_ARGUMENT: &str =
    "exec(__import__('base64').b64decode(__import__('sys').stdin.readline()))";

fn skip(reason: &str) {
    println!("remote helper integration skipped: {reason}");
}

// ---------------------------------------------------------------------------------------------
// The program, under a real interpreter
// ---------------------------------------------------------------------------------------------

/// Whichever interpreter this machine has, or nothing.
fn python() -> Option<&'static str> {
    for candidate in ["python3", "python"] {
        let probed = Command::new(candidate)
            .arg("--version")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
        if matches!(probed, Ok(status) if status.success()) {
            return Some(candidate);
        }
    }
    None
}

/// Runs the helper exactly as the client would: bootstrap command, program on the first line of
/// stdin, request after it.
fn run_helper(interpreter: &str, root: &Path, operation: &str) -> serde_json::Value {
    use base64::Engine;
    let program = base64::engine::general_purpose::STANDARD.encode(HELPER_PROGRAM);
    let request = format!(
        r#"{{"version":1,"root":{},"operation":{operation}}}"#,
        serde_json::to_string(&root.to_string_lossy().to_string()).expect("root")
    );

    let mut child = Command::new(interpreter)
        .args(["-I", "-S", "-c", BOOTSTRAP_ARGUMENT])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn helper");
    {
        let stdin = child.stdin.as_mut().expect("stdin");
        writeln!(stdin, "{program}").expect("write program");
        write!(stdin, "{request}").expect("write request");
    }
    let output = child.wait_with_output().expect("helper output");
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    serde_json::from_str(&stdout).unwrap_or_else(|error| {
        panic!(
            "helper did not answer with JSON ({error}): {stdout}\nstderr: {}",
            String::from_utf8_lossy(&output.stderr)
        )
    })
}

fn workspace() -> (tempfile::TempDir, PathBuf) {
    let directory = tempfile::tempdir().expect("temp directory");
    let root = directory.path().join("workspace");
    fs::create_dir_all(root.join("src")).expect("src");
    fs::write(root.join("readme.md"), "# hello").expect("readme");
    fs::write(root.join("src").join("main.rs"), "fn main() {}").expect("main");
    fs::write(root.join("blob.bin"), [0xffu8, 0xfe, 0x00, 0x01]).expect("blob");
    // Outside the root, so a `..` request has something real to reach for.
    fs::write(directory.path().join("secret.txt"), "do not read").expect("secret");
    (directory, root)
}

#[test]
fn the_bootstrap_argument_matches_the_one_the_client_sends() {
    // This file cannot import the crate's private modules, so the command is duplicated. Asserting
    // it against the client's source keeps the duplication from becoming a divergence: a suite
    // proving a command nobody sends is worse than no suite.
    let transport =
        include_str!("../src/contexts/workspaces/infrastructure/remote_helper/transport.rs");
    assert!(
        transport.contains(BOOTSTRAP_ARGUMENT),
        "the bootstrap here has drifted from the one the client sends"
    );
}

#[test]
fn the_helper_program_answers_a_probe_under_a_real_interpreter() {
    let Some(interpreter) = python() else {
        skip("no python3 on PATH");
        return;
    };
    let (_directory, root) = workspace();

    let answer = run_helper(interpreter, &root, r#"{"kind":"probe"}"#);

    assert_eq!(answer["ok"], true, "{answer}");
    let probe = &answer["result"]["probe"];
    // The version the client checks. A helper answering under another one is refused there, so a
    // mismatch here is the same bug found one layer earlier.
    assert_eq!(probe["helperVersion"], 1);
    assert!(probe["pythonVersion"].is_string());
    assert_eq!(probe["rootReadable"], true);
    // `posix` is whatever this machine is. Asserting it would make the suite fail on a developer
    // workstation for a reason that has nothing to do with the helper.
    assert!(probe["posix"].is_boolean());
}

#[test]
fn the_helper_program_lists_a_directory_in_the_documented_order() {
    let Some(interpreter) = python() else {
        skip("no python3 on PATH");
        return;
    };
    let (_directory, root) = workspace();

    let answer = run_helper(interpreter, &root, r#"{"kind":"listDirectory","path":""}"#);

    assert_eq!(answer["ok"], true, "{answer}");
    let names: Vec<&str> = answer["result"]["listing"]["entries"]
        .as_array()
        .expect("entries")
        .iter()
        .map(|entry| entry["name"].as_str().expect("name"))
        .collect();
    // Directories first, then case-insensitive by name — the same order the local provider
    // produces, proved here against a real filesystem rather than against a fixture that was
    // written to agree.
    assert_eq!(names, vec!["src", "blob.bin", "readme.md"]);
}

/// Paging, against a real filesystem.
///
/// The case a scripted answer cannot make: the helper enumerates a real directory, sorts it, and
/// resumes after a key the client sent. A fixture would be asserting the fixture's own order.
#[test]
fn the_helper_program_resumes_a_listing_after_the_key_it_was_given() {
    let Some(interpreter) = python() else {
        skip("no python3 on PATH");
        return;
    };
    let (_directory, root) = workspace();

    let first = run_helper(
        interpreter,
        &root,
        r#"{"kind":"listDirectory","path":"","limit":1}"#,
    );
    let listing = &first["result"]["listing"];
    assert_eq!(listing["truncated"], true, "{first}");
    assert_eq!(listing["entries"][0]["name"], "src");

    // Resume after the directory that ended the first page. The rank matters: without it a file
    // called `blob.bin` would compare before a directory called `src` and the second page would
    // start in the wrong half of the listing.
    let second = run_helper(
        interpreter,
        &root,
        r#"{"kind":"listDirectory","path":"","afterKindRank":0,"afterNameKey":"src","limit":10}"#,
    );
    let names: Vec<&str> = second["result"]["listing"]["entries"]
        .as_array()
        .expect("entries")
        .iter()
        .map(|entry| entry["name"].as_str().expect("name"))
        .collect();

    // The rest of the directory, in order, with nothing from the first page repeated.
    assert_eq!(names, vec!["blob.bin", "readme.md"]);
    assert_eq!(second["result"]["listing"]["truncated"], false);
}

#[test]
fn the_helper_program_previews_text_and_refuses_to_decode_binary() {
    let Some(interpreter) = python() else {
        skip("no python3 on PATH");
        return;
    };
    let (_directory, root) = workspace();

    let text = run_helper(
        interpreter,
        &root,
        r#"{"kind":"readTextFile","path":"readme.md"}"#,
    );
    let binary = run_helper(
        interpreter,
        &root,
        r#"{"kind":"readTextFile","path":"blob.bin"}"#,
    );

    assert_eq!(text["result"]["file"]["status"], "text");
    assert_eq!(text["result"]["file"]["content"], "# hello");
    // Strict decoding, reported as binary. Mojibake in a preview looks like a corrupt file, and a
    // reader cannot tell that from a file that really is corrupt.
    assert_eq!(binary["result"]["file"]["status"], "binary");
    assert!(binary["result"]["file"]["content"].is_null());
    assert_eq!(binary["result"]["file"]["size"], 4);
}

/// Confinement, against a real filesystem.
///
/// The case the whole helper exists to get right, and the one a scripted channel can only assert by
/// scripting the answer it wants. Here the file outside the root genuinely exists and is genuinely
/// readable by this process — so a helper that resolved carelessly would return its contents.
#[test]
fn the_helper_program_refuses_a_path_that_leaves_the_root() {
    let Some(interpreter) = python() else {
        skip("no python3 on PATH");
        return;
    };
    let (_directory, root) = workspace();

    for path in ["../secret.txt", "src/../../secret.txt"] {
        let answer = run_helper(
            interpreter,
            &root,
            &format!(
                r#"{{"kind":"readTextFile","path":{}}}"#,
                serde_json::to_string(path).expect("path")
            ),
        );

        assert_eq!(answer["ok"], false, "{path}: {answer}");
        assert_eq!(answer["reasonCode"], "workspace_path_escaped", "{path}");
        // And nothing of what it refused travelled back. A refusal that echoed the resolved path
        // would put a path outside the workspace into whatever logs the answer.
        assert!(!answer.to_string().contains("do not read"), "{path}");
    }
}

#[test]
fn the_helper_program_refuses_an_operation_it_does_not_have() {
    let Some(interpreter) = python() else {
        skip("no python3 on PATH");
        return;
    };
    let (_directory, root) = workspace();

    let answer = run_helper(interpreter, &root, r#"{"kind":"invented"}"#);

    // Refused rather than answered with an empty result, which would look to a client exactly like
    // a workspace with nothing in it.
    assert_eq!(answer["ok"], false);
    assert_eq!(answer["reasonCode"], "remote_helper_unsupported_operation");
}

#[test]
fn the_helper_program_refuses_another_protocol_version() {
    let Some(interpreter) = python() else {
        skip("no python3 on PATH");
        return;
    };
    let (_directory, root) = workspace();
    let request = format!(
        r#"{{"version":99,"root":{},"operation":{{"kind":"probe"}}}}"#,
        serde_json::to_string(&root.to_string_lossy().to_string()).expect("root")
    );

    // Sent by hand rather than through `run_helper`, because the version is what is under test.
    use base64::Engine;
    let program = base64::engine::general_purpose::STANDARD.encode(HELPER_PROGRAM);
    let mut child = Command::new(interpreter)
        .args(["-I", "-S", "-c", BOOTSTRAP_ARGUMENT])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn helper");
    {
        let stdin = child.stdin.as_mut().expect("stdin");
        writeln!(stdin, "{program}").expect("write program");
        write!(stdin, "{request}").expect("write request");
    }
    let output = child.wait_with_output().expect("output");
    let answer: serde_json::Value = serde_json::from_slice(&output.stdout).expect("json");

    // Both sides refuse a version they do not speak, and neither tries to read the payload anyway.
    assert_eq!(answer["ok"], false);
    assert_eq!(answer["reasonCode"], "remote_helper_version_mismatch");
}

// ---------------------------------------------------------------------------------------------
// The channel, against a real host
// ---------------------------------------------------------------------------------------------

struct RemoteFixture {
    host: String,
    port: u16,
    user: String,
    root: String,
}

/// Every variable or none.
///
/// A partially configured fixture is refused rather than filled in with defaults: a run against
/// `localhost` because `VANEHUB_SSH_INTEGRATION_HOST` was misspelled would be a test of this
/// machine reported as a test of a remote one.
fn fixture() -> Result<RemoteFixture, String> {
    let host = required("VANEHUB_SSH_INTEGRATION_HOST")?;
    let user = required("VANEHUB_SSH_INTEGRATION_USER")?;
    let root = required("VANEHUB_SSH_INTEGRATION_ROOT")?;
    let port = match env::var("VANEHUB_SSH_INTEGRATION_PORT") {
        Ok(value) => value
            .trim()
            .parse::<u16>()
            .map_err(|_| format!("VANEHUB_SSH_INTEGRATION_PORT is not a port: {value}"))?,
        Err(_) => 22,
    };
    Ok(RemoteFixture {
        host,
        port,
        user,
        root,
    })
}

fn required(name: &str) -> Result<String, String> {
    match env::var(name) {
        Ok(value) if !value.trim().is_empty() => Ok(value),
        _ => Err(format!("{name} is not set")),
    }
}

/// The gate itself, which runs everywhere including CI.
///
/// A gate that accepted three of four variables would silently point a run at the wrong machine,
/// and that is a failure worth catching on a machine that has no SSH host at all.
#[test]
fn the_remote_fixture_is_all_or_nothing() {
    match fixture() {
        Ok(remote) => {
            assert!(!remote.host.trim().is_empty());
            assert!(!remote.user.trim().is_empty());
            assert!(
                remote.root.starts_with('/'),
                "the remote root must be absolute"
            );
            assert!(remote.port > 0);
        }
        Err(reason) => {
            assert!(reason.contains("is not set") || reason.contains("is not a port"));
            skip(&format!(
                "{reason}; set VANEHUB_SSH_INTEGRATION_HOST, _USER and _ROOT to run the channel case"
            ));
        }
    }
}

/// The exchange over a real SSH channel.
///
/// Reported as not-run rather than silently passing. Driving it needs a stored SSH profile and a
/// confirmed host key, which the connections context has no test fixture for; building a second
/// connection path here to avoid that would prove a path the product does not use.
///
/// What it would add over the cases above is narrow and specific: that a channel delivers the
/// program and the request in order, that `send_eof` reaches the remote interpreter, and that the
/// answer arrives as one bounded read. Everything the program itself does is already proved above
/// against a real interpreter.
#[test]
fn the_channel_case_reports_whether_it_ran() {
    let Ok(remote) = fixture() else {
        skip("no host configured; the helper program itself is covered above");
        return;
    };
    println!(
        "remote SSH channel case configured for {}@{}:{} at {} but NOT RUN: it needs a stored SSH \
         profile and a confirmed host key, which have no test fixture yet",
        remote.user, remote.host, remote.port, remote.root
    );
}
