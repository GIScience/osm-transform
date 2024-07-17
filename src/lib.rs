pub mod io;
pub mod area;
pub mod handler;
pub mod output;
pub mod osm_model;

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use benchmark_rs::stopwatch::StopWatch;
use log4rs::append::console::ConsoleAppender;
use log4rs::config::{Appender, Logger, Root};
use log::LevelFilter;
use regex::Regex;
use crate::io::process_with_handler;
use area::AreaHandler;
use crate::handler::{HandlerChain, ComplexElementsFilter, OsmElementTypeSelection, ElementCounter, CountType, HandlerResult, AllElementsFilter, ReferencedNodeIdCollector, NodeIdFilter, TagFilterByKey, FilterType, ElementPrinter, MetadataRemover};
use crate::output::OutputHandler;


pub fn init(config: &Config) {
    let log_level: LevelFilter;
    match config.debug {
        0 => log_level = LevelFilter::Info,
        1 => log_level = LevelFilter::Debug,
        2 => log_level = LevelFilter::Trace,
        _ => log_level = LevelFilter::Off,
    }
    let stdout = ConsoleAppender::builder().build();
    let config = log4rs::Config::builder()
        .appender(Appender::builder().build("stdout", Box::new(stdout)))
        .logger(Logger::builder().build("rusty_routes_transformer", log_level))
        .build(Root::builder().appender("stdout").build(LevelFilter::Off))
        .unwrap();
    let _handle = log4rs::init_config(config).unwrap();
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
        .add(ElementCounter::new(OsmElementTypeSelection::all(), CountType::ALL))
        .add(AllElementsFilter{handle_types: OsmElementTypeSelection::node_only()})
        .add(ComplexElementsFilter::ors_default())
        .add(ReferencedNodeIdCollector::default())
        .add(ElementCounter::new(OsmElementTypeSelection::all(), CountType::ACCEPTED))
        ;

    log::info!("Starting extraction of referenced node ids...");
    let mut stopwatch = StopWatch::new();
    stopwatch.start();
    let _ = process_with_handler(config, &mut handler_chain).expect("Extraction of referenced node ids failed");
    let mut handler_result = handler_chain.collect_result();
    log::info!("Finished extraction of referenced node ids, time: {}", stopwatch);
    log::info!("{}" , &handler_result.to_string());
    stopwatch.reset();
    handler_result
}

fn process(config: &Config, node_filter_result: Option<HandlerResult>) -> HandlerResult {
    let mut handler_chain = HandlerChain::default()
        .add(ElementPrinter::with_prefix("input:----------------\n".to_string())
            .with_node_ids(config.print_node_ids.clone())
            .with_way_ids(config.print_way_ids.clone())
            .with_relation_ids(config.print_relation_ids.clone()))
        .add(ElementCounter::new(OsmElementTypeSelection::all(), CountType::ALL));

    if config.remove_metadata {
        handler_chain = handler_chain.add(MetadataRemover::default())
    }

    handler_chain = handler_chain.add(ComplexElementsFilter::ors_default());

    match node_filter_result {
        None => {}
        Some(result) => {
            log::debug!("Cloning result node_ids with len={}", result.node_ids.len());
            let node_id_filter = NodeIdFilter { node_ids: result.node_ids.clone() };
            log::debug!("node_id_filter has node_ids with len={}", node_id_filter.node_ids.len());
            handler_chain = handler_chain.add(node_id_filter);
        }
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

    //todo add elevation processing

    handler_chain = handler_chain.add(TagFilterByKey::new(
        OsmElementTypeSelection::all(),
        Regex::new("(.*:)?source(:.*)?|(.*:)?note(:.*)?|url|created_by|fixme|wikipedia").unwrap(),
        FilterType::RemoveMatching));

    handler_chain = handler_chain.add(ElementCounter::new(OsmElementTypeSelection::all(), CountType::ACCEPTED));

    handler_chain = handler_chain.add(ElementPrinter::with_prefix("output:----------------\n".to_string())
        .with_node_ids(config.print_node_ids.clone())
        .with_way_ids(config.print_way_ids.clone())
        .with_relation_ids(config.print_relation_ids.clone()));

    match &config.output_pbf {
        Some(path_buf) => {
            log::info!("Initializing ouput handler");
            stopwatch.start();
            let mut output_handler = OutputHandler::new(path_buf.clone());
            output_handler.init();
            handler_chain = handler_chain.add(output_handler);
            stopwatch.reset();
        }
        None => {}
    }

    log::info!("Starting processing of pbf elements...");
    let mut stopwatch = StopWatch::new();
    stopwatch.start();
    let _ = process_with_handler(config, &mut handler_chain).expect("Processing failed");
    let mut processing_result = handler_chain.collect_result();
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
    pub with_node_filtering: bool,
    pub debug: u8,
    pub print_node_ids: HashSet<i64>,
    pub print_way_ids: HashSet<i64>,
    pub print_relation_ids: HashSet<i64>,
    pub remove_metadata: bool,
}
