use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use anyhow::Error;
use benchmark_rs::stopwatch::StopWatch;
use criterion::{criterion_group, criterion_main, Criterion};
use simple_logger::SimpleLogger;

use osm_io::osm::apidb_dump::write::writer::Writer as ApiDbDumpWriter;
use osm_io::osm::model::element::Element::Node;
use osm_io::osm::pbf::reader::Reader as PbfReader;
use log::log;
use osm_io::osm::model::element::Element;
use osm_io::osm::model::node;
use osm_io::osm::pbf;
use osm_io::osm::pbf::parallel_writer::PBF_WRITER;
use serde::de::IntoDeserializer;

fn bench_count(c: &mut Criterion) {
    let mut group = c.benchmark_group("OSMIO Benchmark");

    let input_path = PathBuf::from("/home/jules/HeiGIT/repos/rust-parquet-osm-transform/benches/files/karlsruhe-regbez-latest.osm.pbf");
    #[cfg(feature = "rust-zlib")]
    println!("Using rust-zlib (miniz_oxide)");
    #[cfg(feature = "zlib")]
    println!("Using zlib");
    #[cfg(feature = "zlib-ng")]
    println!("Using zlib-ng");
    let reader = PbfReader::new(&input_path).unwrap();

    let mut nodes_single = Arc::new(AtomicUsize::new(0));
    let mut ways_single = Arc::new(AtomicUsize::new(0));
    let mut relations_single = Arc::new(AtomicUsize::new(0));

    let mut nodes_single_clone = nodes_single.clone();
    let mut ways_single_clone = ways_single.clone();
    let mut relations_single_clone = relations_single.clone();

    // Get all available cores
    let num_cores = num_cpus::get();
    if num_cores > 1 {
        num_cores - 1;
    }
    log::info!("Number of max cores: {}", num_cores);

    // group.bench_function(
    //     "Bench osmio single-threaded",
    //     |b| {
    //         b.iter(|| {
    //             let nodes_single_clone = nodes_single.clone();
    //             let ways_single_clone = ways_single.clone();
    //             let relations_single_clone = relations_single.clone();
    // 
    //             reader.parallel_for_each(1, move |element| {
    //                 match element {
    //                     Node { node: _ } => {
    //                         nodes_single_clone.fetch_add(1, Ordering::SeqCst);
    //                     }
    //                     Element::Way { .. } => {
    //                         ways_single_clone.fetch_add(1, Ordering::SeqCst);
    //                     }
    //                     Element::Relation { .. } => {
    //                         relations_single_clone.fetch_add(1, Ordering::SeqCst);
    //                     }
    //                     Element::Sentinel => {}
    //                 }
    //                 Ok(())
    //             }).unwrap();
    //         });
    //     },
    // );
    // 
    // 
    // println!("Results from multi-threaded");
    // println!("nodes: {}", nodes_single_clone.load(Ordering::SeqCst));
    // println!("ways: {}", ways_single_clone.load(Ordering::SeqCst));
    // println!("relations: {}", relations_single_clone.load(Ordering::SeqCst));

    let mut nodes_multi = Arc::new(AtomicUsize::new(0));
    let mut ways_multi = Arc::new(AtomicUsize::new(0));
    let mut relations_multi = Arc::new(AtomicUsize::new(0));

    let mut nodes_multi_clone = nodes_multi.clone();
    let mut ways_multi_clone = ways_multi.clone();
    let mut relations_multi_clone = relations_multi.clone();


    group.bench_function(
        "Bench osmio multi-threaded",
        |b| {
            b.iter(|| {
                let nodes_multi_clone = nodes_multi.clone();
                let ways_multi_clone = ways_multi.clone();
                let relations_multi_clone = relations_multi.clone();

                reader.parallel_for_each(18, move |element| {
                    match element {
                        Node { node } => {
                            // get node ide
                            let node_id = node.id();
                            let x = node.coordinate();
                            nodes_multi_clone.fetch_add(1, Ordering::SeqCst);
                        }
                        Element::Way { way } => {
                            let way = way.tags();
                            ways_multi_clone.fetch_add(1, Ordering::SeqCst);
                        }
                        Element::Relation { relation } => {
                            let x1 = relation.tags();
                            relations_multi_clone.fetch_add(1, Ordering::SeqCst);
                        }
                        Element::Sentinel => {}
                    }
                    Ok(())
                }).unwrap();
            });
        },
    );

    println!("Results from multi-threaded");
    println!("nodes: {}", nodes_multi_clone.load(Ordering::SeqCst));
    println!("ways: {}", ways_multi_clone.load(Ordering::SeqCst));
    println!("relations: {}", relations_multi_clone.load(Ordering::SeqCst));

    group.finish();
}
criterion_group! {
    name = benches;
    config = Criterion::default().sample_size(20);
    targets = bench_count
}
criterion_main!(benches);
