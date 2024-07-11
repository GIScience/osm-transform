pub mod conf;
pub mod io;
pub mod area;
pub mod handler;
pub mod output;
pub mod osm_model;

use std::process::exit;
use benchmark_rs::stopwatch::StopWatch;
use log::LevelFilter;
use crate::io::process_with_handler;
use conf::Config;
use area::AreaHandler;
use osm_io::osm::model::node::Node;
use osm_io::osm::model::relation::Relation;
use osm_io::osm::model::way::Way;
use crate::handler::{CountType, ElementCounter, HandlerChain, OsmElementTypeSelection};
use crate::output::OutputHandler;

pub fn run(config: &Config) {

    log::info!("Initializing ouput handler");
    let mut output_handler = OutputHandler::new(config);
    output_handler.init();

    log::info!("Reading area mapping CSV");
    let mut area_handler = AreaHandler::default();
    let mut stopwatch = StopWatch::new();
    stopwatch.start();
    area_handler.load(&config).expect("Area handler failed to load CSV file");
    log::info!("Loaded: {} areas", area_handler.mapping.id.len());
    log::info!("Finished reading area mapping, time: {}", stopwatch);
    stopwatch.reset();
    stopwatch.start();

    let mut handler_chain = HandlerChain::default().add_unboxed(area_handler).add_unboxed(output_handler);
    let _ = process_with_handler(config, &mut handler_chain).expect("Area handler failed");
    log::info!("Finished mapping, time: {}", stopwatch);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hello() {}
}
