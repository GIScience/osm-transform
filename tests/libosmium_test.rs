use std::env;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use libosmium::{Handler, ItemBuffer, ItemRef, Node, Way};
use libosmium::handler::Relation;

#[derive(Debug)]
struct NodeBuffer {
    nodes: ItemBuffer,
    ways: ItemBuffer,
    relations: ItemBuffer,
}

impl Handler for NodeBuffer {
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


#[test]
fn test_libosmium() {
    let file = "test/karlsruhe-regbez-latest.osm.pbf";
    println!("Current file name: {:?}", file);
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
    println!("TEST-1");


    println!("TEST0");
    let mut handler = NodeBuffer {
        nodes: ItemBuffer::new(),
        ways: Default::default(),
        relations: Default::default(),
    };
    println!("TEST1");

    handler
        .apply(&file)
        .map_err(|cstr| cstr.to_string_lossy().to_string()).unwrap();

    println!("TEST2");
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


    println!("Results from libosmium benchmark");
    println!("nodes: {}", nodes_single_clone.load(Ordering::SeqCst));
    println!("ways: {}", ways_single_clone.load(Ordering::SeqCst));
    println!("relations: {}", relations_single_clone.load(Ordering::SeqCst));
}