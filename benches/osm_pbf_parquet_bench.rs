use criterion::{criterion_group, criterion_main, Criterion};
use osm_pbf_parquet::pbf_driver;
use osm_pbf_parquet::util::Args;
use std::fs;

pub fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("Bench 3", |b| {
        b.iter(|| {
            let args = Args::new(
                "test/karlsruhe-regbez-latest.osm.pbf".to_string(),
                "./test/bench-out/".to_string(),
                0,
            );
            let _ = pbf_driver(args);
        })
    });
}

criterion_group! {
    name = benches;
    config = Criterion::default().sample_size(10);
    targets = criterion_benchmark
}
criterion_main!(benches);