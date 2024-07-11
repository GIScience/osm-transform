use std::env;
use std::path::PathBuf;
use clap::Parser;
use rusty_routes_transformer::conf::Config;
use rusty_routes_transformer::run;
use log4rs::append::console::ConsoleAppender;
use log4rs::config::{Appender, Logger, Root};
use log::LevelFilter;

fn main() {
    let args = Args::parse();

    let log_level: LevelFilter;
    match args.debug {
        0 => log_level = LevelFilter::Info,
        1 => log_level = LevelFilter::Debug,
        2 => log_level = LevelFilter::Trace,
        _ => log_level = LevelFilter::Off,
    }

    let stdout = ConsoleAppender::builder().build();
    let config = log4rs::Config::builder()
        .appender(Appender::builder().build("stdout", Box::new(stdout)))
        .logger(Logger::builder().build("rusty_routes_transformer", log_level))
        .build(Root::builder().appender("stdout").build(log_level))
        .unwrap();
    let _handle = log4rs::init_config(config).unwrap();

    let mut config = read_conf_file();
    merge_args(&mut config, &args);
    run(&config);
}

fn merge_args(config: &mut Config, args: &Args) {

    config.input_path = args.input_pbf.clone();
    config.output_path = args.output_pbf.clone();
    config.country_path = args.borders.clone();
}

fn read_conf_file() -> Config {
    Config::default()
}


/// Preprocessor to prepare OSM PBF-Files for openrouteservice
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// PBF file to preprocess
    #[arg(short, long, value_name = "FILE")]
    input_pbf: String, //Option<PathBuf>,

    /// Result PBF file
    #[arg(short, long, value_name = "FILE")]
    output_pbf: String, //Option<PathBuf>,

    /// CSV File with border geometries for country mapping
    #[arg(short, long, value_name = "FILE")]
    borders: String, //Option<PathBuf>,

    /// Turn debugging information on
    #[arg(short, long, action = clap::ArgAction::Count)]
    debug: u8,
}
