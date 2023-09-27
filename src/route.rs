use std::sync::{
    atomic::{AtomicI32, Ordering},
    Arc, RwLock,
};

use atomic_float::AtomicF64;
use color_eyre::eyre::Result;
use rayon::prelude::{ParallelBridge, ParallelIterator};

use crate::api::itinerary::{AllItineraries, Itinerary, ItineraryLeg};

pub type Address = String;

#[derive(Clone, Debug)]
struct Solution {
    addresses: Vec<String>,
    itineraries: Vec<Itinerary>,
}

pub fn factorial(num: u128) -> u128 {
    (1..=num).product()
}

// Find shortest route that visits all addresses
pub fn find_shortest_route(addresses: &Vec<Address>, itineraries: &AllItineraries) -> Result<()> {
    use itertools::Itertools;

    let perms = addresses.iter().permutations(addresses.len());
    let shortest_distance = AtomicF64::new(std::f64::MAX);
    let shortest_duration = AtomicF64::new(std::f64::MAX);
    let shortest_duration_with_start = AtomicF64::new(std::f64::MAX);
    let shortest_solution: Arc<RwLock<Option<Solution>>> = Arc::new(RwLock::new(None));

    let i: AtomicI32 = AtomicI32::new(0);
    let num_perms = factorial(addresses.len() as u128);
    perms.par_bridge().for_each(|permutation| {
        let i = i.fetch_add(1, Ordering::Relaxed);

        if i % 100000 == 0 {
            println!(
                "Permutation {} of {} ({:.2}%)",
                i,
                num_perms,
                i as f64 / num_perms as f64 * 100.0
            );
        }

        let mut total_duration = 0.0;
        let mut start_duration = 0.0;
        let mut end_duration = 0.0;
        let mut total_distance = 0.0;
        let mut total_walk_distance = 0.0;
        let mut legs: Vec<ItineraryLeg> = Vec::new();

        // Add the duration from all addresses to the first address
        for address in permutation.iter() {
            if address == &permutation[0] {
                continue;
            }

            let itinerary = itineraries
                .get(*address)
                .unwrap()
                .get(permutation[0])
                .unwrap();

            start_duration += itinerary.duration;
        }

        // Add the duration from the last address to all other addresses
        for address in permutation.iter() {
            if address == &permutation[permutation.len() - 1] {
                continue;
            }

            let itinerary = itineraries
                .get(permutation[permutation.len() - 1])
                .unwrap()
                .get(*address)
                .unwrap();

            end_duration += itinerary.duration;
        }

        // Add up durations between all address pairs
        for (from, to) in permutation.iter().tuple_windows() {
            let itinerary = itineraries.get(*from).unwrap().get(*to).unwrap();

            total_duration += itinerary.duration;
            total_distance += itinerary.legs.iter().map(|leg| leg.distance).sum::<f64>();
            total_walk_distance += itinerary.walk_distance;
            legs.extend(itinerary.legs.clone());
        }

        // println!(
        //     "Total duration: {:.0} min, total distance: {:.1} km, total walk distance: {:.1} km",
        //     total_duration / 60.0,
        //     total_distance / 1000.0,
        //     total_walk_distance / 1000.0
        // );

        // println!("Legs: {:#?}", legs);

        if total_duration + start_duration + end_duration
            < shortest_duration_with_start.load(Ordering::SeqCst)
        {
            shortest_distance.store(total_distance, Ordering::SeqCst);
            shortest_duration.store(total_duration, Ordering::SeqCst);
            shortest_duration_with_start.store(
                total_duration + start_duration + end_duration,
                Ordering::SeqCst,
            );
            let mut shortest_solution = shortest_solution.write().unwrap();
            *shortest_solution = Some(Solution {
                addresses: permutation.iter().map(|a| (*a).clone()).collect(),
                itineraries: permutation
                    .iter()
                    .tuple_windows()
                    .map(|(from, to)| itineraries[*from][*to].clone())
                    .collect(),
            });
        }
    });

    println!(
        "Shortest distance: {:.1} km",
        shortest_distance.load(Ordering::SeqCst) / 1000.0
    );
    println!(
        "Shortest duration: {:.0} min",
        shortest_duration.load(Ordering::SeqCst) / 60.0
    );
    let shortest_solution = shortest_solution.read().unwrap();
    println!("Shortest solution: {:#?}", *shortest_solution);

    Ok(())
}
