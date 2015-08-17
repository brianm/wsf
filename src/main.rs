extern crate hyper;
extern crate rustc_serialize;
extern crate chrono;
extern crate regex;
extern crate env_logger;
#[macro_use] extern crate log;
extern crate docopt;

use std::ascii::AsciiExt;
use std::io::Read;
use std::env;
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::fmt;
use std::error::Error;
use std::result;

use chrono::naive::datetime::NaiveDateTime;
use chrono::offset::local::Local;
use chrono::offset::fixed::FixedOffset;
use chrono::datetime::DateTime;
use chrono::Datelike;

use rustc_serialize::json;
use rustc_serialize::Decodable;
use hyper::client::Client;

use regex::Regex;

use docopt::Docopt;

static USAGE: &'static str = "
Washing State Ferry Schedules

Usage:
  wsf [options] <from> <to>
  wsf -h

  <from> and <to> are a prefix of the departing terminal and arriving
  terminal, respectively. For example 'wsf sea ba' is equivalent to
  'wsf Seattle \"Bainbridge Island\"'.

Options:
  -h --help     Show this screen.
";

#[derive(Debug, RustcDecodable)]
struct Args {
    arg_from: String,
    arg_to: String,
}

fn main() {
    env_logger::init().unwrap();

    let args: Args = Docopt::new(USAGE)
                            .and_then(|d| d.decode())
                            .unwrap_or_else(|e| e.exit());

    let from_in: &str =  &args.arg_from.to_ascii_lowercase();
    let to_in: &str   = &args.arg_to.to_ascii_lowercase();

    // pull in api key at *build* time from environment
    let mut s = Session::new(env!("WSDOT_API_KEY"));

    let now = Local::now();
    let mut from: Option<i32> = None;
    let mut to: Option<i32> = None;
    for terminal in s.terminals().unwrap().iter() {
        if terminal.Description.to_ascii_lowercase().starts_with(&from_in) {
            from = Some(terminal.TerminalID);
        }
        if terminal.Description.to_ascii_lowercase().starts_with(&to_in) {
            to = Some(terminal.TerminalID);
        }
    }

    let tc = s.schedule(from.unwrap(), to.unwrap()).unwrap();
    for time in tc.Times.iter() {
        if time.depart_time() > now {
            println!("{}\t{}\t{}\t{}",
                     time.depart_time().time(),
                     tc.DepartingTerminalName,
                     tc.ArrivingTerminalName,
                     time.VesselName );
        }

    }
    s.save_cache();
}

struct Session {
    api_key: String,
    client: Client,
    cache: Cache,
    cacheflushdate: String,
    cache_path: String,
    offline: bool,
}

impl Session {

    fn new(api_key: &str) -> Session {
        let mut cache_path: PathBuf = env::home_dir().unwrap();
        cache_path.push(".wsf.cache");
        let cache_path = format!("{}", cache_path.display());

        let mut s = Session {
            api_key: api_key.to_string(),
            client: Client::new(),
            cache: Cache::load(&cache_path),
            cacheflushdate: String::new(),
            cache_path: cache_path,
            offline: false,
        };

        s.offline = match s.get::<String>(format!("/cacheflushdate")) {
            Ok(cfd) => {
                    s.cacheflushdate = cfd;
                    false
                },
            Err(_) => true,
        };
        s
    }

    fn save_cache(&mut self) {
        self.cache.cache_flush_date = self.cacheflushdate.clone();
        let mut f = File::create(&self.cache_path).unwrap();
        let encoded = json::encode(&self.cache).unwrap();
        f.write_all(encoded.as_bytes()).unwrap();
    }

    fn get<T: Decodable>(&self, path: String) -> Result<T> {
        let url = &format!("http://www.wsdot.wa.gov/ferries/api/schedule/rest{}?apiaccesscode={}",
                            path,
                            self.api_key);
        let mut res = try!(self.client.get(url).send());
        assert_eq!(res.status, hyper::Ok);

        let mut buf = String::new();
        try!(res.read_to_string(&mut buf));
        Ok(try!(json::decode::<T>(&buf)))
    }

    fn terminals(&mut self) -> Result<Vec<Terminal>> {
        if self.offline || (self.cache.cache_flush_date == self.cacheflushdate) {
            return Ok(self.cache.terminals.clone())
        }
        else {
            let now = Local::today();
            let path = format!("/terminals/{}-{}-{}", now.year(), now.month(), now.day());
            let routes: Vec<Terminal> = try!(self.get(path));
            self.cache.terminals = routes.clone();
            return Ok(routes);
        }
    }

    fn schedule(&mut self, from: i32, to: i32) -> Result<TerminalCombo> {
        let mut cache_is_stale = true;
        let cache_key = format!("{} {}", from, to);

        if self.offline || (self.cache.cache_flush_date == self.cacheflushdate) {
            if self.cache.sailings.contains_key(&cache_key) {
                // cache is up to date and has route!
                // unwrap is correct as we checked for enry first
                return Ok(self.cache.sailings.get(&cache_key).unwrap().clone());
            }
            else {
                // cache is up to date, but we don't have this route in it
                cache_is_stale = false;
            }
        }

        if cache_is_stale {
            self.cache.sailings.clear();
        }

        let now = Local::now();
        let path = format!("/schedule/{}-{}-{}/{}/{}",
                            now.year(), now.month(), now.day(), from, to);

        let schedule: Schedule = try!(self.get(path));

        self.cache.sailings.insert(cache_key, schedule.TerminalCombos[0].clone());
        Ok(schedule.TerminalCombos[0].clone())
    }
}

#[derive(RustcDecodable, RustcEncodable, Debug)]
struct Cache {
    terminals: Vec<Terminal>,
    sailings: HashMap<String, TerminalCombo>,
    cache_flush_date: String,
}

impl Cache {
    fn load(path: &String) -> Cache {
        let r = File::open(path);
        match r {
            Ok(mut f) => {
                let mut s = String::new();
                f.read_to_string(&mut s).unwrap();
                let cache = json::decode(&s).unwrap();
                cache
            },
            Err(_) => {
                Cache {
                    terminals: vec![],
                    sailings: HashMap::new(),
                    cache_flush_date: String::new(),
                }
            }
        }
    }
}

#[allow(non_snake_case)]
#[derive(RustcDecodable, RustcEncodable, Debug, Clone)]
struct Terminal {
    TerminalID: i32,
    Description: String,
}

#[allow(non_snake_case)]
#[derive(RustcDecodable, RustcEncodable, Clone, Debug)]
struct SailingTime {
    DepartingTime: String,
    ArrivingTime: Option<String>,
    VesselName: String,
}

impl SailingTime {
    // parse date strings of form "/Date(1436318400000-0700)/"
    fn depart_time(&self) -> DateTime<Local> {
        let re = Regex::new(r"^/Date\((\d{10})000-(\d{2})(\d{2})\)/$").unwrap();
        let caps = re.captures(&self.DepartingTime).unwrap();

        let epoch: i64 = caps.at(1).unwrap().parse().unwrap();
        let tz_hours: i32 = caps.at(2).unwrap().parse().unwrap();
        let tz_minutes: i32 = caps.at(3).unwrap().parse().unwrap();

        let nd = NaiveDateTime::from_timestamp(epoch, 0);
        let tz = FixedOffset::west((tz_hours * 3600) + (tz_minutes * 60));
        let fotz: DateTime<FixedOffset> = DateTime::from_utc(nd, tz);
        fotz.with_timezone(&Local)
    }
}

#[allow(non_snake_case)]
#[derive(RustcDecodable, RustcEncodable, Clone, Debug)]
struct TerminalCombo {
    Times: Vec<SailingTime>,
    DepartingTerminalName: String,
    ArrivingTerminalName: String,
}

#[allow(non_snake_case)]
#[derive(RustcDecodable, RustcEncodable, Debug)]
struct Schedule {
    TerminalCombos: Vec<TerminalCombo>,
}

type Result<T> = result::Result<T, CliError>;

#[derive(Debug)]
enum CliError {
    Parse(rustc_serialize::json::DecoderError),
    Http(hyper::error::Error),
    Input(std::io::Error),
}


impl From<hyper::error::Error> for CliError {
    fn from(err: hyper::error::Error) -> CliError {
        CliError::Http(err)
    }
}


impl From<std::io::Error> for CliError {
    fn from(err: std::io::Error) -> CliError {
        CliError::Input(err)
    }
}

impl From<rustc_serialize::json::DecoderError> for CliError {
    fn from(err: rustc_serialize::json::DecoderError) -> CliError {
        CliError::Parse(err)
    }
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        /*
        match *self {
            CliError::Io(ref err) => err.fmt(f),
            CliError::Csv(ref err) => err.fmt(f),
            CliError::NotFound => write!(f, "No matching cities with a \
                                             population were found."),
        }
        */
        write!(f, "oops!")
    }
}

impl Error for CliError {
    fn description(&self) -> &str {
        /*
        match *self {
            CliError::Io(ref err) => err.description(),
            CliError::Csv(ref err) => err.description(),
            CliError::NotFound => "not found",
        }
        */
        "broke and went boom"
    }
}
