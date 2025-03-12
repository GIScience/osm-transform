use std::collections::HashSet;
use std::path::PathBuf;

use clap::Parser;

use osm_transform::{Config, init, run, validate};
use osm_transform::handler::HandlerData;

fn main() {
    let args = Args::parse();
    let config = args.to_config();
    init(&config);
    validate(&config);
    let handler_data = run(&config);
    print_statistics(&config, handler_data);
}

fn print_statistics(config: &Config, handler_data: HandlerData) {
    if config.get_summary_level() > 0 {
        println!("{}", handler_data.summary(&config));
    }
}

/// Preprocessor to prepare OSM PBF-Files for openrouteservice
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// PBF file to preprocess.
    #[arg(short = 'i', long, value_name = "FILE")]
    pub(crate) input_pbf: Option<PathBuf>,

    /// Result PBF file. Will be overwritten if existing! If not specified, no output is written to a file. But can still be useful in combination with id-logging.
    #[arg(short = 'o', long, value_name = "FILE")]
    pub(crate) output_pbf: Option<PathBuf>,

    /// File or directory with data for country enrichment:
    /// If a country CSV file with border geometries is specified, a country index is computed and saved to an index directory.
    /// If an index directory of a previous run is specified, the pre-computed index is loaded, which is much faster.
    /// In this case, --country-tile-size is ignored.
    /// If neither of the two entries is made, no country enrichment is carried out and no country codes are added to nodes.
    #[arg(short = 'c', long, value_name = "FILE")]
    pub(crate) country_data: Option<PathBuf>,

    /// Tile size for the country index grid in decimal degrees. Only relevant for country index computation, see --country-data.
    #[arg(long, default_value = "1.0")]
    pub(crate) country_tile_size: f64,

    /// Elevation GeoTiff Files (glob patterns allowed) to enrich nodes with elevation data.
    #[arg(short = 'e', long, value_name = "PATTERN", num_args = 1..)]
    pub(crate) elevation_tiffs: Vec<String>,

    /// Split ways at elevation changes.
    #[arg(short = 'w', long)]
    pub elevation_way_splitting: bool,

    /// Resolution for elevation way splitting in direction longitude
    #[arg(long, default_value = "0.001")]
    pub elevation_resolution_lon: f64,

    /// Resolution for elevation way splitting in direction latitude
    #[arg(long, default_value = "0.001")]
    pub elevation_resolution_lat: f64,

    /// Threshold for elevation way splitting
    #[arg(long, default_value = "10.0")]
    pub elevation_threshold: f64,

    /// Size of the elevation buffer for each elevation tiff file. This is the number of nodes that are buffered in memory before their elevation is read from the elevation tiff file in a batch.
    #[arg(short = 'b', long, default_value = "1000000")]
    pub elevation_batch_size: usize,

    /// Total number of nodes that are buffered in all elevation file buffers. When the number is reached, the largest buffer is flushed.
    #[arg(short = 't', long, default_value = "50000000")]
    pub elevation_total_buffer_size: usize,

    /// Do NOT overwrite original elevation data with the value from an elevation tiff.
    #[arg(long)]
    pub elevation_keep_original_value: bool,

    /// Do NOT remove nodes that are not referenced by accepted ways or relations.
    /// Not recommended for openrouteservice graph building
    #[arg(long)]
    pub suppress_node_filtering: bool,

    /// Do NOT remove metadata 'version', 'timestamp', 'changeset', 'uid', 'user'
    #[arg(long)]
    pub keep_metadata: bool,

    /// Print node with id=<ID> at beginning and end of processing pipeline. Can be added multiple times.
    #[arg(short = 'N', long, value_name = "ID")]
    pub print_node: Vec<i64>,

    /// Print way with id=<ID> at beginning and end of processing pipeline. Can be added multiple times.
    #[arg(short = 'W', long, value_name = "ID")]
    pub print_way: Vec<i64>,

    /// Print relation with id=<ID> at beginning and end of processing pipeline. Can be added multiple times.
    #[arg(short = 'R', long, value_name = "ID")]
    pub print_relation: Vec<i64>,

    /// Can be added multiple times to get more detailed summery
    #[arg(short = 'v', long, action = clap::ArgAction::Count)]
    pub verbose: u8,

    /// No output to stdout. This overrules --verbose
    #[arg(short = 'q', long)]
    pub quiet: bool,

    #[arg(short = '@', long, hide = true, action = clap::ArgAction::Count)]
    pub loglevel: u8,

    //todo add custom filter options
}
impl Args {
    pub fn to_config(self) -> Config {
        Config {
            input_pbf: self.input_pbf,
            output_pbf: self.output_pbf,

            with_node_filtering: ! self.suppress_node_filtering,
            remove_metadata: ! self.keep_metadata,

            country_data: self.country_data,
            country_tile_size: self.country_tile_size,

            elevation_tiffs: self.elevation_tiffs,
            elevation_batch_size: self.elevation_batch_size,
            elevation_total_buffer_size: self.elevation_total_buffer_size,
            elevation_way_splitting: self.elevation_way_splitting,
            elevation_threshold: self.elevation_threshold,
            resolution_lon: self.elevation_resolution_lon,
            resolution_lat: self.elevation_resolution_lat,
            keep_original_elevation: self.elevation_keep_original_value,

            print_node_ids: HashSet::from_iter(self.print_node),
            print_way_ids: HashSet::from_iter(self.print_way),
            print_relation_ids: HashSet::from_iter(self.print_relation),

            verbosity: self.verbose,
            quiet: self.quiet,
            loglevel: self.loglevel,
        }
    }
}
