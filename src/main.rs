use std::collections::HashMap;

use color_eyre::eyre::Result;
use dotenv::dotenv;
use reqwest::Method;
use serde::{Deserialize, Serialize};

#[derive(Clone, Deserialize, Debug, Serialize, PartialEq)]
struct Address {
    name: String,
    lat: f64,
    lon: f64,
}

#[derive(Clone, Deserialize, Debug, Serialize)]
struct Route {
    #[serde(rename = "shortName")]
    short_name: String,
}

#[derive(Clone, Deserialize, Debug, Serialize)]
struct Trip {
    route: Route,
}

#[derive(Clone, Deserialize, Debug, Serialize)]
struct ItineraryLeg {
    mode: String,
    duration: f64,
    distance: f64,
    trip: Option<Trip>,
}

#[derive(Clone, Deserialize, Debug, Serialize)]
struct Itinerary {
    legs: Vec<ItineraryLeg>,
    duration: f64,
    #[serde(rename = "walkDistance")]
    walk_distance: f64,
}

#[derive(Clone, Debug)]
struct Solution {
    addresses: Vec<Address>,
    itineraries: Vec<Itinerary>,
}

fn main() -> Result<()> {
    dotenv().ok();
    color_eyre::install()?;

    let file = std::fs::read("./addresses.json")?;
    let addresses: Vec<Address> = serde_json::from_slice(&file)?;

    let itineraries = match std::fs::read("./itineraries.json").ok() {
        Some(data) => {
            dbg!("Found itineraries.json, skipping fetch");
            serde_json::from_slice(&data)?
        }
        None => get_all_itineraries(&addresses)?,
    };

    find_shortest_route(&addresses, &itineraries)?;

    Ok(())
}

// Find shortest route that visits all addresses
fn find_shortest_route(addresses: &Vec<Address>, itineraries: &AllItineraries) -> Result<()> {
    use itertools::Itertools;

    let perms = addresses.iter().permutations(addresses.len());
    let mut shortest_distance = std::f64::MAX;
    let mut shortest_duration = std::f64::MAX;
    let mut shortest_solution: Option<Solution> = None;
    let mut longest_distance = 0.0;
    let mut longest_duration = 0.0;
    let mut longest_solution: Option<Solution> = None;

    for permutation in perms {
        let mut total_duration = 0.0;
        let mut total_distance = 0.0;
        let mut total_walk_distance = 0.0;
        let mut legs: Vec<ItineraryLeg> = Vec::new();

        for (from, to) in permutation.iter().tuple_windows() {
            let itinerary = itineraries.get(&from.name).unwrap().get(&to.name).unwrap();

            total_duration += itinerary.duration;
            total_distance += itinerary.legs.iter().map(|leg| leg.distance).sum::<f64>();
            total_walk_distance += itinerary.walk_distance;
            legs.extend(itinerary.legs.clone());
        }

        println!(
            "Total duration: {:.0} min, total distance: {:.1} km, total walk distance: {:.1} km",
            total_duration / 60.0,
            total_distance / 1000.0,
            total_walk_distance / 1000.0
        );

        // println!("Legs: {:#?}", legs);

        if total_duration < shortest_duration {
            shortest_distance = total_distance;
            shortest_duration = total_duration;
            shortest_solution = Some(Solution {
                addresses: permutation.iter().map(|a| a.clone().clone()).collect(),
                itineraries: permutation
                    .iter()
                    .tuple_windows()
                    .map(|(from, to)| itineraries[&from.name][&to.name].clone())
                    .collect(),
            });
        }

        if total_duration > longest_duration {
            longest_distance = total_distance;
            longest_duration = total_duration;
            longest_solution = Some(Solution {
                addresses: permutation.iter().map(|a| a.clone().clone()).collect(),
                itineraries: permutation
                    .iter()
                    .tuple_windows()
                    .map(|(from, to)| itineraries[&from.name][&to.name].clone())
                    .collect(),
            });
        }
    }

    println!("Longest distance: {:.1} km", longest_distance / 1000.0);
    println!("Longest duration: {:.0} min", longest_duration / 60.0);
    println!("Longest solution: {:#?}", longest_solution);

    println!("Shortest distance: {:.1} km", shortest_distance / 1000.0);
    println!("Shortest duration: {:.0} min", shortest_duration / 60.0);
    println!("Shortest solution: {:#?}", shortest_solution);

    Ok(())
}

#[derive(Deserialize, Debug)]
struct Plan {
    itineraries: Vec<Itinerary>,
}

#[derive(Deserialize, Debug)]
struct ResponseData {
    plan: Plan,
}

#[derive(Deserialize, Debug)]
struct Response {
    data: ResponseData,
}

type AllItineraries = HashMap<String, HashMap<String, Itinerary>>;

fn get_all_itineraries(addresses: &Vec<Address>) -> Result<AllItineraries> {
    let mut itineraries: AllItineraries = HashMap::new();

    for from_address in addresses {
        for to_address in addresses {
            if from_address == to_address {
                continue;
            }

            let mut result = get_itineraries(from_address, to_address)?;
            result.sort_by_key(|itinerary| itinerary.duration as i32);

            if let Some(fastest) = result.first() {
                itineraries
                    .entry(from_address.name.clone())
                    .or_insert_with(HashMap::new)
                    .insert(to_address.name.clone(), fastest.clone());
            }
        }
    }

    std::fs::write(
        "./itineraries.json",
        serde_json::to_string_pretty(&itineraries)?,
    )?;

    dbg!("Wrote itineraries.json");

    Ok(itineraries)
}

fn get_itineraries(from: &Address, to: &Address) -> Result<Vec<Itinerary>> {
    println!("Getting itineraries from {} to {}", from.name, to.name);

    let query = format!(
        r#"
    {{
      plan(
        from: {{lat: {}, lon: {}}}
        to: {{lat: {}, lon: {}}}
        numItineraries: 5
      ) {{
        itineraries {{
          duration
          walkDistance
          legs {{
            startTime
            endTime
            mode
            duration
            realTime
            distance
            transitLeg
            trip {{
              route {{
                shortName
              }}
            }}
          }}
        }}
      }}
    }}
"#,
        from.lat, from.lon, to.lat, to.lon
    );

    let client = reqwest::blocking::Client::new();
    let response = client
        .request(
            Method::POST,
            "https://api.digitransit.fi/routing/v1/routers/hsl/index/graphql",
        )
        .header("Content-Type", "application/graphql")
        .header(
            "digitransit-subscription-key",
            std::env::var("DIGITRANSIT_PRIMARY_KEY")?,
        )
        .body(query)
        .send()?;

    let json = response.json::<Response>()?;

    Ok(json.data.plan.itineraries)
}
