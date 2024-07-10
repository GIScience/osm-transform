use std::env;
use rusty_routes_transformer::conf::Config;
use rusty_routes_transformer::{benchmark_io, run};
use log4rs::append::console::ConsoleAppender;
use log4rs::config::{Appender, Logger, Root};
use log::LevelFilter;

fn main() {

    let stdout = ConsoleAppender::builder().build();
    let config = log4rs::Config::builder()
        .appender(Appender::builder().build("stdout", Box::new(stdout)))
        .logger(Logger::builder().build("rusty_routes_transformer", LevelFilter::Info))
        .build(Root::builder().appender("stdout").build(LevelFilter::Info))
        .unwrap();
    let _handle = log4rs::init_config(config).unwrap();

    let mut config = read_conf_file();
    merge_args(&mut config);
    benchmark_io(&config);
    //run(&config);
}

fn merge_args(config: &mut Config) {
    config.param = 222;
    let args: Vec<String> = env::args().collect();
    if args.len() > 1usize {
        config.input_path = args[1].to_string();
    }
    if args.len() > 2usize {
        config.country_path = args[2].to_string();
    }
}

fn read_conf_file() -> Config {
    Config::default()
}
