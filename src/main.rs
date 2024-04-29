use rusty_routes_transformer::run;
use rusty_routes_transformer::conf::Config;

fn main() {
    let mut config = read_conf_file();
    merge_args(&mut config);
    run(config);
}

fn merge_args(config: &mut Config) {
    config.param = 222
}

fn read_conf_file() -> Config {
    Config{param: 111}
}
