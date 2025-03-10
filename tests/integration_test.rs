use osm_io::osm::model::element::Element;
use osm_io::osm::pbf::reader::Reader;
use rusty_routes_transformer::Config;
use std::collections::HashSet;
use std::path::PathBuf;
use std::{fs, panic};
use std::fs::File;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;

fn base_config() -> Config {
    let mut config = Config {
        input_pbf: PathBuf::from("test/baarle_small.pbf"),
        output_pbf: None,
        country_data: None,
        country_tile_size: 0.4,
        elevation_tiffs: vec![],
        elevation_batch_size: 10000,
        elevation_total_buffer_size: 50000,
        elevation_way_splitting: false,
        remove_metadata: false,
        keep_original_elevation: false,
        with_node_filtering: false,
        print_node_ids: HashSet::new(),
        print_way_ids: HashSet::new(),
        print_relation_ids: HashSet::new(),
        resolution_lon: 0.0001,
        resolution_lat: 0.0001,
        elevation_threshold: 1.0,
        verbosity: 3u8,
        loglevel: 0,
        quiet: false,
    };
    config.print_way_ids.insert(7216689i64);
    config.print_node_ids.insert(1);
    config
}

const BAARLE_NODE_COUNT: u64 = 3964u64;
const BAARLE_RELATION_COUNT: u64 = 59u64;
const BAARLE_WAY_COUNT: u64 = 463u64;
const FILTERED_NODE_COUNT: u64 = 299u64;
const FILTERED_RELATION_COUNT: u64 = 29u64;
const FILTERED_WAY_COUNT: u64 = 51u64;
const SPLIT_NODE_COUNT: u64 = 3973u64;
const FILTERED_SPLIT_NODE_COUNT: u64 = 308u64;

#[test]
fn run_minimal() {
    let config = base_config();
    rusty_routes_transformer::init(&config);
    let data = rusty_routes_transformer::run(&config);
    println!("{}", data.summary(&config));
    assert_eq!(&data.input_node_count, &BAARLE_NODE_COUNT);
    assert_eq!(&data.output_node_count, &BAARLE_NODE_COUNT);
    assert_eq!(&data.input_relation_count, &BAARLE_RELATION_COUNT);
    assert_eq!(&data.output_relation_count, &FILTERED_RELATION_COUNT);
    assert_eq!(&data.input_way_count, &BAARLE_WAY_COUNT);
    assert_eq!(&data.output_way_count, &FILTERED_WAY_COUNT);
}
#[test]
fn run_minimal_write() {
    let mut config = base_config();
    config.output_pbf = Some(PathBuf::from("target/tmp/output-integration-test-run_minimal_write.pbf"));
    rusty_routes_transformer::init(&config);
    let data = rusty_routes_transformer::run(&config);
    println!("{}", data.summary(&config));
    assert_eq!(&data.input_node_count, &BAARLE_NODE_COUNT);
    assert_eq!(&data.output_node_count, &BAARLE_NODE_COUNT);
    assert_eq!(&data.input_relation_count, &BAARLE_RELATION_COUNT);
    assert_eq!(&data.output_relation_count, &FILTERED_RELATION_COUNT);
    assert_eq!(&data.input_way_count, &BAARLE_WAY_COUNT);
    assert_eq!(&data.output_way_count, &FILTERED_WAY_COUNT);
}
#[test]
fn run_all() {
    fs::remove_dir_all("mapping_test_idx_0_40");
    let mut config = base_config();
    config.output_pbf = Some(PathBuf::from("target/tmp/output-integration-test-run_all.pbf"));
    config.country_data = Some(PathBuf::from("test/mapping_test.csv"));
    config.country_tile_size = 0.4;
    config.elevation_tiffs = vec!["test/*.tif".to_string()];
    config.elevation_batch_size = 100000;
    config.elevation_total_buffer_size = 500000;
    config.with_node_filtering = true;
    config.remove_metadata = true;
    config.elevation_way_splitting = true;
    rusty_routes_transformer::init(&config);
    let data = rusty_routes_transformer::run(&config);
    println!("{}", data.summary(&config));
    assert_eq!(&data.input_node_count, &BAARLE_NODE_COUNT);
    assert_eq!(&data.output_node_count, &FILTERED_SPLIT_NODE_COUNT);
    assert_eq!(&data.input_relation_count, &BAARLE_RELATION_COUNT);
    assert_eq!(&data.output_relation_count, &FILTERED_RELATION_COUNT);
    assert_eq!(&data.input_way_count, &BAARLE_WAY_COUNT);
    assert_eq!(&data.output_way_count, &FILTERED_WAY_COUNT);
    check_pbf("target/tmp/output-integration-test-run_all.pbf", Some(42645645));
    check_pbf("target/tmp/output-integration-test-run_all.pbf", Some(50000000001));
}
#[test]
fn run_country() {
    fs::remove_dir_all("mapping_test_idx_0_40");
    let mut config = base_config();
    config.country_data = Some(PathBuf::from("test/mapping_test.csv"));
    config.country_tile_size = 0.4;
    rusty_routes_transformer::init(&config);
    let data = rusty_routes_transformer::run(&config);
    println!("{}", data.summary(&config));
    assert_eq!(&data.input_node_count, &BAARLE_NODE_COUNT);
    assert_eq!(&data.output_node_count, &BAARLE_NODE_COUNT);
    assert_eq!(&data.input_relation_count, &BAARLE_RELATION_COUNT);
    assert_eq!(&data.output_relation_count, &FILTERED_RELATION_COUNT);
    assert_eq!(&data.input_way_count, &BAARLE_WAY_COUNT);
    assert_eq!(&data.output_way_count, &FILTERED_WAY_COUNT);
}

#[test]
fn run_node_filtering() {
    let mut config = base_config();
    config.with_node_filtering = true;
    rusty_routes_transformer::init(&config);
    let data = rusty_routes_transformer::run(&config);
    println!("{}", data.summary(&config));
    assert_eq!(&data.input_node_count, &BAARLE_NODE_COUNT);
    assert_eq!(&data.output_node_count, &FILTERED_NODE_COUNT);
    assert_eq!(&data.input_relation_count, &BAARLE_RELATION_COUNT);
    assert_eq!(&data.output_relation_count, &FILTERED_RELATION_COUNT);
    assert_eq!(&data.input_way_count, &BAARLE_WAY_COUNT);
    assert_eq!(&data.output_way_count, &FILTERED_WAY_COUNT);
}

#[test]
fn run_remove_metadata() {
    let mut config = base_config();
    config.remove_metadata = true;
    rusty_routes_transformer::init(&config);
    let data = rusty_routes_transformer::run(&config);
    println!("{}", data.summary(&config));
    assert_eq!(&data.input_node_count, &BAARLE_NODE_COUNT);
    assert_eq!(&data.output_node_count, &BAARLE_NODE_COUNT);
    assert_eq!(&data.input_relation_count, &BAARLE_RELATION_COUNT);
    assert_eq!(&data.output_relation_count, &FILTERED_RELATION_COUNT);
    assert_eq!(&data.input_way_count, &BAARLE_WAY_COUNT);
    assert_eq!(&data.output_way_count, &FILTERED_WAY_COUNT);
}
#[test]
fn run_elevation() {
    let mut config = base_config();
    config.elevation_tiffs = vec!["test/*.tif".to_string()];
    rusty_routes_transformer::init(&config);
    let data = rusty_routes_transformer::run(&config);
    println!("{}", data.summary(&config));
    assert_eq!(&data.input_node_count, &BAARLE_NODE_COUNT);
    assert_eq!(&data.output_node_count, &BAARLE_NODE_COUNT);
    assert_eq!(&data.input_relation_count, &BAARLE_RELATION_COUNT);
    assert_eq!(&data.output_relation_count, &FILTERED_RELATION_COUNT);
    assert_eq!(&data.input_way_count, &BAARLE_WAY_COUNT);
    assert_eq!(&data.output_way_count, &FILTERED_WAY_COUNT);
}
#[test]
fn run_elevation_way_splitting() {
    let mut config = base_config();
    config.elevation_tiffs = vec!["test/*.tif".to_string()];
    config.elevation_way_splitting = true;
    rusty_routes_transformer::init(&config);
    let data = rusty_routes_transformer::run(&config);
    println!("{}", data.summary(&config));
    assert_eq!(&data.input_node_count, &BAARLE_NODE_COUNT);
    assert!(&data.output_node_count > &BAARLE_NODE_COUNT);
    assert_eq!(&data.input_relation_count, &BAARLE_RELATION_COUNT);
    assert_eq!(&data.output_relation_count, &FILTERED_RELATION_COUNT);
    assert_eq!(&data.input_way_count, &BAARLE_WAY_COUNT);
    assert_eq!(&data.output_way_count, &FILTERED_WAY_COUNT);
}
#[test]
fn run_elevation_way_splitting_write() {
    let mut config = base_config();
    config.elevation_tiffs = vec!["test/*.tif".to_string()];
    config.elevation_way_splitting = true;
    config.output_pbf = Some(PathBuf::from("target/tmp/output-integration-test-run_elevation_way_splitting_write.pbf"));
    rusty_routes_transformer::init(&config);
    let data = rusty_routes_transformer::run(&config);
    println!("{}", data.summary(&config));
    assert_eq!(&data.input_node_count, &BAARLE_NODE_COUNT);
    assert_eq!(&data.output_node_count, &SPLIT_NODE_COUNT);
    assert_eq!(&data.input_relation_count, &BAARLE_RELATION_COUNT);
    assert_eq!(&data.output_relation_count, &FILTERED_RELATION_COUNT);
    assert_eq!(&data.input_way_count, &BAARLE_WAY_COUNT);
    assert_eq!(&data.output_way_count, &FILTERED_WAY_COUNT);
    check_pbf("target/tmp/output-integration-test-run_elevation_way_splitting_write.pbf", Some(42645645));
    check_pbf("target/tmp/output-integration-test-run_elevation_way_splitting_write.pbf", Some(50000000001));
}

fn check_pbf(path: &str, expected_node: Option<i64>) {
    let path_buf = PathBuf::from(path);
    let reader = Reader::new(&path_buf).expect("pbf file not found");
    let mut node_found = false;
    for element in reader.elements().expect("corrupted file") {
        match &element {
            Element::Node { node } => {
                if let Some(expected_node) = expected_node {
                    if node.id() == expected_node {
                        node_found = true;
                    }
                }
            }
            _ => (),
        }
    }
    if let Some(_expected_node) = expected_node {
        assert!(node_found);
    }
}

#[test]
fn fail_validation_if_input_file_does_not_exist() {
    let mut config = base_config();
    config.input_pbf = PathBuf::from("test/does_not_exist.pbf");
    validate_and_expect_error(config);
}
#[test] fn fail_validation_if_input_file_is_empty() {
    let mut config = base_config();
    config.input_pbf =  PathBuf::from("target/tmp/empty_test_input.pbf");
    test_with_file(&PathBuf::from("target/tmp/empty_test_input.pbf"), "simulated empty input file", validate_and_expect_error, config );
}
#[test] fn fail_validation_if_input_file_is_not_readable() {
    let mut config = base_config();
    config.input_pbf = PathBuf::from("target/tmp/readonly_input.pbf");
    let path_buf = PathBuf::from("target/tmp/readonly_input.pbf");
    let mut input_file = File::create(&path_buf).expect("could not create simulated input file");
    input_file.write_all("content".as_bytes()).expect("could not write to simulated input file");
    fs::set_permissions(&path_buf, fs::Permissions::from_mode(0o333)).expect("could not set permissions on simulated input file");
    validate_and_expect_error(config);
    fs::remove_file(path_buf).expect("removing simulated input file failed");
}
#[test] fn fail_validation_if_output_file_already_exists() {
    let mut config = base_config();
    config.input_pbf = PathBuf::from("target/tmp/test_output.pbf");
    test_with_file(&PathBuf::from("target/tmp/test_output.pbf"), "simulated pre-existing output file", validate_and_expect_error, config );
}
#[test]
fn fail_validation_if_output_file_in_readonly_directory() {
    let mut config = base_config();
    config.output_pbf = Some(PathBuf::from("test_dir_readonly/output.pbf"));
    test_with_readonly_dir(&PathBuf::from("test_dir_readonly"), "simulated pre-existing readonly output directory", validate_and_expect_error, config );
}

#[test]
fn fail_validation_if_country_index_directory_already_exists() {
    let mut config = base_config();
    config.country_data = Some(PathBuf::from("test/mapping_test.csv"));
    config.country_tile_size = 2.0;
    test_with_dir(&PathBuf::from("mapping_test_idx_2_00"), "simulated pre-existing country index directory", validate_and_expect_error, config );
}


fn test_with_file(path_buf: &PathBuf, label: &str, test_fn: fn(Config), config: Config) {
    let msg = format!("{}: {}", label, path_buf.to_str().unwrap());
    File::create(path_buf).expect(&msg);
    test_fn(config);
    fs::remove_file(path_buf).expect(&msg);
}

fn test_with_dir(path_buf: &PathBuf, label: &str, test_fn: fn(Config), config: Config) {
    let msg = format!("{}: {}", label, path_buf.to_str().unwrap());
    fs::create_dir(path_buf).expect(&msg);
    test_fn(config);
    fs::remove_dir_all(path_buf).expect(&msg);
}

fn test_with_readonly_dir(path_buf: &PathBuf, label: &str, test_fn: fn(Config), config: Config) {
    let msg = format!("{}: {}", label, path_buf.to_str().unwrap());
    fs::create_dir(path_buf).expect(&msg);
    fs::set_permissions(path_buf, fs::Permissions::from_mode(0o444)).expect(&msg);
    test_fn(config);
    fs::remove_dir_all(path_buf).expect(&msg);
}

fn validate_and_expect_error(config: Config) {
    let result = panic::catch_unwind(|| {
        rusty_routes_transformer::init(&config);
        rusty_routes_transformer::validate(&config);
    });
    assert!(result.is_err());
}