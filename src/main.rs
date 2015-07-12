extern crate hyper;
extern crate rustc_serialize;
extern crate chrono;
extern crate regex;
extern crate env_logger;
#[macro_use] extern crate log;

use std::ascii::AsciiExt;
use std::io::Read;
use std::env;
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

use chrono::naive::datetime::NaiveDateTime;
use chrono::offset::local::Local;
use chrono::offset::fixed::FixedOffset;
use chrono::datetime::DateTime;
use chrono::Datelike;

use rustc_serialize::json;
use hyper::client::Client;
use regex::Regex;

fn main() {
    env_logger::init().unwrap();

    let mut args      = env::args();
    let _program      = args.next().expect("program should be first arg!");
    let from_in: &str = &args.next().expect("must specify from");
    let to_in: &str   = &args.next().expect("must specify to");

    // pull in api key at *build* time from environment
    let mut s = Session::new(env!("WSDOT_API_KEY"));

    let now = Local::now();
    let mut from: Option<i32> = None;
    let mut to: Option<i32> = None;
    for terminal in s.terminals().iter() {
        if terminal.Description.to_ascii_lowercase().starts_with(&from_in) {
            from = Some(terminal.TerminalID);
        }
        if terminal.Description.to_ascii_lowercase().starts_with(&to_in) {
            to = Some(terminal.TerminalID);
        }
    }

    let tc = s.schedule(from.unwrap(), to.unwrap());
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
        };

        let url = &s.url(format!("/cacheflushdate"));
        let mut r = s.client.get(url).send().unwrap();
        assert_eq!(r.status, hyper::Ok);
        r.read_to_string(&mut s.cacheflushdate).unwrap();
        s
    }

    fn save_cache(&mut self) {
        self.cache.cache_flush_date = self.cacheflushdate.clone();
        let mut f = File::create(&self.cache_path).unwrap();
        let encoded = json::encode(&self.cache).unwrap();
        f.write_all(encoded.as_bytes()).unwrap();
    }

    fn url(&self, path: String) -> String {
        format!("http://www.wsdot.wa.gov/ferries/api/schedule/rest{}?apiaccesscode={}",
                path,
                self.api_key)
    }

    fn terminals(&mut self) -> Vec<Terminal> {
        if self.cache.cache_flush_date == self.cacheflushdate {
            self.cache.terminals.clone()
        }
        else {
            let now = Local::today();
            let url = &self.url(format!("/terminals/{}-{}-{}", now.year(), now.month(), now.day()));

            let mut res = self.client.get(url).send().unwrap();
            assert_eq!(res.status, hyper::Ok);

            let mut buf = String::new();
            res.read_to_string(&mut buf).unwrap();
            let routes: Vec<Terminal> = json::decode(&buf).unwrap();
            self.cache.terminals = routes.clone();
            routes
        }
    }

    fn schedule(&mut self, from: i32, to: i32) -> TerminalCombo {
        let mut cache_is_stale = true;
        let cache_key = format!("{} {}", from, to);

        if self.cache.cache_flush_date == self.cacheflushdate {
            if self.cache.sailings.contains_key(&cache_key) {
                // cache is up to date and has route!
                return self.cache.sailings.get(&cache_key).unwrap().clone()
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
        let url = &self.url(format!("/schedule/{}-{}-{}/{}/{}",
                                    now.year(), now.month(), now.day(), from, to));

        let mut res = self.client.get(url).send().unwrap();
        assert_eq!(res.status, hyper::Ok);

        let mut buf = String::new();
        res.read_to_string(&mut buf).unwrap();
        let schedule: ScheduleResult = json::decode(&buf).unwrap();
        self.cache.sailings.insert(cache_key, schedule.TerminalCombos[0].clone());
        schedule.TerminalCombos[0].clone()
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
struct ScheduleResult {
    TerminalCombos: Vec<TerminalCombo>,
}
