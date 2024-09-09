use std::env;
use std::ffi::CString;
use std::fmt::Pointer;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use criterion::{criterion_group, criterion_main, Criterion};
use libosmium::{Handler, ItemBuffer, ItemRef, Node, Way};
use libosmium::handler::Relation;
use osm_io::osm::model::element::Element;
use osm_io::osm::pbf::reader::Reader as PbfReader;

#[derive(Debug)]
struct AllBuffer {
    ways: ItemBuffer,
    relations: ItemBuffer,
    nodes: ItemBuffer,
}

#[derive(Debug)]
struct WayRelBuffer {
    ways: ItemBuffer,
    relations: ItemBuffer,
}

#[derive(Debug)]
struct WayBuffer {
    ways: ItemBuffer,
}


#[derive(Debug)]
struct NodeBuffer {
    nodes: ItemBuffer,
}

impl Handler for WayRelBuffer {
    fn relation(&mut self, relation: &Relation) {
        self.relations.push(relation);
    }

    fn way(&mut self, way: &Way) {
        self.ways.push(way);
    }
}

impl Handler for NodeBuffer
{
    fn node(&mut self, node: &Node) {
        self.nodes.push(node);
    }
}

impl Handler for WayBuffer {
    fn way(&mut self, way: &Way) {
        self.ways.push(way);
    }
}

impl Handler for AllBuffer {
    fn node(&mut self, node: &Node) {
        self.nodes.push(node);
    }
    fn relation(&mut self, relation: &Relation) {
        self.relations.push(relation);
    }

    fn way(&mut self, way: &Way) {
        self.ways.push(way);
    }
}

fn bench_libosmium_vs_osmio(c: &mut Criterion) {
    let mut group = c.benchmark_group("OSMIO Benchmark");

    let file = "test/karlsruhe-regbez-latest.osm.pbf";
    println!("Current file name: {:?}", file);
    let nodes_single = Arc::new(AtomicUsize::new(0));
    let ways_single = Arc::new(AtomicUsize::new(0));
    let relations_single = Arc::new(AtomicUsize::new(0));

    let nodes_single_clone = nodes_single.clone();
    let ways_single_clone = ways_single.clone();
    let relations_single_clone = relations_single.clone();

    group.bench_function(
        "Bench libosmium all apply",
        |b| {
            b.iter(|| {
                let mut handler = AllBuffer {
                    ways: Default::default(),
                    relations: Default::default(),
                    nodes: Default::default(),
                };

                handler
                    .apply(&file)
                    .map_err(|cstr| cstr.to_string_lossy().to_string()).unwrap();

                for node in handler.nodes.iter() {
                    if let Some(ItemRef::Node(node)) = node.cast() {
                        if !node.tags().is_empty() {
                            let location = node.location();
                            let x = node.id();
                            nodes_single_clone.fetch_add(1, Ordering::SeqCst);
                        }
                    } else {
                        unreachable!("The buffer was only populated with nodes");
                    }
                }

                for way in handler.ways.iter() {
                    if let Some(ItemRef::Way(way)) = way.cast() {
                        if !way.tags().is_empty() {
                            let x1 = way.nodes();
                            let x2 = way.tags();
                            ways_single_clone.fetch_add(1, Ordering::SeqCst);
                        }
                    } else {
                        unreachable!("The buffer was only populated with ways");
                    }
                }

                for relation in handler.relations.iter() {
                    if let Some(ItemRef::Relation(relation)) = relation.cast() {
                        if !relation.tags().is_empty() {
                            let x1 = relation.tags();
                            relations_single_clone.fetch_add(1, Ordering::SeqCst);
                        }
                    } else {
                        unreachable!("The buffer was only populated with relations");
                    }
                }
            });
        },
    );
    let file_path = PathBuf::from(file);

    let reader = PbfReader::new(&file_path).unwrap();

    let nodes_multi = Arc::new(AtomicUsize::new(0));
    let ways_multi = Arc::new(AtomicUsize::new(0));
    let relations_multi = Arc::new(AtomicUsize::new(0));

    let nodes_multi_clone = nodes_multi.clone();
    let ways_multi_clone = ways_multi.clone();
    let relations_multi_clone = relations_multi.clone();


    group.bench_function(
        "Bench osmio multi-threaded",
        |b| {
            b.iter(|| {
                let nodes_multi_clone = nodes_multi.clone();
                let ways_multi_clone = ways_multi.clone();
                let relations_multi_clone = relations_multi.clone();

                reader.parallel_for_each(18, move |element| {
                    match element {
                        osm_io::osm::model::element::Element::Node { node } => {
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
    println!("Results from libosmium benchmark");
    println!("nodes: {}", nodes_single_clone.load(Ordering::SeqCst));
    println!("ways: {}", ways_single_clone.load(Ordering::SeqCst));
    println!("relations: {}", relations_single_clone.load(Ordering::SeqCst));
    

    println!("Results from osmio multi-threaded");
    println!("nodes: {}", nodes_multi_clone.load(Ordering::SeqCst));
    println!("ways: {}", ways_multi_clone.load(Ordering::SeqCst));
    println!("relations: {}", relations_multi_clone.load(Ordering::SeqCst));

    group.finish();
}

criterion_group! {
    name = benches;
    config = Criterion::default().sample_size(10);
    targets = bench_libosmium_vs_osmio
}
criterion_main!(benches);