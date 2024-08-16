// benches/benchmark_run_all
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::path::PathBuf;
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
        debug: 3,
    }
}

#[allow(dead_code)]
fn benchmark_run_all(_c: &mut Criterion) {
    let mut config = base_config();
    config.output_pbf = Some(PathBuf::from("target/tmp/output-integration-bench-run_all.pbf"));
    config.country_csv = Some(PathBuf::from("test/mapping_test.csv"));
    config.elevation_tiffs = vec!["test/srtm*.tif".to_string(), "test/region*.tif".to_string()];
    config.elevation_batch_size = 100000;
    config.elevation_total_buffer_size = 500000;
    config.with_node_filtering = true;
    config.remove_metadata = true;

    let mut criterion = Criterion::default().sample_size(10); // Set sample size for this benchmark

    criterion.bench_function("run_all", |b| {
        b.iter(|| {
            rusty_routes_transformer::init(&config);
            let result = rusty_routes_transformer::run(&config);
            black_box(result);
        })
    });
}

criterion_group!(benches, benchmark_run_all);
criterion_main!(benches);