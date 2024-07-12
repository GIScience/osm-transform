use std::path::PathBuf;
use rusty_routes_transformer::Config;
use rusty_routes_transformer::handler::HandlerResult;

#[test]
fn run_all() {
    let config = Config {
        input_pbf: PathBuf::from("test/baarle_small.pbf"),
        output_pbf:  Some(PathBuf::from("target/tmp/output_integration-test-run_all.pbf")),
        country_csv: Some(PathBuf::from("test/mapping_test.csv")),
        debug: 1,
        with_processing: true,
        with_node_filtering: true,
    };
    rusty_routes_transformer::init(&config);
    let result = rusty_routes_transformer::run(&config);
    assert_eq!(result.count_all_nodes, 3964);
    assert_eq!(result.count_accepted_nodes, 299);
}

#[test]
fn run_no_output_pbf() {
    let config = Config {
        input_pbf: PathBuf::from("test/baarle_small.pbf"),
        output_pbf:  None,
        country_csv: Some(PathBuf::from("test/mapping_test.csv")),
        debug: 1,
        with_processing: true,
        with_node_filtering: true,
    };
    let result = rusty_routes_transformer::run(&config);
    assert_eq!(result.count_all_nodes, 3964);
    assert_eq!(result.count_accepted_nodes, 299);
}

#[test]
fn run_no_country_csv() {
    let config = Config {
        input_pbf: PathBuf::from("test/baarle_small.pbf"),
        output_pbf:  Some(PathBuf::from("target/tmp/output_integration-test-run_no_country.pbf")),
        country_csv: None,
        debug: 1,
        with_processing: true,
        with_node_filtering: true,
    };
    rusty_routes_transformer::init(&config);
    let result = rusty_routes_transformer::run(&config);
    assert_eq!(result.count_all_nodes, 3964);
    assert_eq!(result.count_accepted_nodes, 299);
}