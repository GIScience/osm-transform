use std::collections::HashSet;
use std::path::PathBuf;

use clap::Parser;

use rusty_routes_transformer::{Config, init, run, validate};
use rusty_routes_transformer::handler::HandlerData;

fn main() {
    let args = Args::parse();
    let config = args.to_config();
    init(&config);
    validate(&config);
    let handler_data = run(&config);
    print_statistics(&config, handler_data);
}

fn print_statistics(config: &Config, handler_data: HandlerData) {
    if config.statistics_level > 0 {
        println!("{}", handler_data.summary(&config));
    }
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

    /// Base name of country index consisting of files <basename>.index.csv, <basename>.index.csv, <basename>.area.csv and <basename>.name.csv.
    /// If specified, the pre-computed country index is loaded, --country_csv and --country_tile_size are ignored.
    /// If neither country_index nor country_csv are specified, no area mapping is performed, no country tags added.
    #[arg(short = 'c', long, value_name = "FILE")]
    pub(crate) country_index: Option<String>,

    /// CSV File with border geometries for country mapping.
    /// If neither country_index nor country_csv are specified, no area mapping is performed, no country tags added.
    #[arg(short = 'C', long, value_name = "FILE")]
    pub(crate) country_csv: Option<PathBuf>,

    /// Tile size for the country index grid
    #[arg(long, default_value = "1.0")]
    pub(crate) country_tile_size: f64,

    /// Elevation GeoTiff Files (glob patterns allowed) to enrich nodes with elevation data.
    #[arg(short = 'e', long, value_name = "PATTERN", num_args = 1..)]
    pub(crate) elevation_tiffs: Vec<String>,

    /// Size of the elevation buffer for each elevation tiff file. This is the number of nodes that are buffered in memory before their elevation is read from the elevation tiff file in a batch.
    #[arg(short = 'b', long, default_value = "1000000")]
    pub elevation_batch_size: usize,

    /// Total number of nodes that are buffered in all elevation file buffers. When the number is reached, the largest buffer is flushed.
    #[arg(short = 'B', long, default_value = "50000000")]
    pub elevation_total_buffer_size: usize,

    /// Split ways at elevation changes.
    #[arg(short = 's', long)]
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

    /// Do NOT overwrite original elevation data with the value from an elevation tiff.
    #[arg(long)]
    pub keep_original_elevation: bool,

    /// Resolution for way splitting in direction longitude
    #[arg(long, default_value = "0.001")]
    pub resolution_lon: f64,

    /// Resolution for way splitting in direction latitude
    #[arg(long, default_value = "0.001")]
    pub resolution_lat: f64,

    /// Elevation threshold defining when to introduce intermediate nodes
    #[arg(long, default_value = "10.0")]
    pub elevation_threshold: f64,

    /// Print statistics. Can be added multiple times to increase verbosity.
    #[arg(short = 'S', long, action = clap::ArgAction::Count)]
    pub stat: u8,

    //todo add custom filter options
}
impl Args {
    pub fn to_config(self) -> Config {
        Config {
            input_pbf: self.input_pbf,
            output_pbf: self.output_pbf,

            with_node_filtering: ! self.suppress_node_filtering,
            remove_metadata: ! self.keep_metadata,

            country_index: self.country_index,
            country_csv: self.country_csv,
            country_tile_size: self.country_tile_size,

            elevation_tiffs: self.elevation_tiffs,
            elevation_batch_size: self.elevation_batch_size,
            elevation_total_buffer_size: self.elevation_total_buffer_size,
            elevation_way_splitting: self.elevation_way_splitting,
            elevation_threshold: self.elevation_threshold,
            resolution_lon: self.resolution_lon,
            resolution_lat: self.resolution_lat,
            keep_original_elevation: self.keep_original_elevation,

            print_node_ids: HashSet::from_iter(self.print_node_ids),
            print_way_ids: HashSet::from_iter(self.print_way_ids),
            print_relation_ids: HashSet::from_iter(self.print_relation_ids),

            statistics_level: self.stat,
            debug: self.debug,
        }
    }
}
