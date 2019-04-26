use dirs;

use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::io::Write;
use std::path::PathBuf;

use chrono::datetime::DateTime;
use chrono::naive::datetime::NaiveDateTime;
use chrono::offset::fixed::FixedOffset;
use chrono::offset::local::Local;
use chrono::Datelike;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json;

use reqwest;

use failure::Fail;
use regex::Regex;
use std::result;

type Result<T> = result::Result<T, failure::Error>;

pub struct Session {
    api_key: String,
    cache: Cache,
    cacheflushdate: String,
    cache_path: String,
    offline: bool,
}

impl Session {
    pub fn new(api_key: &str) -> Session {
        let mut cache_path: PathBuf = dirs::home_dir().unwrap();
        cache_path.push(".wsf.cache");
        let cache_path = cache_path.display().to_string();

        let mut s = Session {
            api_key: api_key.to_string(),
            cache: Cache::load(&cache_path).unwrap_or_else(|_| Cache::empty()),
            cacheflushdate: String::new(),
            cache_path,
            offline: false,
        };

        // TODO this is kind of gross, cfd as optional to indicate offline, maybe?
        s.offline = match s.get::<String>("/cacheflushdate".to_owned()) {
            Ok(cfd) => {
                s.cacheflushdate = cfd;
                false
            }
            Err(_) => true,
        };
        s
    }

    pub fn save_cache(&mut self) -> Result<()> {
        self.cache.cache_flush_date = self.cacheflushdate.clone();

        let mut f = File::create(&self.cache_path)?;
        let encoded = serde_json::to_string(&self.cache)?;
        f.write_all(encoded.as_bytes())?;
        Ok(())
    }

    fn get<T: DeserializeOwned>(&self, path: String) -> Result<T> {
        let url = &format!(
            "http://www.wsdot.wa.gov/ferries/api/schedule/rest{}?apiaccesscode={}",
            path, self.api_key
        );
        let mut response = reqwest::get(url)?;

        let it: T = response.json()?;
        Ok(it)
    }

    pub fn find_terminal(&mut self, term: &str) -> Result<Terminal> {
        let r = self
            .terminals()?
            .iter()
            .cloned()
            .find(|t| t.Description.to_ascii_lowercase().starts_with(&term))
            .ok_or_else(|| WsfError::TerminalNotFound(term.to_string()));
        Ok(r?)
    }

    pub fn terminals(&mut self) -> Result<Vec<Terminal>> {
        if self.offline || (self.cache.cache_flush_date == self.cacheflushdate) {
            Ok(self.cache.terminals.clone())
        } else {
            let now = Local::today();
            let path = format!("/terminals/{}-{}-{}", now.year(), now.month(), now.day());
            let routes: Vec<Terminal> = self.get(path)?;
            self.cache.terminals = routes.clone();
            Ok(routes)
        }
    }

    pub fn schedule(&mut self, from: i32, to: i32) -> Result<TerminalCombo> {
        let mut cache_is_stale = true;
        let cache_key = format!("{} {}", from, to);

        if self.offline || (self.cache.cache_flush_date == self.cacheflushdate) {
            if self.cache.sailings.contains_key(&cache_key) {
                // cache is up to date and has route!
                return Ok(self
                    .cache
                    .sailings
                    .get(&cache_key)
                    .expect("checked for key in cache then not found")
                    .clone());
            } else {
                // cache is up to date, but we don't have this route in it
                cache_is_stale = false;
            }
        }

        if cache_is_stale {
            self.cache.sailings.clear();
        }

        let now = Local::now();
        let path = format!(
            "/schedule/{}-{}-{}/{}/{}",
            now.year(),
            now.month(),
            now.day(),
            from,
            to
        );

        let schedule: Schedule = self.get(path)?;

        self.cache
            .sailings
            .insert(cache_key, schedule.TerminalCombos[0].clone());
        Ok(schedule.TerminalCombos[0].clone())
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct Cache {
    terminals: Vec<Terminal>,
    sailings: HashMap<String, TerminalCombo>,
    cache_flush_date: String,
}

impl Cache {
    fn empty() -> Cache {
        Cache {
            terminals: vec![],
            sailings: HashMap::new(),
            cache_flush_date: String::new(),
        }
    }

    fn load(path: &str) -> Result<Cache> {
        let mut f = File::open(path)?;
        let mut s = String::new();
        f.read_to_string(&mut s)?;
        let cache: Cache = serde_json::from_str(&s)?;
        Ok(cache)
    }
}

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Terminal {
    pub TerminalID: i32,
    pub Description: String,
}

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SailingTime {
    pub DepartingTime: String,
    pub ArrivingTime: Option<String>,
    pub VesselName: String,
}

impl SailingTime {
    // parse date strings of form "/Date(1436318400000-0700)/"
    pub fn depart_time(&self) -> DateTime<Local> {
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
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TerminalCombo {
    pub Times: Vec<SailingTime>,
    pub DepartingTerminalName: String,
    pub ArrivingTerminalName: String,
}

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Debug)]
pub struct Schedule {
    pub TerminalCombos: Vec<TerminalCombo>,
}

#[derive(Debug, Fail)]
pub enum WsfError {
    #[fail(display = "Terminal not found: {}", _0)]
    TerminalNotFound(String),
}

