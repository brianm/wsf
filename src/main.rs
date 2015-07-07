extern crate hyper;
extern crate rustc_serialize;
extern crate chrono;
extern crate regex;

use std::io::Read;
use std::env;

use chrono::*;
use rustc_serialize::json;
use hyper::client::Client;
use regex::Regex;

struct Session {
    api_key: &'static str,
}

impl Session {
    fn new(api_key: &'static str) -> Session {
        Session { api_key: api_key }
    }

    fn url(&self, path: String) -> String {
        format!("http://www.wsdot.wa.gov/ferries/api/schedule/rest{}?apiaccesscode={}",
                path,
                self.api_key)
    }
}

#[allow(non_snake_case)]
#[derive(RustcDecodable, Debug)]
struct SailingTime {
    DepartingTime: String,
    ArrivingTime: Option<String>
}


impl SailingTime {
    // parse strings of form "/Date(1436318400000-0700)/"
    fn depart_time(&self, ltz: Local) -> DateTime<Local> {
        let re = Regex::new(r"^/Date\((\d{10})000-(\d{2})(\d{2})\)/$").unwrap();
        let caps = re.captures(&self.DepartingTime).unwrap();

        let epoch_s = caps.at(1).unwrap();
        let epoch: i64 =  epoch_s.parse().unwrap();

        let tzhs = caps.at(2).unwrap();
        let tzhi: i32 = tzhs.parse().unwrap();

        let tzms = caps.at(3).unwrap();
        let tzmi: i32 = tzms.parse().unwrap();

        let nd = NaiveDateTime::from_timestamp(epoch, 0);
        let tz = FixedOffset::west((tzhi * 3600) + (tzmi * 60));
        let fotz: DateTime<FixedOffset> = DateTime::from_utc(nd, tz);
        fotz.with_timezone(&ltz)
    }
}


#[allow(non_snake_case)]
#[derive(RustcDecodable, Debug)]
struct TerminalCombo {
    Times: Vec<SailingTime>,
}

#[allow(non_snake_case)]
#[derive(RustcDecodable, Debug)]
struct ScheduleResult {
    TerminalCombos: Vec<TerminalCombo>,
}

fn main() {
    let mut args = env::args();
    let _program = args.next().unwrap();
    let route: &str = &args.next()
                           .expect("must pass one of sea-bi or bi-sea");

    let (from, to) = match route {
        "sea-bi" => (7,3),
        "bi-sea" => (3, 7),
        _ => panic!("only bi-sea and sea-bi supported"),
    };

    let now = Local::now();
    let s = Session::new(env!("WSDOT_API_KEY"));
    let url = &s.url(format!("/schedule/{}-{}-{}/{}/{}",
                             now.year(), now.month(), now.day(),
                             from, to));
    //println!("{}", url);

    let client = Client::new();
    let mut res = client.get(url).send().unwrap();

    assert_eq!(res.status, hyper::Ok);
    let mut s = String::new();
    res.read_to_string(&mut s).unwrap();
    let routes: ScheduleResult = json::decode(&s).unwrap();

    assert_eq!(1, routes.TerminalCombos.len());
    for time in routes.TerminalCombos[0].Times.iter() {
        let dt = time.depart_time(now.timezone());
        if dt > now {
            println!("{}", dt.time());
        }
    }
}

#[test]
fn test_regex() {
    // parse strings of form "/Date(1436318400000-0700)/"
    let re = Regex::new(r"^/Date\((\d{10})000-(\d{4})\)/$").unwrap();
    let caps = re.captures("/Date(1436318400000-0700)/").unwrap();
    assert_eq!(caps.at(1).unwrap(), "1436318400");
    assert_eq!(caps.at(2).unwrap(), "0700");
}
