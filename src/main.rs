extern crate hyper;
extern crate rustc_serialize;

use rustc_serialize::json;
use std::io::Read;
use hyper::client::Client;

/*
    [{
        "ContingencyAdj": [{
                "AdjType": 1,
                "DateFrom": "/Date(-62135568000000-0800)/",
                "DateThru": "/Date(-62135568000000-0800)/",
                "EventDescription": null,
                "EventID": null,
                "ReplacedBySchedRouteID": null
            }],
        "ContingencyOnly": false,
        "Description": "Anacortes / Sidney B.C.",
        "RegionID": 1,
        "RouteAbbrev": "ana-sid",
        "RouteID": 10,
        "SchedRouteID": 1627,
        "ScheduleID": 150,
        "SeasonalRouteNotes": "Note: The earlier international sailing to Sidney, B.C. stops at Friday Harbor westbound to load vehicles and passengers destined to Sidney, B.C., and the later eastbound sailing stops to unload vehicles and passengers from Sidney, B.C. to the islands. ",
        "ServiceDisruptions": []
    }]
*/

#[allow(non_snake_case)]
#[derive(RustcDecodable, RustcEncodable, Debug)]
struct SchedRoute {
    RouteID: u32,
    Description: String,
    RouteAbbrev: String,
}

fn main() {
    let api_key: &'static str = env!("WSDOT_API_KEY");
    let base = "http://www.wsdot.wa.gov/ferries/api/schedule/rest";
    let url = &format!("{}/schedroutes?apiaccesscode={}", base, api_key);

    println!("{}", url);
    let client = Client::new();
    let mut res = client.get(url).send().unwrap();

    assert_eq!(res.status, hyper::Ok);
    let mut s = String::new();
    res.read_to_string(&mut s).unwrap();
    let routs: Vec<SchedRoute> = json::decode(&s).unwrap();
    println!("{:?}", routs);
}
