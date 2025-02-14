use std::collections::HashSet;
use std::path::PathBuf;

use clap::Parser;

use rusty_routes_transformer::{Config, init, run};
use rusty_routes_transformer::handler::HandlerResult;

fn main() {
    let args = Args::parse();
    let config = args.to_config();
    init(&config);
    let handler_result = run(&config);
    print_statistics(&config, handler_result);
}

fn print_statistics(config: &Config, handler_result: HandlerResult) {
    println!("{}", handler_result.statistics(&config));
}

/// Preprocessor to prepare OSM PBF-Files for openrouteservice
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// PBF file to preprocess.
    #[arg(short = 'i', long, value_name = "FILE")]
    pub(crate) input_pbf: PathBuf,

    /// Result PBF file. Will be overwritten if existing! If not specified, no output is written to a file. But can still be useful in combination with id-logging.
    #[arg(short = 'o', long, value_name = "FILE")]
    pub(crate) output_pbf: Option<PathBuf>,

    /// CSV File with border geometries for country mapping. If not specified, no area mapping is performed, no country tags added.
    #[arg(short = 'c', long, value_name = "FILE")]
    pub(crate) country_csv: Option<PathBuf>,

    /// Elevation GeoTiff Files (glob patterns allowed) to enrich nodes with elevation data.
    #[arg(short = 'e', long, value_name = "PATTERN", action = clap::ArgAction::Append)]
    pub(crate) elevation_tiffs: Vec<String>,

    /// Size of the elevation buffer for each elevation tiff file. This is the number of nodes that are buffered in memory before their elevation is read from the elevation tiff file in a batch.
    #[arg(short = 'b', long, default_value = "1000000")]
    pub elevation_batch_size: usize,

    /// Total number of nodes that are buffered in all elevation file buffers. When the number is reached, the largest buffer is flushed.
    #[arg(short = 'B', long, default_value = "50000000")]
    pub elevation_total_buffer_size: usize,

    /// Split ways at elevation changes.
    #[arg(long)]
    pub elevation_way_splitting: bool,

    /// Turn debugging information on.
    #[arg(short = 'd', long, action = clap::ArgAction::Count)]
    pub debug: u8,

    /// Print node with id=<ID> at beginning and end of processing pipeline.
    #[arg(short = 'n', long, value_name = "ID")]
    pub print_node_ids: Vec<i64>,

    /// Print way with id=<ID> at beginning and end of processing pipeline.
    #[arg(short = 'w', long, value_name = "ID")]
    pub print_way_ids: Vec<i64>,

    /// Print relation with id=<ID> at beginning and end of processing pipeline.
    #[arg(short = 'r', long, value_name = "ID")]
    pub print_relation_ids: Vec<i64>,

    /// Suppress node filtering. This means, that ALL nodes, ways, relations are handled by thy processing pass.
    #[arg(long)]
    pub suppress_node_filtering: bool,

    /// Do NOT remove metadata 'version', 'timestamp', 'changeset', 'uid', 'user'
    #[arg(long)]
    pub keep_metadata: bool,

    /// Resolution for way splitting in direction longitude
    #[arg(long, default_value = "0.001")]
    pub resolution_lon: f64,

    /// Resolution for way splitting in direction latitude
    #[arg(long, default_value = "0.001")]
    pub resolution_lat: f64,

    /// Elevation threshold defining when to introduce intermediate nodes
    #[arg(long, default_value = "10.0")]
    pub elevation_threshold: f64,
}
impl Args {
    pub fn to_config(self) -> Config {
        Config {
            input_pbf: self.input_pbf,
            country_csv: self.country_csv,
            output_pbf: self.output_pbf,
            elevation_tiffs: self.elevation_tiffs,
            elevation_batch_size: self.elevation_batch_size,
            elevation_total_buffer_size: self.elevation_total_buffer_size,
            elevation_way_splitting: self.elevation_way_splitting,
            debug: self.debug,
            with_node_filtering: ! self.suppress_node_filtering,
            print_node_ids: HashSet::from_iter(self.print_node_ids),
            print_way_ids: HashSet::from_iter(self.print_way_ids),
            print_relation_ids: HashSet::from_iter(self.print_relation_ids),
            remove_metadata: ! self.keep_metadata,
            resolution_lon: self.resolution_lon,
            resolution_lat: self.resolution_lat,
            elevation_threshold: self.elevation_threshold,
        }
    }
}
