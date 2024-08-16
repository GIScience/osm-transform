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

fn benchmark_run_minimal(_c: &mut Criterion) {
    let config = base_config();
    let mut criterion = Criterion::default().sample_size(100); // Set sample size for this benchmark

    criterion.bench_function("run_minimal", |b| {
        b.iter(|| {
            rusty_routes_transformer::init(&config);
            let result = rusty_routes_transformer::run(&config);
            black_box(result);
        })
    });
}

criterion_group!(benches, benchmark_run_minimal);
criterion_main!(benches);