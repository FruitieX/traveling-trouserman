use std::collections::HashMap;

use color_eyre::eyre::Result;
use eyre::eyre;
use reqwest::Method;
use serde::{Deserialize, Serialize};

use crate::{api::coords::get_coords, route::Address};

use super::coords::Coords;

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
pub struct ItineraryLeg {
    mode: String,
    duration: f64,
    pub distance: f64,
    trip: Option<Trip>,
}

#[derive(Clone, Deserialize, Debug, Serialize)]
pub struct Itinerary {
    pub legs: Vec<ItineraryLeg>,
    pub duration: f64,
    #[serde(rename = "walkDistance")]
    pub walk_distance: f64,
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

pub fn get_itineraries(
    from: &Address,
    to: &Address,
    coords: &HashMap<Address, Coords>,
) -> Result<Vec<Itinerary>> {
    println!("Getting itineraries from {} to {}", from, to);

    let from_coords = coords
        .get(from)
        .ok_or_else(|| eyre!("Could not find coords for {}", from))?;
    let to_coords = coords
        .get(to)
        .ok_or_else(|| eyre!("Could not find coords for {}", to))?;

    let query = format!(
        r#"
    {{
      plan(
        from: {{lat: {}, lon: {}}}
        to: {{lat: {}, lon: {}}}
        numItineraries: 5
        date: "2023-10-07"
        time: "12:00:00"
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
        from_coords.lat, from_coords.lon, to_coords.lat, to_coords.lon
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

pub type AllItineraries = HashMap<String, HashMap<String, Itinerary>>;

pub fn get_all_itineraries(addresses: &Vec<Address>) -> Result<AllItineraries> {
    let mut itineraries: AllItineraries = HashMap::new();

    let coords = addresses
        .iter()
        .map(|address| (address.clone(), get_coords(address).unwrap()))
        .collect();

    dbg!(&coords);

    for from_address in addresses {
        for to_address in addresses {
            if from_address == to_address {
                continue;
            }

            let mut result = get_itineraries(from_address, to_address, &coords)?;
            result.sort_by_key(|itinerary| itinerary.duration as i32);

            if let Some(fastest) = result.first() {
                itineraries
                    .entry(from_address.clone())
                    .or_insert_with(HashMap::new)
                    .insert(to_address.clone(), fastest.clone());
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
