#[macro_use]
extern crate serde_derive;

use anyhow::Result;
use chrono::prelude::*;
use docopt::Docopt;

static USAGE: &str = "
Washington State Ferry Schedules

Usage:
  wsf [options] <from> <to>
  wsf (-h | --help)

  <from> and <to> are a prefix of the departing terminal and arriving
  terminal, respectively. For example 'wsf sea ba' is equivalent to
  'wsf Seattle \"Bainbridge Island\"'.

Options:
  -a --all      Show all times for today, not just remaining ones
  -h --help     Show this screen.
";

#[derive(Debug, Deserialize)]
struct Args {
    arg_from: String,
    arg_to: String,
    flag_all: bool,
}

async fn run() -> Result<()> {
    let args: Args = Docopt::new(USAGE)
        .and_then(|d| d.deserialize())
        .unwrap_or_else(|e| e.exit());

    let from_in: &str = &args.arg_from.to_ascii_lowercase();
    let to_in: &str = &args.arg_to.to_ascii_lowercase();

    let mut s = wsf::Session::new("afddf683-37c5-4d1a-8486-f7004a16d86d").await;

    let from = s.find_terminal(from_in).await?.TerminalID;
    let to = s.find_terminal(to_in).await?.TerminalID;

    let tc = s.schedule(from, to).await?;

    let now = Local::now();
    for time in tc.Times.iter() {
        if args.flag_all || time.depart_time()? > now {
            println!(
                "{}\t{}\t{}\t{}",
                time.depart_time()?.time(),
                tc.DepartingTerminalName,
                tc.ArrivingTerminalName,
                time.VesselName
            );
        };
    }
    s.save_cache()?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    human_panic::setup_panic!();

    let env = env_logger::Env::default().filter_or("WSF_LOG", "info");
    env_logger::init_from_env(env);

    run().await
}
