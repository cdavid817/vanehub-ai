use std::env;
use std::io::{self, BufRead, Write};
use std::thread;
use std::time::Duration;

fn main() -> io::Result<()> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.iter().any(|arg| arg == "--version") {
        println!("vanehub-fixture 1.0.0");
        return Ok(());
    }
    if args.iter().any(|arg| arg == "run") {
        let prompt = args.join(" ");
        if prompt.contains("VANEHUB_E2E_SLOW") {
            thread::sleep(Duration::from_millis(1_000));
        }
        let final_text = if prompt.contains("VANEHUB_E2E_OVERSIZED") {
            "界".repeat(20_001)
        } else {
            "deterministic fixture final".to_string()
        };
        println!(
            r#"{{"type":"text","sessionID":"fixture-session","part":{{"id":"fixture-text","messageID":"fixture-message","sessionID":"fixture-session","type":"text","text":"{final_text}"}}}}"#
        );
        println!(
            r#"{{"type":"step_finish","sessionID":"fixture-session","part":{{"id":"fixture-finish","reason":"stop","messageID":"fixture-message","sessionID":"fixture-session","type":"step-finish"}}}}"#
        );
        return Ok(());
    }

    // The readiness probes the registry declares, answered and exited rather than fallen through
    // to the interactive loop. `registry.rs` runs exactly these: `doctor` for claude-code,
    // `login status` for codex-cli, `auth list` for opencode, each with a bounded budget. Blocking
    // on stdin until that budget expires marks the executable faulty, and launch resolution then
    // refuses an Agent the management page has just listed as runnable.
    //
    // Matched by argument rather than by asking whether stdin is a terminal. That check looked
    // equivalent and is not: an Agent terminal session is also driven over pipes, so treating every
    // pipe as a probe made real sessions exit instantly, and the Loop centre tripped its error
    // boundary with "Agent terminal is not connected".
    if let Some(first) = env::args().nth(1) {
        if matches!(first.as_str(), "doctor" | "login" | "auth") {
            println!("VANEHUB-FIXTURE-CLI READY");
            io::stdout().flush()?;
            return Ok(());
        }
    }

    thread::sleep(Duration::from_millis(500));
    println!("VANEHUB-FIXTURE-CLI READY");
    io::stdout().flush()?;
    for line in io::stdin().lock().lines() {
        let line = line?;
        if line.eq_ignore_ascii_case("vanehub-fixture-stop") {
            println!("VANEHUB-FIXTURE-CLI STOPPING");
            break;
        }
        if !line.is_empty() {
            println!("VANEHUB-FIXTURE-ECHO {line}");
        }
        io::stdout().flush()?;
    }
    Ok(())
}
