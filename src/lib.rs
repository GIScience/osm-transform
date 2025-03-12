pub mod io;
pub mod area;
pub mod output;
pub mod srs;
pub mod handler;

#[macro_use]
extern crate maplit;

use std::sync::Once;
use std::collections::HashSet;
use std::fs;
use std::fs::File;
use std::path::PathBuf;
use benchmark_rs::stopwatch::StopWatch;
use glob::glob;
use log4rs::append::console::ConsoleAppender;
use log4rs::config::{Appender, Logger, Root};
use log::{debug, error, info, log_enabled, warn, LevelFilter};
use log::Level::Trace;
use regex::Regex;
use crate::io::process_with_handler;
use area::{AreaHandler, AreaMappingManager};
use ElementCountResultType::{AcceptedCount, InputCount, OutputCount};
use osm_io::osm::model::relation::{Member, MemberData, Relation};
use osm_io::osm::model::tag::Tag;
use osm_io::osm::model::element::Element;
use osm_io::osm::model::coordinate::Coordinate;
use osm_io::osm::model::node::Node;
use osm_io::osm::model::way::Way;
use crate::handler::{HandlerChain, HandlerData, OsmElementTypeSelection};
use crate::handler::collect::{IdCollector, MinMaxIdCollector, ReferencedNodeIdCollector};
use crate::handler::filter::{AllElementsFilter, ComplexElementsFilter, FilterType, IdFilter, TagFilterByKey};
use crate::handler::geotiff::{BufferingElevationEnricher, GeoTiffManager};
use crate::handler::info::{ElementCountResultType, ElementCounter, ElementPrinter};
use crate::handler::modify::MetadataRemover;
use crate::handler::skip_ele::SkipElevationNodeCollector;
use crate::handler::tests::MemberType;
use crate::output::{SimpleOutputHandler, SplittingOutputHandler};

// Initialize only once to prevent integration tests from trying
// to allocate the logger/console multiple times when run in 
// parallel.
static INIT: Once = Once::new();

const TAGS_TO_REMOVE: &str = "(.*:)?source(:.*)?|(.*:)?note(:.*)?|url|created_by|fixme|wikipedia";

pub fn init(config: &Config) {
    INIT.call_once(|| {
        let log_level: LevelFilter;
            log_level = config.get_log_level();
        let stdout = ConsoleAppender::builder()
            .encoder(Box::new(log4rs::encode::pattern::PatternEncoder::new("{d(%Y-%m-%d %H:%M:%S%.3f %Z)(utc)} - {l:5} - {m}{n}")))
            .build();
        let config = log4rs::Config::builder()
            .appender(Appender::builder().build("stdout", Box::new(stdout)))
            .logger(Logger::builder().build("rusty_routes_transformer", log_level))
            .build(Root::builder().appender("stdout").build(LevelFilter::Off))
            .unwrap();
        let _handle = log4rs::init_config(config).unwrap();
    });
}

pub fn validate(config: &Config) {
    validate_file(&config.input_pbf, "Input file");
    validate_file_writable(&config.output_pbf, "Output file");
    validate_country_tile_size(&config.country_tile_size);
    validate_country_data(config);
    validate_elevation_data(config);
}

fn validate_file_writable(path_buf: &Option<PathBuf>, label: &str) {
    match path_buf {
        None => {}
        Some(path_buf) => {
            if path_buf.exists() {
                panic!("{} already exists: {}", label, path_buf.display());
            } else {
                match File::create(path_buf) {
                    Ok(_) => {
                        fs::remove_file(path_buf).expect("Failed to remove test file");
                    }
                    Err(e) => {
                        panic!("{} {} cannot not be created: {}", label, path_buf.display(), e);
                    }
                }
            }
        }
    }
}

fn validate_elevation_data(config: &Config) {
    for files_pattern in config.elevation_tiffs.clone() {
        let mut counter = 0;
        match glob(files_pattern.as_str()) {
            Ok(paths) => {
                for entry in paths {
                    match entry {
                        Ok(path) => {
                            validate_file(&path, "Elevation GeoTiff file");
                            counter += 1;
                        }
                        Err(e) => error!("Error reading path: {:?}", e),
                    }
                }
            }
            Err(e) => error!("Failed to read glob pattern: {:?}", e),
        }
        if counter == 0 {
            std::panic!("No geotiff files found for glob pattern {}", files_pattern);
        }
    }
}

fn validate_country_data(config: &Config) {
    if ! config.should_enrich_country() {
        return;
    }
    if config.should_build_country_index() {
        match &config.country_data {
            Some(country_source) => {
                validate_file(country_source, "Country CSV file");
                let index_dir = AreaMappingManager::country().get_index_dir_name(country_source, config.country_tile_size);
                let path_buf = PathBuf::from(&index_dir);
                if path_buf.exists() {
                    panic!("Country index directory already exists: {}. Load this index (fast) or delete directory to re-generate index (slow)", index_dir);
                }
                match File::create(&path_buf) {
                    Ok(_) => {
                        fs::remove_file(&path_buf).expect("Failed to remove test directory");
                    }
                    Err(e) => {
                        panic!("Country index directory {} cannot be created: {}", &index_dir, e);
                    }
                }
            }
            None => {
                panic!("Country CSV file not specified");
            }
        }
    } else if config.should_load_country_index() {
        match &config.country_data {
            Some(country_source) => {
                AreaMappingManager::country().validate_index_files(country_source);
            }
            None => {
                panic!("Country index directory not specified");
            }
        }
    } else {
        panic!("No valid country data source specified");
    }
}

pub(crate) fn validate_file(path_buf: &PathBuf, label: &str) {
    if !path_buf.exists() {
        panic!("{} does not exist: {}", label, path_buf.display());
    }
    if !path_buf.is_file() {
        panic!("{} is not a file: {}", label,path_buf.display());
    }
    if path_buf.metadata().unwrap().len() == 0 {
        panic!("{} is empty: {}", label, path_buf.display());
    }
    if ! File::open(path_buf).is_ok() {
        panic!("{} could not be opened: {}", label, path_buf.display());
    }
    match path_buf.canonicalize() {
        Ok(absolute_path) => {
            match fs::metadata(&absolute_path) {
                Ok(metadata) => {
                    let file_size = metadata.len();
                    debug!("Found readable {}: {}, size: {} bytes", label.to_lowercase(), absolute_path.display(), file_size);
                }
                Err(e) => {
                    warn!("Failed to get metadata for {}: {}", label.to_lowercase(), e);
                }
            }
        }
        Err(e) => {
            warn!("Failed to get absolute path of {}: {}", label.to_lowercase(), e);
        }
    }
}

fn validate_country_tile_size(country_tile_size: &f64) {
    if country_tile_size <= &0.0 {
        panic!("Country tile size must be greater than 0.0");
    }
    if &180.0 % (1.0/country_tile_size) != 0.0 {
        panic!("Country tile size must be a divisor of 180.0");
    }
}

pub fn run(config: &Config) -> HandlerData {
    info!("{} v{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
    let mut stopwatch_total = StopWatch::new();
    stopwatch_total.start();
    let mut data = HandlerData::default();
    run_filter_chain(config, &mut data);
    run_processing_chain(config, &mut data);
    stopwatch_total.stop();
    data.total_processing_time = stopwatch_total.accumulated();
    data
}

fn run_filter_chain(config: &Config, data: &mut HandlerData){
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

    handler_chain = handler_chain.add(IdCollector{handle_types: OsmElementTypeSelection::way_relation()});

    let _ = process_with_handler(config, &mut handler_chain, data, info_msg).expect("Extraction of referenced node ids failed");
    handler_chain.close_handlers(data);
    info!("{} done, time: {}", info_msg, stopwatch_run_filter_chain);
    stopwatch_run_filter_chain.stop();
}

fn run_processing_chain(config: &Config, data: &mut HandlerData) {//TODO use bitvec filters also for ways and relations
    data.clear_non_input_counts();
    let mut handler_chain = HandlerChain::default()
        .add(ElementCounter::new(InputCount))
        .add(ElementPrinter::with_prefix("\ninput:----------------\n".to_string())
            .with_node_ids(config.print_node_ids.clone())
            .with_way_ids(config.print_way_ids.clone())
            .with_relation_ids(config.print_relation_ids.clone()));

    if config.with_node_filtering {
        handler_chain = handler_chain.add(IdFilter{handle_types: OsmElementTypeSelection::all()});
    } else {
        handler_chain = handler_chain.add(IdFilter{handle_types: OsmElementTypeSelection::way_relation()});
    }

    if config.remove_metadata {
        handler_chain = handler_chain.add(MetadataRemover::default())
    }

    if config.get_summary_level() > 2 {
        handler_chain = handler_chain.add(MinMaxIdCollector::new(OsmElementTypeSelection::all()));
    }
    handler_chain = handler_chain.add(ElementCounter::new(AcceptedCount));//todo needed? let writers count?

    let mut stopwatch = StopWatch::new();

    if config.should_enrich_elevation() {
        stopwatch.start();
        info!("Creating spatial elevation index...");
        let geotiff_manager: GeoTiffManager = GeoTiffManager::with_file_patterns(config.elevation_tiffs.clone());
        let elevation_enricher = BufferingElevationEnricher::new(
            geotiff_manager,
            config.elevation_batch_size,
            config.elevation_total_buffer_size,
            data.no_elevation_node_ids.clone(),//todo avoid cloning - pass HandlerData to fn where needed
            config.elevation_way_splitting,
            config.keep_original_elevation,
            config.resolution_lon,
            config.resolution_lat,
            config.elevation_threshold);
        if log_enabled!(Trace) {
            handler_chain = handler_chain.add(ElementPrinter::with_prefix(" before BufferingElevationEnricher:----------------\n".to_string())
                .with_node_ids(config.print_node_ids.clone())
                .with_way_ids(config.print_way_ids.clone())
                .with_relation_ids(config.print_relation_ids.clone()));
        }
        handler_chain = handler_chain.add(elevation_enricher);
        if log_enabled!(Trace) {
            handler_chain = handler_chain.add(ElementPrinter::with_prefix(" after BufferingElevationEnricher:----------------\n".to_string())
                .with_node_ids(config.print_node_ids.clone())
                .with_way_ids(config.print_way_ids.clone())
                .with_relation_ids(config.print_relation_ids.clone()));
        }
        info!("Creating spatial elevation index done, time: {}", stopwatch);
        stopwatch.reset();
    }

    if config.should_enrich_country() {
        if config.should_load_country_index() {
            info!("Loading spatial country index...");
            stopwatch.start();
            let mapping = AreaMappingManager::country().load_index(&config.country_data).expect("Failed to load index");
            let area_handler = AreaHandler::new(mapping);
            info!("Loading spatial country index done, time: {}", stopwatch);
            stopwatch.reset();
            handler_chain = handler_chain.add(area_handler);
        } else if config.should_build_country_index() {
            info!("Creating spatial country index with country-tile-size={}...", config.country_tile_size);
            stopwatch.start();
            let mapping = AreaMappingManager::country().build_index(&config.country_data.clone().unwrap(), config.country_tile_size).expect("Area handler failed to load CSV file");
            let area_handler = AreaHandler::new(mapping);
            info!("Creating spatial country index for {} countries and country-tile-size={} done, time: {}", area_handler.mapping.id.len(), config.country_tile_size, stopwatch);
            info!("The country index files were saved to {} and can be loaded in subsequent runs to reduce processing time",
                AreaMappingManager::country().get_index_dir_name(&config.country_data.clone().unwrap(), config.country_tile_size));
            stopwatch.reset();
            handler_chain = handler_chain.add(area_handler);
        } else {
            panic!("No valid country data source specified");
        }
    }

    handler_chain = handler_chain.add(TagFilterByKey::new(
        OsmElementTypeSelection::all(),
        Regex::new(TAGS_TO_REMOVE).unwrap(),
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
    let _ = process_with_handler(config, &mut handler_chain, data, info_msg).expect("2.Pass: Processing pbf elements failed");
    handler_chain.close_handlers(data);
    info!("{} done, time: {}", info_msg, stopwatch);
    debug!("{} HandlerData:\n{}", info_msg, data.format_multi_line());
    stopwatch.reset();
}

#[derive(Debug, Default)]
pub struct Config {
    pub input_pbf: PathBuf,
    pub output_pbf: Option<PathBuf>,

    pub with_node_filtering: bool,
    pub remove_metadata: bool,

    pub country_data: Option<PathBuf>,
    pub country_tile_size: f64,

    pub elevation_tiffs: Vec<String>,
    pub elevation_batch_size: usize,
    pub elevation_total_buffer_size: usize,
    pub elevation_way_splitting: bool,
    pub elevation_threshold: f64,
    pub resolution_lon: f64,
    pub resolution_lat: f64,
    pub keep_original_elevation: bool,

    pub print_relation_ids: HashSet<i64>,
    pub print_node_ids: HashSet<i64>,
    pub print_way_ids: HashSet<i64>,

    pub verbosity: u8,
    pub loglevel: u8,
    pub quiet: bool,
}

impl Config {
    pub fn should_enrich_elevation(&self) -> bool {
        self.elevation_tiffs.len() > 0
    }
    pub fn should_enrich_country(&self) -> bool {
        self.country_data.is_some()
    }
    pub fn should_load_country_index(&self) -> bool {
        match &self.country_data {
            None => {false}
            Some(cd) => {cd.is_dir()}
        }
    }
    pub fn should_build_country_index(&self) -> bool {
        match &self.country_data {
            None => {false}
            Some(cd) => {cd.is_file()}
        }
    }
    pub fn get_summary_level(&self) -> u8 {
        if self.quiet {
            return 0;
        }
        self.verbosity + 1
    }
    pub fn get_log_level(&self) -> LevelFilter {
        if self.quiet {
            return LevelFilter::Off;
        }
        if self.loglevel == 13 {
            return LevelFilter::Trace;
        }
        if self.loglevel > 2 {
            return LevelFilter::Debug;
        }
        LevelFilter::Info
    }


}

#[cfg(test)]
pub(crate) mod test {
    use osm_io::osm::model::coordinate::Coordinate;
    use osm_io::osm::model::element::Element;
    use osm_io::osm::model::node::Node;
    use osm_io::osm::model::relation::{Member, MemberData, Relation};
    use osm_io::osm::model::tag::Tag;
    use osm_io::osm::model::way::Way;
    use crate::handler::HandlerData;
    use crate::handler::tests::MemberType;

    pub(crate) fn simple_node_element(id: i64, tags: Vec<(&str, &str)>) -> Element {
        let tags_obj = tags.iter().map(|(k, v)| Tag::new(k.to_string(), v.to_string())).collect();
        node_element(id, 1, Coordinate::new(1.0f64, 1.1f64), 1, 1, 1, "a".to_string(), true, tags_obj)
    }

    pub(crate) fn simple_node(id: i64, tags: Vec<(&str, &str)>) -> Node {
        let tags_obj = tags.iter().map(|(k, v)| Tag::new(k.to_string(), v.to_string())).collect();
        Node::new(id, 1, Coordinate::new(1.0f64, 1.1f64), 1, 1, 1, "a".to_string(), true, tags_obj)
    }

    pub(crate) fn simple_way_element(id: i64, refs: Vec<i64>, tags: Vec<(&str, &str)>) -> Element {
        let tags_obj = tags.iter().map(|(k, v)| Tag::new(k.to_string(), v.to_string())).collect();
        way_element(id, 1, 1, 1, 1, "a_user".to_string(), true, refs, tags_obj)
    }

    pub(crate) fn simple_way(id: i64, refs: Vec<i64>, tags: Vec<(&str, &str)>) -> Way {
        let tags_obj = tags.iter().map(|&(k, v)| Tag::new(String::from(k), String::from(v))).collect();
        Way::new(id, 1, 1, 1, 1, String::from("a_user"), true, refs, tags_obj)
    }

    pub(crate) fn simple_relation_element(id: i64, members: Vec<(MemberType, i64, &str)>, tags: Vec<(&str, &str)>) -> Element {
        Element::Relation { relation: simple_relation(id, members, tags) }
    }

    pub(crate) fn simple_relation(id: i64, members: Vec<(MemberType, i64, &str)>, tags: Vec<(&str, &str)>) -> Relation {
        let members_obj = members.iter().map(|(t, id, role)| {
            match t {
                MemberType::Node => { Member::Node { member: MemberData::new(id.clone(), role.to_string()) } }
                MemberType::Way => { Member::Way { member: MemberData::new(id.clone(), role.to_string()) } }
                MemberType::Relation => { Member::Relation { member: MemberData::new(id.clone(), role.to_string()) } }
            }
        }).collect();
        let tags_obj = tags.iter().map(|(k, v)| Tag::new(k.to_string(), v.to_string())).collect();
        Relation::new(id, 1, 1, 1, 1, "a_user".to_string(), true, members_obj, tags_obj)
    }

    pub(crate) fn node_element(id: i64, version: i32, coordinate: Coordinate, timestamp: i64, changeset: i64, uid: i32, user: String, visible: bool, tags: Vec<Tag>) -> Element {
        Element::Node { node: Node::new(id, version, coordinate, timestamp, changeset, uid, user, visible, tags) }
    }

    pub(crate) fn way_element(id: i64, version: i32, timestamp: i64, changeset: i64, uid: i32, user: String, visible: bool, refs: Vec<i64>, tags: Vec<Tag>) -> Element {
        Element::Way { way: Way::new(id, version, timestamp, changeset, uid, user, visible, refs, tags) }
    }

    pub(crate) fn as_node_element(node: Node) -> Element {
        Element::Node { node: node }
    }

    pub(crate) fn as_way_element(way: Way) -> Element {
        Element::Way { way: way }
    }
    pub(crate) fn default_handler_data_with_nodes(nodes: Vec<Node>) -> HandlerData {
        let mut handler_data = HandlerData::default();
        handler_data.nodes = nodes;
        handler_data
    }
}
