use std::collections::HashSet;
use std::path::PathBuf;

use rusty_routes_transformer::Config;

fn base_config() -> Config {
    Config {
        input_pbf: PathBuf::from("test/baarle_small.pbf"),
        output_pbf:  Some(PathBuf::from("target/tmp/output_integration-test-run_all.pbf")),
        country_csv: Some(PathBuf::from("test/mapping_test.csv")),
        debug: 1,
        with_node_filtering: true,
        print_node_ids: HashSet::new(),
        print_way_ids: HashSet::new(),
        print_relation_ids: HashSet::new(),
    }
}

#[test]
fn run_all() {
    let mut config = base_config();
    rusty_routes_transformer::init(&config);
    let result = rusty_routes_transformer::run(&config);
    assert_eq!(result.count_all_nodes, 3964);
    assert_eq!(result.count_accepted_nodes, 299);
}

#[test]
fn run_no_output_pbf() {
    let mut config = base_config();
    config.country_csv = None;
    let result = rusty_routes_transformer::run(&config);
    assert_eq!(result.count_all_nodes, 3964);
    assert_eq!(result.count_accepted_nodes, 299);
}

#[test]
fn run_no_country_csv() {
    let mut config = base_config();
    config.output_pbf = None;
    rusty_routes_transformer::init(&config);
    let result = rusty_routes_transformer::run(&config);
    assert_eq!(result.count_all_nodes, 3964);
    assert_eq!(result.count_accepted_nodes, 299);
}