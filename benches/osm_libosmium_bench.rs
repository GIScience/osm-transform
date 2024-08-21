use std::env;
use std::ffi::CString;
use std::fmt::Pointer;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use criterion::{criterion_group, criterion_main, Criterion};
use libosmium::{Handler, ItemBuffer, ItemRef, Node, Way};
use libosmium::handler::Relation;

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

fn bench_libosmium(c: &mut Criterion) {
    let mut group = c.benchmark_group("OSMIO Benchmark");

    let file = "/home/jules/HeiGIT/repos/rust-parquet-osm-transform/benches/files/karlsruhe-regbez-latest.osm.pbf";
    println!("Current file name: {:?}", file);
    let nodes_single = Arc::new(AtomicUsize::new(0));
    let ways_single = Arc::new(AtomicUsize::new(0));
    let relations_single = Arc::new(AtomicUsize::new(0));

    let nodes_single_clone = nodes_single.clone();
    let ways_single_clone = ways_single.clone();
    let relations_single_clone = relations_single.clone();

    group.bench_function(
        "Bench libosmium node only",
        |b| {
            b.iter(|| {
                let mut handler = NodeBuffer {
                    nodes: ItemBuffer::new(),
                };

                handler
                    .apply(&file)
                    .map_err(|cstr| cstr.to_string_lossy().to_string()).unwrap();

                for node in handler.nodes.iter() {
                    if let Some(ItemRef::Node(node)) = node.cast() {
                        if !node.tags().is_empty() {
                            nodes_single_clone.fetch_add(1, Ordering::SeqCst);
                        }
                    } else {
                        unreachable!("The buffer was only populated with nodes");
                    }
                }
            });
        },
    );

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
                            let x = node.tags();
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

    group.bench_function(
        "Bench libosmium ways apply_with_ways ",
        |b| {
            b.iter(|| {
                let mut handler = WayBuffer {
                    ways: Default::default(),
                };

                handler
                    .apply_with_ways(&file)
                    .map_err(|cstr| cstr.to_string_lossy().to_string()).unwrap();

                for way in handler.ways.iter() {
                    if let Some(ItemRef::Way(way)) = way.cast() {
                        if !way.tags().is_empty() {
                            let way_id = way.id();
                            way.nodes().iter().for_each(|node| {
                                let node_id = node.id;
                            });
                            way.tags().into_iter().for_each(|(key, value)| {
                                let key = key.to_string();
                                let value = value.to_string();
                            });
                            ways_single_clone.fetch_add(1, Ordering::SeqCst);
                        }
                    } else {
                        unreachable!("The buffer was only populated with ways");
                    }
                }
            });
        },
    );

    group.bench_function(
        "Bench libosmium ways and relations apply_with_ways ",
        |b| {
            b.iter(|| {
                let mut handler = WayRelBuffer {
                    ways: Default::default(),
                    relations: Default::default(),
                };

                handler
                    .apply_with_ways(&file)
                    .map_err(|cstr| cstr.to_string_lossy().to_string()).unwrap();

                for way in handler.ways.iter() {
                    if let Some(ItemRef::Way(way)) = way.cast() {
                        if !way.tags().is_empty() {
                            ways_single_clone.fetch_add(1, Ordering::SeqCst);
                        }
                    } else {
                        unreachable!("The buffer was only populated with ways");
                    }
                }

                for relation in handler.relations.iter() {
                    if let Some(ItemRef::Relation(relation)) = relation.cast() {
                        if !relation.tags().is_empty() {
                            relations_single_clone.fetch_add(1, Ordering::SeqCst);
                        }
                    } else {
                        unreachable!("The buffer was only populated with relations");
                    }
                }
            });
        },
    );

    group.bench_function(
        "Bench libosmium all apply_with_ways ",
        |b| {
            b.iter(|| {
                let mut handler = AllBuffer {
                    nodes: ItemBuffer::new(),
                    ways: Default::default(),
                    relations: Default::default(),
                };

                handler
                    .apply_with_ways(&file)
                    .map_err(|cstr| cstr.to_string_lossy().to_string()).unwrap();

                for node in handler.nodes.iter() {
                    if let Some(ItemRef::Node(node)) = node.cast() {
                        if !node.tags().is_empty() {
                            nodes_single_clone.fetch_add(1, Ordering::SeqCst);
                        }
                    } else {
                        unreachable!("The buffer was only populated with nodes");
                    }
                }

                for way in handler.ways.iter() {
                    if let Some(ItemRef::Way(way)) = way.cast() {
                        if !way.tags().is_empty() {
                            ways_single_clone.fetch_add(1, Ordering::SeqCst);
                        }
                    } else {
                        unreachable!("The buffer was only populated with ways");
                    }
                }

                for relation in handler.relations.iter() {
                    if let Some(ItemRef::Relation(relation)) = relation.cast() {
                        if !relation.tags().is_empty() {
                            relations_single_clone.fetch_add(1, Ordering::SeqCst);
                        }
                    } else {
                        unreachable!("The buffer was only populated with relations");
                    }
                }
            });
        },
    );
    group.finish();
}

criterion_group! {
    name = benches;
    config = Criterion::default().sample_size(10);
    targets = bench_libosmium
}
criterion_main!(benches);