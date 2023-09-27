use color_eyre::eyre::Result;
use eyre::eyre;
use reqwest::Method;
use serde::{Deserialize, Serialize};

use crate::route::Address;

#[derive(Clone, Deserialize, Debug, Serialize, PartialEq)]
pub struct Coords {
    pub lat: f64,
    pub lon: f64,
}

#[derive(Clone, Deserialize, Debug)]
struct Geometry {
    coordinates: (f64, f64),
}

#[derive(Clone, Deserialize, Debug)]
struct Feature {
    geometry: Geometry,
}

#[derive(Clone, Deserialize, Debug)]
struct Response {
    features: Vec<Feature>,

}

pub fn get_coords(addr: &Address) -> Result<Coords> {
    println!("Getting coordinates for {}", addr);

    let client = reqwest::blocking::Client::new();
    let response = client
        .request(
            Method::GET,
            format!("https://api.digitransit.fi/geocoding/v1/search?text={}&boundary.circle.lat=60.2&boundary.circle.lon=24.936&boundary.circle.radius=30&size=1", urlencoding::encode(addr))
        )
        .header(
            "digitransit-subscription-key",
            std::env::var("DIGITRANSIT_PRIMARY_KEY")?,
        )
        .send()?;

    let json = response.json::<Response>()?;
    let first_feature = json
        .features
        .first()
        .ok_or_else(|| eyre!("Result contained no features"))?;
    let coords = first_feature.geometry.coordinates;

    Ok(Coords {
        lat: coords.1,
        lon: coords.0,
    })
}
