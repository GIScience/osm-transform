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
    #[arg(short, long, value_name = "FILE")]
    pub(crate) input_pbf: PathBuf,

    /// Result PBF file. Will be overwritten if existing! If not specified, no output is written to a file. But can still be useful in combination with id-logging.
    #[arg(short, long, value_name = "FILE")]
    pub(crate) output_pbf: Option<PathBuf>,

    /// CSV File with border geometries for country mapping. If not specified, no area mapping is performed, no country tags added.
    #[arg(short, long, value_name = "FILE")]
    pub(crate) country_csv: Option<PathBuf>,

    /// Turn debugging information on.
    #[arg(short, long, action = clap::ArgAction::Count)]
    pub debug: u8,

    /// Suppress node filtering. This means, that ALL nodes, ways, relations are handled by thy processing pass.
    #[arg(long)]
    pub suppress_node_filtering: bool,

    /// Suppress changing the pbf elements. The output (except pbf header) should be the same as the input. Can be used for performance measuring of the read/write/node-filtering parts.
    #[arg(long)]
    pub suppress_processing: bool,
}
impl Args {
    pub fn to_config(mut self) -> Config {
        Config {
            input_pbf: self.input_pbf,
            country_csv: self.country_csv,
            output_pbf: self.output_pbf,
            debug: self.debug,
            with_node_filtering: ! self.suppress_node_filtering,
            with_processing: ! self.suppress_processing,
        }
    }
}
