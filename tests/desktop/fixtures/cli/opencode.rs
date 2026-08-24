use std::env;
use std::io::{self, BufRead, IsTerminal, Write};
use std::thread;
use std::time::Duration;

fn main() -> io::Result<()> {
    if env::args().skip(1).any(|arg| arg == "--version") {
        println!("vanehub-fixture 1.0.0");
        return Ok(());
    }

    // Anything that is not a terminal is a probe, not a session: readiness checks such as
    // `auth list` run this with pipes and a bounded budget. Falling through to the interactive loop
    // blocked on stdin until that budget expired, and a timed-out probe marks the executable
    // faulty -- which made launch resolution refuse an Agent the management page had just listed as
    // runnable, so every spec that needed one failed with "not found on PATH".
    if !io::stdin().is_terminal() {
        println!("VANEHUB-FIXTURE-CLI READY");
        io::stdout().flush()?;
        return Ok(());
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
