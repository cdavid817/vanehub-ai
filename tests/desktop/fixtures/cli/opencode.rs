use std::env;
use std::io::{self, BufRead, Write};
use std::thread;
use std::time::Duration;

fn main() -> io::Result<()> {
    if env::args().skip(1).any(|arg| arg == "--version") {
        println!("vanehub-fixture 1.0.0");
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
