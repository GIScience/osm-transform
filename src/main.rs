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
    /// PBF file to preprocess
    #[arg(short, long, value_name = "FILE")]
    pub(crate) input_pbf: PathBuf,

    /// Result PBF file
    #[arg(short, long, value_name = "FILE")]
    pub(crate) output_pbf: Option<PathBuf>,

    /// CSV File with border geometries for country mapping
    #[arg(short, long, value_name = "FILE")]
    pub(crate) country_csv: Option<PathBuf>,

    /// Turn debugging information on
    #[arg(short, long, action = clap::ArgAction::Count)]
    pub debug: u8,
}
impl Args {
    pub fn to_config(mut self) -> Config {
        Config {
            input_pbf: self.input_pbf,
            country_csv: self.country_csv,
            output_pbf: self.output_pbf,
            debug: self.debug,
        }
    }
}
