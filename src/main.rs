use std::collections::HashSet;
use std::path::PathBuf;

use clap::Parser;

use rusty_routes_transformer::{Config, init, run};

fn main() {
    let args = Args::parse();
    let config = args.to_config();
    init(&config);
    run(&config);
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
}
impl Args {
    pub fn to_config(mut self) -> Config {
        Config {
            input_pbf: self.input_pbf,
            country_csv: self.country_csv,
            output_pbf: self.output_pbf,
            debug: self.debug,
            with_node_filtering: ! self.suppress_node_filtering,
            print_node_ids: HashSet::from_iter(self.print_node_ids),
            print_way_ids: HashSet::from_iter(self.print_way_ids),
            print_relation_ids: HashSet::from_iter(self.print_relation_ids),
        }
    }
}
