pub mod config;
pub mod daemon;
pub mod ipc;
pub mod log;
pub mod pin;
pub mod scratch;

use std::io;

const USAGE: &str = "\
usage: niri-helper <command>

commands:
  daemon              run the event-stream daemon (default)

when no command is given, `daemon` is run.";

pub async fn run() -> io::Result<()> {
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        None | Some("daemon") => daemon::run().await,
        Some("-h" | "--help") => {
            println!("{USAGE}");
            Ok(())
        }
        Some(other) => {
            eprintln!("niri-helper: unknown command '{other}'\n\n{USAGE}");
            std::process::exit(2);
        }
    }
}
