pub mod io;
pub mod area;
pub mod handler;
pub mod output;
pub mod osm_model;

use std::path::PathBuf;
use benchmark_rs::stopwatch::StopWatch;
use log4rs::append::console::ConsoleAppender;
use log4rs::config::{Appender, Logger, Root};
use log::LevelFilter;
use crate::io::process_with_handler;
use area::AreaHandler;
use crate::handler::{FinalHandler, HandlerChain};
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
        .logger(Logger::builder().build("rrt", log_level))
        .build(Root::builder().appender("stdout").build(log_level))
        .unwrap();
    let _handle = log4rs::init_config(config).unwrap();
}

pub fn run(config: &Config) {
    let mut stopwatch = StopWatch::new();

    let mut handler_chain = HandlerChain::default();

    match &config.country_csv {
        Some(path_buf) => {
            log::info!("Reading area mapping CSV");
            stopwatch.start();
            let mut area_handler = AreaHandler::default();
            area_handler.load(path_buf.clone()).expect("Area handler failed to load CSV file");
            log::info!("Loaded: {} areas", area_handler.mapping.id.len());
            log::info!("Finished reading area mapping, time: {}", stopwatch);
            handler_chain = handler_chain.add_unboxed(area_handler);
            stopwatch.reset();
        }
        None => ()
    }

    match &config.output_pbf {
        Some(path_buf) => {
            log::info!("Initializing ouput handler");
            stopwatch.start();
            let mut output_handler = OutputHandler::new(path_buf.clone());
            output_handler.init();
            stopwatch.reset();
            handler_chain = handler_chain.add_unboxed(output_handler);
        }
        None => {
            handler_chain = handler_chain.add_unboxed(FinalHandler::new());
        }
    }

    stopwatch.start();
    let _ = process_with_handler(config, &mut handler_chain).expect("Area handler failed");
    log::info!("Finished mapping, time: {}", stopwatch);
}


#[derive(Debug, Default)]
pub struct Config {
    pub input_pbf: PathBuf,
    pub country_csv: Option<PathBuf>,
    pub output_pbf: Option<PathBuf>,
    pub debug: u8
}
