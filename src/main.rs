#[macro_use]
extern crate serde_derive;

use chrono::offset::local::Local;
use docopt::Docopt;
use env_logger;
use failure::Error;
use wsf;

static USAGE: &'static str = "
Washing State Ferry Schedules

Usage:
  wsf [options] <from> <to>
  wsf (-h | --help)

  <from> and <to> are a prefix of the departing terminal and arriving
  terminal, respectively. For example 'wsf sea ba' is equivalent to
  'wsf Seattle \"Bainbridge Island\"'.

Options:
  -a --all      Show all times for today, not just remaining
  -h --help     Show this screen.
";

#[derive(Debug, Deserialize)]
struct Args {
    arg_from: String,
    arg_to: String,
    flag_all: bool,
}

fn run() -> Result<(), Error> {
    let env = env_logger::Env::default().filter_or("WSF_LOG", "info");

    env_logger::init_from_env(env);

    let args: Args = Docopt::new(USAGE)
        .and_then(|d| d.deserialize())
        .unwrap_or_else(|e| e.exit());

    let from_in: &str = &args.arg_from.to_ascii_lowercase();
    let to_in: &str = &args.arg_to.to_ascii_lowercase();

    let mut s = wsf::Session::new("afddf683-37c5-4d1a-8486-f7004a16d86d");

    let from = s.find_terminal(&from_in)?.TerminalID;
    let to = s.find_terminal(&to_in)?.TerminalID;

    let tc = s.schedule(from, to)?;

    let now = Local::now();
    for time in tc.Times.iter() {
        if args.flag_all {
            println!(
                "{}\t{}\t{}\t{}",
                time.depart_time().time(),
                tc.DepartingTerminalName,
                tc.ArrivingTerminalName,
                time.VesselName
            );
        } else if time.depart_time() > now {
            println!(
                "{}\t{}\t{}\t{}",
                time.depart_time().time(),
                tc.DepartingTerminalName,
                tc.ArrivingTerminalName,
                time.VesselName
            );
        }
    }
    Ok(s.save_cache()?)
}

fn main() {
    match run() {
        Ok(_) => {}
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(1);
        }
    }
}
