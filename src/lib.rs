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
use log::LevelFilter;
use regex::Regex;
use crate::io::process_with_handler;
use area::AreaHandler;
use crate::handler::{HandlerChain, HandlerResult, OsmElementTypeSelection};
use crate::handler::collect::ReferencedNodeIdCollector;
use crate::handler::filter::{AllElementsFilter, ComplexElementsFilter, FilterType, NodeIdFilter, TagFilterByKey};
use crate::handler::geotiff::BufferingElevationEnricher;
use crate::handler::info::{ElementCounter, ElementPrinter};
use crate::handler::modify::MetadataRemover;

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

pub fn run(config: &Config) -> HandlerResult{
    let mut result: Option<HandlerResult> = None;
    if config.with_node_filtering {
        result = Some(extract_referenced_nodes(config));
    }
    result = Some(process(config, result));
    result.unwrap()
}
fn extract_referenced_nodes(config: &Config) -> HandlerResult {
    let mut handler_chain = HandlerChain::default()
        .add(ElementCounter::new("initial"))
        .add(AllElementsFilter{handle_types: OsmElementTypeSelection::node_only()})
        .add(ComplexElementsFilter::ors_default())
        .add(ReferencedNodeIdCollector::default())
        .add(ElementCounter::new("final"))
        ;

    log::info!("Starting extraction of referenced node ids...");
    let mut stopwatch = StopWatch::new();
    stopwatch.start();
    let _ = process_with_handler(config, &mut handler_chain).expect("Extraction of referenced node ids failed");
    let handler_result = handler_chain.collect_result();

    log::info!("Finished extraction of referenced node ids, time: {}", stopwatch);
    if log::log_enabled!(log::Level::Trace)  {
        log::trace!("Generating node-id statistics...");
        log::trace!("{}" , &handler_result.to_string_with_node_ids());
    }
    else {
        log::info!("{}" , &handler_result.to_string());
    }
    stopwatch.reset();
    handler_result
}

fn process(config: &Config, node_filter_result: Option<HandlerResult>) -> HandlerResult {
    let mut node_ids = None;
    let mut skip_ele = None;

    match node_filter_result {
        None => {}
        Some(result) => {
            node_ids = Some(result.node_ids);
            skip_ele = Some(result.skip_ele);
        }
    }

    let mut handler_chain = HandlerChain::default()
        .add(ElementPrinter::with_prefix("\ninput:----------------\n".to_string())
            .with_node_ids(config.print_node_ids.clone())
            .with_way_ids(config.print_way_ids.clone())
            .with_relation_ids(config.print_relation_ids.clone()))
        .add(ElementCounter::new("initial"));

    if config.remove_metadata {
        handler_chain = handler_chain.add(MetadataRemover::default())
    }

    handler_chain = handler_chain.add(ComplexElementsFilter::ors_default());

    if node_ids.is_some() {
        let node_id_filter = NodeIdFilter { node_ids: node_ids.unwrap() };
        log::debug!("node_id_filter has node_ids with len={}", node_id_filter.node_ids.len());
        handler_chain = handler_chain.add(node_id_filter);
    }

    let mut stopwatch = StopWatch::new();
    match &config.country_csv {
        Some(path_buf) => {
            log::info!("Reading area mapping CSV");
            stopwatch.start();
            let mut area_handler = AreaHandler::default();
            area_handler.load(path_buf.clone()).expect("Area handler failed to load CSV file");
            log::info!("Loaded: {} areas", area_handler.mapping.id.len());
            log::info!("Finished reading area mapping, time: {}", stopwatch);
            handler_chain = handler_chain.add(area_handler);
            stopwatch.reset();
        }
        None => ()
    }

    if &config.elevation_tiffs.len() > &0 {
        log::info!("Initializing elevation enricher");
        stopwatch.start();
        let mut elevation_enricher = BufferingElevationEnricher::new(config.elevation_batch_size, config.elevation_total_buffer_size, skip_ele);
        for elevation_glob_pattern in &config.elevation_tiffs {
            let _ = elevation_enricher.init(elevation_glob_pattern);
        }
        log::info!("Finished initializing elevation enricher, time: {}", stopwatch);
        handler_chain = handler_chain.add(elevation_enricher);
        stopwatch.reset();
    }

    handler_chain = handler_chain.add(TagFilterByKey::new(
        OsmElementTypeSelection::all(),
        Regex::new("(.*:)?source(:.*)?|(.*:)?note(:.*)?|url|created_by|fixme|wikipedia").unwrap(),
        FilterType::RemoveMatching));

    handler_chain = handler_chain.add(ElementCounter::new("final"));

    handler_chain = handler_chain.add(ElementPrinter::with_prefix("\noutput:----------------\n".to_string())
        .with_node_ids(config.print_node_ids.clone())
        .with_way_ids(config.print_way_ids.clone())
        .with_relation_ids(config.print_relation_ids.clone()));


    match &config.output_pbf {
        Some(path_buf) => {
            log::info!("Initializing ouput handler");
            stopwatch.start();
            if config.elevation_way_splitting {
                let mut output_handler = SplittingOutputHandler::new(path_buf.clone());
                output_handler.init();
                handler_chain = handler_chain.add(output_handler);
            } else {
                let mut output_handler = SimpleOutputHandler::new(path_buf.clone());
                output_handler.init();
                handler_chain = handler_chain.add(output_handler);
            }
            stopwatch.reset();
        }
        None => {}
    }

    log::info!("Starting processing of pbf elements...");
    let mut stopwatch = StopWatch::new();
    stopwatch.start();
    let _ = process_with_handler(config, &mut handler_chain).expect("Processing failed");
    let processing_result = handler_chain.collect_result();
    log::info!("Finished processing of pbf elements, time: {}", stopwatch);
    log::info!("{}" , &processing_result.to_string());
    stopwatch.reset();
    processing_result
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
}
