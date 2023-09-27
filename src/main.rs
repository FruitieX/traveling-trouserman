use api::itinerary::get_all_itineraries;
use color_eyre::eyre::Result;
use dotenv::dotenv;
use route::{find_shortest_route, Address};

mod api;
mod route;

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
