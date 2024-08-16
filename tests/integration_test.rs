use std::path::PathBuf;
use osm_io::osm::model::element::Element;
use osm_io::osm::pbf::reader::Reader;
use rustc_hash::FxHashSet;
use rusty_routes_transformer::Config;

fn base_config() -> Config {
    Config {
        input_pbf: PathBuf::from("test/baarle_small.pbf"),
        output_pbf: None,
        country_csv: None,
        elevation_tiffs: vec![],
        elevation_batch_size: 10000,
        elevation_total_buffer_size: 50000,
        remove_metadata: false,
        with_node_filtering: false,
        print_node_ids: FxHashSet::default(),
        print_way_ids: FxHashSet::default(),
        print_relation_ids: FxHashSet::default(),
        debug: 1,
    }
}

const BAARLE_NODE_COUNT: u64 = 3964u64;
const BAARLE_RELATION_COUNT: u64 = 59u64;
const BAARLE_WAY_COUNT: u64 = 463u64;
const FILTERED_NODE_COUNT: u64 = 299u64;
const FILTERED_RELATION_COUNT: u64 = 29u64;
const FILTERED_WAY_COUNT: u64 = 51u64;

#[test]
fn run_minimal() {
    let config = base_config();
    rusty_routes_transformer::init(&config);
    let result = rusty_routes_transformer::run(&config);
    assert_eq!(result.counts.get("nodes count initial").unwrap(), &BAARLE_NODE_COUNT);
    assert_eq!(result.counts.get("nodes count final").unwrap(), &BAARLE_NODE_COUNT);
    assert_eq!(result.counts.get("relations count initial").unwrap(), &BAARLE_RELATION_COUNT);
    assert_eq!(result.counts.get("relations count final").unwrap(), &FILTERED_RELATION_COUNT);
    assert_eq!(result.counts.get("ways count initial").unwrap(), &BAARLE_WAY_COUNT);
    assert_eq!(result.counts.get("ways count final").unwrap(), &FILTERED_WAY_COUNT);
}
#[test]
fn run_minimal_write() {
    let mut config = base_config();
    config.output_pbf = Some(PathBuf::from("target/tmp/output-integration-test-run_minimal_write.pbf"));
    rusty_routes_transformer::init(&config);
    let result = rusty_routes_transformer::run(&config);
    assert_eq!(result.counts.get("nodes count initial").unwrap(), &BAARLE_NODE_COUNT);
    assert_eq!(result.counts.get("nodes count final").unwrap(), &BAARLE_NODE_COUNT);
    assert_eq!(result.counts.get("relations count initial").unwrap(), &BAARLE_RELATION_COUNT);
    assert_eq!(result.counts.get("relations count final").unwrap(), &FILTERED_RELATION_COUNT);
    assert_eq!(result.counts.get("ways count initial").unwrap(), &BAARLE_WAY_COUNT);
    assert_eq!(result.counts.get("ways count final").unwrap(), &FILTERED_WAY_COUNT);
}
#[test]
fn run_all() {
    let mut config = base_config();
    config.output_pbf = Some(PathBuf::from("target/tmp/output-integration-test-run_all.pbf"));
    config.country_csv = Some(PathBuf::from("test/mapping_test.csv"));
    config.elevation_tiffs = vec!["test/srtm*.tif".to_string(), "test/region*.tif".to_string()];
    config.elevation_batch_size = 100000;
    config.elevation_total_buffer_size = 500000;
    config.with_node_filtering = true;
    config.remove_metadata = true;
    rusty_routes_transformer::init(&config);
    let result = rusty_routes_transformer::run(&config);
    assert_eq!(result.counts.get("nodes count initial").unwrap(), &BAARLE_NODE_COUNT);
    assert_eq!(result.counts.get("nodes count final").unwrap(), &FILTERED_NODE_COUNT);
    assert_eq!(result.counts.get("relations count initial").unwrap(), &BAARLE_RELATION_COUNT);
    assert_eq!(result.counts.get("relations count final").unwrap(), &FILTERED_RELATION_COUNT);
    assert_eq!(result.counts.get("ways count initial").unwrap(), &BAARLE_WAY_COUNT);
    assert_eq!(result.counts.get("ways count final").unwrap(), &FILTERED_WAY_COUNT);
    check_pbf("target/tmp/output-integration-test-run_all.pbf", Some(42645645));
}
#[test]
fn run_country() {
    let mut config = base_config();
    config.country_csv = Some(PathBuf::from("test/mapping_test.csv"));
    rusty_routes_transformer::init(&config);
    let result = rusty_routes_transformer::run(&config);
    assert_eq!(result.counts.get("nodes count initial").unwrap(), &BAARLE_NODE_COUNT);
    assert_eq!(result.counts.get("nodes count final").unwrap(), &BAARLE_NODE_COUNT);
    assert_eq!(result.counts.get("relations count initial").unwrap(), &BAARLE_RELATION_COUNT);
    assert_eq!(result.counts.get("relations count final").unwrap(), &FILTERED_RELATION_COUNT);
    assert_eq!(result.counts.get("ways count initial").unwrap(), &BAARLE_WAY_COUNT);
    assert_eq!(result.counts.get("ways count final").unwrap(), &FILTERED_WAY_COUNT);
}

#[test]
fn run_node_filtering() {
    let mut config = base_config();
    config.with_node_filtering = true;
    rusty_routes_transformer::init(&config);
    let result = rusty_routes_transformer::run(&config);
    assert_eq!(result.counts.get("nodes count initial").unwrap(), &BAARLE_NODE_COUNT);
    assert_eq!(result.counts.get("nodes count final").unwrap(), &FILTERED_NODE_COUNT);
    assert_eq!(result.counts.get("relations count initial").unwrap(), &BAARLE_RELATION_COUNT);
    assert_eq!(result.counts.get("relations count final").unwrap(), &FILTERED_RELATION_COUNT);
    assert_eq!(result.counts.get("ways count initial").unwrap(), &BAARLE_WAY_COUNT);
    assert_eq!(result.counts.get("ways count final").unwrap(), &FILTERED_WAY_COUNT);
}

#[test]
fn run_remove_metadata() {
    let mut config = base_config();
    config.remove_metadata = true;
    rusty_routes_transformer::init(&config);
    let result = rusty_routes_transformer::run(&config);
    assert_eq!(result.counts.get("nodes count initial").unwrap(), &BAARLE_NODE_COUNT);
    assert_eq!(result.counts.get("nodes count final").unwrap(), &BAARLE_NODE_COUNT);
    assert_eq!(result.counts.get("relations count initial").unwrap(), &BAARLE_RELATION_COUNT);
    assert_eq!(result.counts.get("relations count final").unwrap(), &FILTERED_RELATION_COUNT);
    assert_eq!(result.counts.get("ways count initial").unwrap(), &BAARLE_WAY_COUNT);
    assert_eq!(result.counts.get("ways count final").unwrap(), &FILTERED_WAY_COUNT);
}
#[test]
fn run_elevation() {
    let mut config = base_config();
    config.elevation_tiffs = vec!["test/*.tif".to_string()];
    rusty_routes_transformer::init(&config);
    let result = rusty_routes_transformer::run(&config);
    assert_eq!(result.counts.get("nodes count initial").unwrap(), &BAARLE_NODE_COUNT);
    assert_eq!(result.counts.get("nodes count final").unwrap(), &BAARLE_NODE_COUNT);
    assert_eq!(result.counts.get("relations count initial").unwrap(), &BAARLE_RELATION_COUNT);
    assert_eq!(result.counts.get("relations count final").unwrap(), &FILTERED_RELATION_COUNT);
    assert_eq!(result.counts.get("ways count initial").unwrap(), &BAARLE_WAY_COUNT);
    assert_eq!(result.counts.get("ways count final").unwrap(), &FILTERED_WAY_COUNT);
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