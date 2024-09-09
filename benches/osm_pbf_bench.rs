use criterion::{criterion_group, criterion_main, Criterion};
use osmpbf::{Element, ElementReader};
use std::env;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;


fn bench_count(c: &mut Criterion) {
    let file = PathBuf::from("test/karlsruhe-regbez-latest.osm.pbf");
    // print the current working directory
    println!("Current working directory: {:?}", std::env::current_dir().unwrap());
    #[cfg(feature = "rust-zlib")]
    println!("Using rust-zlib (miniz_oxide)");
    #[cfg(feature = "zlib")]
    println!("Using zlib");
    #[cfg(feature = "zlib-ng")]
    println!("Using zlib-ng");
    let total_nodes = Arc::new(AtomicUsize::new(0));
    let total_ways = Arc::new(AtomicUsize::new(0));
    let total_relations = Arc::new(AtomicUsize::new(0));

    let nodes_clone = Arc::clone(&total_nodes);
    let ways_clone = Arc::clone(&total_ways);
    let relations_clone = Arc::clone(&total_relations);

    c.bench_function("Bench 2", |b| {
        b.iter(|| {
            let path = std::path::Path::new(&file);
            let reader = ElementReader::from_path(path).unwrap();
            let (nodes, ways, relations) = reader
                .par_map_reduce(
                    |element| match element {
                        Element::Node(_) | Element::DenseNode(_) => (1, 0, 0),
                        Element::Way(_) => (0, 1, 0),
                        Element::Relation(_) => (0, 0, 1),
                    },
                    || (0u64, 0u64, 0u64),
                    |a, b| (a.0 + b.0, a.1 + b.1, a.2 + b.2),
                )
                .unwrap();
            nodes_clone.fetch_add(nodes as usize, Ordering::SeqCst);
            ways_clone.fetch_add(ways as usize, Ordering::SeqCst);
            relations_clone.fetch_add(relations as usize, Ordering::SeqCst);
        })
    });

    println!("Total Nodes: {}", total_nodes.load(Ordering::SeqCst));
    println!("Total Ways: {}", total_ways.load(Ordering::SeqCst));
    println!("Total Relations: {}", total_relations.load(Ordering::SeqCst));;
}
criterion_group! {
    name = benches;
    config = Criterion::default().sample_size(10);
    targets = bench_count
}
criterion_main!(benches);
