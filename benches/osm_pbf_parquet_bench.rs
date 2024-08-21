use criterion::{criterion_group, criterion_main, Criterion};
use osm_pbf_parquet::driver;
use osm_pbf_parquet::util::Args;
use std::fs;

pub fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("Bench 3", |b| {
        b.iter(|| {
            let args = Args::new(
                "/home/jules/HeiGIT/repos/rust-parquet-osm-transform/benches/files/karlsruhe-regbez-latest.osm.pbf".to_string(),
                "./test/bench-out/".to_string(),
                0,
            );
            let _ = driver(args);
        })
    });
}

criterion_group! {
    name = benches;
    config = Criterion::default().sample_size(10);
    targets = criterion_benchmark
}
criterion_main!(benches);