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
use io::process_file;
use osm_io::osm::model::node::Node;
use osm_io::osm::model::relation::Relation;
use osm_io::osm::model::way::Way;
use crate::output::OutputHandler;

pub fn run(config: &Config) {

    let mut output_handler = OutputHandler::new(config);
    output_handler.init();
    let mut handler_chain = AreaHandler::new(output_handler);
    log::info!("Reading area mapping CSV");
    let mut stopwatch = StopWatch::new();
    stopwatch.start();
    handler_chain.load(&config).expect("Area handler failed to load CSV file");
    log::info!("Loaded: {} areas", handler_chain.mapping.id.len());
    log::info!("Finished reading area mapping, time: {}", stopwatch);
    stopwatch.reset();
    stopwatch.start();
    log::info!("Mapping nodes in PBF file");
    let _ = process_with_handler(config, &mut handler_chain).expect("Area handler failed");
    log::info!("Finished mapping, time: {}", stopwatch);

    // process_file().expect("did not work");

    // read pbf, filter node ids belonging to ways -> node_ids, extract bbox, maxId (gefilterte)
    // reader(config, filter, bbox_extracotr, max_id_extractor);

    // let mut bbox_collector = BboxCollector{next: None, min_lat: 0f64, min_lon: 0f64, max_lat: 0f64, max_lon: 0f64};
    // let mut filter = Filter{next: &bbox_collector, node_ids: Vec::new(), way_ids: Vec::new()};
    // process_with_handler(config, filter);

    // download geotiffs for bbox
    // geo_tiff_downloader(config, bbox_extractor);

    // read pbf, nodes: handle notes to keep
    //                      remove tags
    //                      if elevation: add ele tag
    //                      if interpolation & elevation: add node_id:coordinates
    //                      if country: add country tag
    //                      write node to node pbf file nodes1
    // reader(config, filter, remove_tags, elevation_handler, interpolation_handler, country_handler, output_handler)

    //            ways:
    //                    remove tags
    //                    if interpolation: interpolate: create new nodes an add to nodes1
    //                    write way to ways
    //             relations:
    //                  remove tags
    //                  write
    //  if interpolated : merge files
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hello() {}
}
