extern crate hyper;
extern crate rustc_serialize;
extern crate chrono;
extern crate regex;

use std::ascii::AsciiExt;
use std::io::Read;
use std::env;

use chrono::naive::datetime::NaiveDateTime;
use chrono::offset::local::Local;
use chrono::offset::fixed::FixedOffset;
use chrono::datetime::DateTime;
use chrono::Datelike;

use rustc_serialize::json;
use hyper::client::Client;
use regex::Regex;

fn main() {
    let mut args      = env::args();
    let _program      = args.next().expect("program should be first arg!");
    let from_in: &str = &args.next().expect("must specify from");
    let to_in: &str   = &args.next().expect("must specify to");

    // pull in api key at *build* time from environment
    let s = Session::new(env!("WSDOT_API_KEY"));

    let now = Local::now();
    let mut from: Option<i32> = None;
    let mut to: Option<i32> = None;
    let terminals = s.terminals();
    for terminal in terminals.iter() {
        if terminal.Description.to_ascii_lowercase().starts_with(&from_in) {
            from = Some(terminal.TerminalID);
        }
        if terminal.Description.to_ascii_lowercase().starts_with(&to_in) {
            to = Some(terminal.TerminalID);
        }
    }

    let url = &s.url(format!("/schedule/{}-{}-{}/{}/{}",
                             now.year(), now.month(), now.day(), from.unwrap(), to.unwrap()));

    let mut res = s.client.get(url).send().unwrap();
    assert_eq!(res.status, hyper::Ok);

    let mut buf = String::new();
    res.read_to_string(&mut buf).unwrap();
    let schedule: ScheduleResult = json::decode(&buf).unwrap();

    assert_eq!(1, schedule.TerminalCombos.len());
    let tc = &schedule.TerminalCombos[0];
    for time in tc.Times.iter() {
        if time.depart_time() > now {
            println!("{}\t{}\t{}\t{}",
                     time.depart_time().time(),
                     tc.DepartingTerminalName,
                     tc.ArrivingTerminalName,
                     time.VesselName );
        }

    }
}

struct Session {
    api_key: String,
    client: Client,
}

impl Session {
    fn new(api_key: &str) -> Session {
        Session {
            api_key: api_key.to_string(),
            client: Client::new(),
        }
    }

    fn url(&self, path: String) -> String {
        format!("http://www.wsdot.wa.gov/ferries/api/schedule/rest{}?apiaccesscode={}",
                path,
                self.api_key)
    }

    fn terminals(&self) -> Vec<Terminal> {
        let now = Local::today();
        let url = &self.url(format!("/terminals/{}-{}-{}", now.year(), now.month(), now.day()));

        let mut res = self.client.get(url).send().unwrap();
        assert_eq!(res.status, hyper::Ok);

        let mut buf = String::new();
        res.read_to_string(&mut buf).unwrap();
        let routes: Vec<Terminal> = json::decode(&buf).unwrap();
        routes
    }
}

#[allow(non_snake_case)]
#[derive(RustcDecodable, RustcEncodable, Debug, Clone)]
struct Terminal {
    TerminalID: i32,
    Description: String,
}

#[allow(non_snake_case)]
#[derive(RustcDecodable, RustcEncodable, Debug)]
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
#[derive(RustcDecodable, RustcEncodable, Debug)]
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
