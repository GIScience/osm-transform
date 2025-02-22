pub mod io;
pub mod area;
pub mod output;
pub mod srs;
pub mod handler;

#[macro_use]
extern crate maplit;

use std::sync::Once;
use std::collections::HashSet;
use std::path::PathBuf;
use benchmark_rs::stopwatch::StopWatch;
use log4rs::append::console::ConsoleAppender;
use log4rs::config::{Appender, Logger, Root};
use log::{info, debug, trace, LevelFilter, error, log_enabled};
use osm_io::osm::pbf;
use regex::Regex;
use crate::io::process_with_handler;
use area::AreaHandler;
use ElementCountResultType::{AcceptedCount, InputCount, OutputCount};
use crate::handler::{HandlerChain, HandlerResult, OsmElementTypeSelection};
use crate::handler::collect::ReferencedNodeIdCollector;
use crate::handler::filter::{AllElementsFilter, ComplexElementsFilter, FilterType, NodeIdFilter, TagFilterByKey};
use crate::handler::geotiff::{BufferingElevationEnricher, GeoTiffManager};
use crate::handler::info::{ElementCountResultType, ElementCounter, ElementPrinter};
use crate::handler::modify::MetadataRemover;
use crate::handler::skip_ele::SkipElevationNodeCollector;
use crate::output::{SimpleOutputHandler, SplittingOutputHandler};

// Initialize only once to prevent integration tests from trying
// to allocate the logger/console multiple times when run in 
// parallel.
static INIT: Once = Once::new();

pub fn init(config: &Config) {
    INIT.call_once(|| {
        let log_level: LevelFilter;
        match config.debug {
            0 => log_level = LevelFilter::Off,
            1 => log_level = LevelFilter::Info,
            2 => log_level = LevelFilter::Debug,
            _ => log_level = LevelFilter::Trace,
        }
        let stdout = ConsoleAppender::builder().build();
        let config = log4rs::Config::builder()
            .appender(Appender::builder().build("stdout", Box::new(stdout)))
            .logger(Logger::builder().build("rusty_routes_transformer", log_level))
            .build(Root::builder().appender("stdout").build(LevelFilter::Off))
            .unwrap();
        let _handle = log4rs::init_config(config).unwrap();
    });
}

pub fn run(config: &Config) -> HandlerResult {
    let mut stopwatch_total = StopWatch::new();
    stopwatch_total.start();

    let mut result = HandlerResult::default();
    count_elements(config, &mut result);//todo add cli option for this - only really needed for very large files
    run_filter_chain(config, &mut result);
    run_processing_chain(config, &mut result);

    info!("Total processing time: {}", stopwatch_total);
    stopwatch_total.stop();
    result
}
fn count_elements(config: &Config, result: &mut HandlerResult) {
    info!("Counting pbf elements...");
    let mut stopwatch = StopWatch::new();
    stopwatch.start();
    let reader = pbf::reader::Reader::new(&config.input_pbf);
    match reader {
        Ok(reader) => {
            let nwr_counts = reader.count_objects();
            match nwr_counts {
                Ok(counts) => {
                    info!("Counting pbf elements done: {:?} nodes, ways, relations, time: {}", &counts, stopwatch);
                    result.input_node_count = counts.0 as u64;
                    result.input_way_count = counts.1 as u64;
                    result.input_relation_count = counts.2 as u64;
                }
                Err(e) => {
                    error!("Failed to count pbf elements: {}", e);
                    panic!("Failed to count pbf elements: {}", e);
                }
            }
        }
        Err(e) => {
            error!("Failed to read pbf file: {}", e);
            panic!("Failed to read pbf file: {}", e);
        }
    }
}

fn run_filter_chain(config: &Config, result: &mut HandlerResult){
    let mut stopwatch_run_filter_chain = StopWatch::new();
    stopwatch_run_filter_chain.start();
    let info_msg = "1.Pass: Filtering pbf elements";
    info!("{}...", info_msg);
    let mut handler_chain = HandlerChain::default()
        .add(ElementCounter::new(InputCount))//todo needed? let io count?
        .add(AllElementsFilter{handle_types: OsmElementTypeSelection::node_only()})
        .add(ComplexElementsFilter::ors_default())
        .add(ElementCounter::new(AcceptedCount));

    if config.with_node_filtering {
        handler_chain = handler_chain.add(ReferencedNodeIdCollector::default());
    }

    if &config.elevation_tiffs.len() > &0 {
        handler_chain = handler_chain.add(SkipElevationNodeCollector::default())
    }
    //todo add IdCollector{handle_types: OsmElementTypeSelection::way_relation()}
    let _ = process_with_handler(config, &mut handler_chain, result, info_msg).expect("Extraction of referenced node ids failed");
    handler_chain.collect_result(result);
    info!("{} done, time: {}", info_msg, stopwatch_run_filter_chain);
    stopwatch_run_filter_chain.stop();
}

fn run_processing_chain(config: &Config, result: &mut HandlerResult) {//TODO use bitvec filters also for ways and relations
    result.clear_non_input_counts();
    let mut handler_chain = HandlerChain::default()
        .add(ElementCounter::new(InputCount))
        .add(ElementPrinter::with_prefix("\ninput:----------------\n".to_string())
            .with_node_ids(config.print_node_ids.clone())
            .with_way_ids(config.print_way_ids.clone())
            .with_relation_ids(config.print_relation_ids.clone()));

    if config.remove_metadata {
        handler_chain = handler_chain.add(MetadataRemover::default())
    }

    handler_chain = handler_chain.add(ComplexElementsFilter::ors_default());//todo remove when bitvec filters are used

    if config.with_node_filtering {//todo move up before metadata remover
        handler_chain = handler_chain.add(NodeIdFilter { });
    }

    handler_chain = handler_chain.add(ElementCounter::new(AcceptedCount));

    let mut stopwatch = StopWatch::new();
    match &config.country_csv {
        Some(path_buf) => {
            info!("Creating spatial country index...");
            stopwatch.start();
            let mut area_handler = AreaHandler::default();
            area_handler.load(path_buf.clone()).expect("Area handler failed to load CSV file");
            debug!("Loaded: {} areas", area_handler.mapping.id.len());
            info!("Creating spatial country index done, time: {}", stopwatch);
            stopwatch.reset();

            handler_chain = handler_chain.add(area_handler);
        }
        None => (),
    }

    if &config.elevation_tiffs.len() > &0 {
        stopwatch.start();
        info!("Creating spatial elevation index...");
        let geotiff_manager: GeoTiffManager = GeoTiffManager::with_file_patterns(config.elevation_tiffs.clone());
        let elevation_enricher = BufferingElevationEnricher::new(
            geotiff_manager,
            config.elevation_batch_size,
            config.elevation_total_buffer_size,
            result.skip_ele.clone(),
            config.elevation_way_splitting,
            config.resolution_lon,
            config.resolution_lat,
            config.elevation_threshold);
        if log_enabled!(log::Level::Trace) {
            handler_chain = handler_chain.add(ElementPrinter::with_prefix(" before BufferingElevationEnricher:----------------\n".to_string())
                .with_node_ids(config.print_node_ids.clone())
                .with_way_ids(config.print_way_ids.clone())
                .with_relation_ids(config.print_relation_ids.clone()));
        }
        handler_chain = handler_chain.add(elevation_enricher);
        if log_enabled!(log::Level::Trace) {
            handler_chain = handler_chain.add(ElementPrinter::with_prefix(" after BufferingElevationEnricher:----------------\n".to_string())
                .with_node_ids(config.print_node_ids.clone())
                .with_way_ids(config.print_way_ids.clone())
                .with_relation_ids(config.print_relation_ids.clone()));
        }
        info!("Creating spatial elevation index done, time: {}", stopwatch);
        stopwatch.reset();
    }

    handler_chain = handler_chain.add(TagFilterByKey::new(
        OsmElementTypeSelection::all(),
        Regex::new("(.*:)?source(:.*)?|(.*:)?note(:.*)?|url|created_by|fixme|wikipedia").unwrap(),
        FilterType::RemoveMatching));

    handler_chain = handler_chain.add(ElementCounter::new(OutputCount));

    handler_chain = handler_chain.add(ElementPrinter::with_prefix("\noutput:----------------\n".to_string())
        .with_node_ids(config.print_node_ids.clone())
        .with_way_ids(config.print_way_ids.clone())
        .with_relation_ids(config.print_relation_ids.clone()));

    match &config.output_pbf {
        Some(path_buf) => {
            if config.elevation_way_splitting == true {
                let mut output_handler = SplittingOutputHandler::new(path_buf.clone());
                output_handler.init();
                handler_chain = handler_chain.add(output_handler);
            } else {
                let mut output_handler = SimpleOutputHandler::new(path_buf.clone());
                output_handler.init();
                handler_chain = handler_chain.add(output_handler);
            }
        }
        None => {}
    }

    let info_msg = "2.Pass: Processing pbf elements";
    info!("{}...", info_msg);
    stopwatch.reset();
    stopwatch.start();
    let _ = process_with_handler(config, &mut handler_chain, result, info_msg).expect("2.Pass: Processing pbf elements failed");
    handler_chain.collect_result(result);
    info!("{} done, time: {}", info_msg, stopwatch);
    debug!("{} HandlerResult:\n{}", info_msg, result.format_multi_line());
    stopwatch.reset();
}

#[derive(Debug, Default)]
pub struct Config {
    pub input_pbf: PathBuf,
    pub country_csv: Option<PathBuf>,
    pub output_pbf: Option<PathBuf>,
    pub elevation_tiffs: Vec<String>,
    pub with_node_filtering: bool,
    pub debug: u8,
    pub print_node_ids: HashSet<i64>,
    pub print_way_ids: HashSet<i64>,
    pub print_relation_ids: HashSet<i64>,
    pub remove_metadata: bool,
    pub elevation_batch_size: usize,
    pub elevation_total_buffer_size: usize,
    pub elevation_way_splitting: bool,
    pub resolution_lon: f64,
    pub resolution_lat: f64,
    pub elevation_threshold: f64,
    pub statistics_level: u8,
}
