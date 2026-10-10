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
  daemon        run the event-stream daemon (default)
  scratch name  bring, spawn, or hide the scratchpad application

`scratch` operates on an application declared under [scratchpad.applications].
when no command is given, `daemon` is run.";

pub async fn run() -> io::Result<()> {
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        None | Some("daemon") => daemon::run().await,
        Some("scratch") => {
            let Some(name) = args.next() else {
                eprintln!("niri-helper: `scratch` requires an application name\n\n{USAGE}");
                std::process::exit(2);
            };
            if args.next().is_some() {
                eprintln!("niri-helper: unexpected extra arguments\n\n{USAGE}");
                std::process::exit(2);
            }
            scratch::run(name).await
        }
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
