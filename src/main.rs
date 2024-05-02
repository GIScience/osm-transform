use rusty_routes_transformer::conf::Config;
use rusty_routes_transformer::run;

fn main() {
    let mut config = read_conf_file();
    merge_args(&mut config);
    run(config);
}

fn merge_args(config: &mut Config) {
    config.param = 222
}

fn read_conf_file() -> Config {
    Config { param: 111, country_path: "./test/mapping_test.csv".to_string() }
}
