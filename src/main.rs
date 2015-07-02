extern crate hyper;

use std::io::Read;
use hyper::client::{Client};

fn main() {
    let api_key: &'static str = env!("WSDOT_API_KEY");
    let url = &format!("http://www.wsdot.wa.gov/ferries/api/schedule/rest/schedroutes?apiaccesscode={}", api_key);

    let client = Client::new();    
    let mut res = client.get(url).send().unwrap();

    assert_eq!(res.status, hyper::Ok);
    let mut s = String::new();
    res.read_to_string(&mut s).unwrap();
    println!("{}", s);
}
