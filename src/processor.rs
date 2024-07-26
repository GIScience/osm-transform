use std::collections::{BTreeMap, HashMap, HashSet};

use bit_vec::BitVec;
use osm_io::osm::model::element::Element;
use osm_io::osm::model::node::Node;
use osm_io::osm::model::relation::Relation;
use osm_io::osm::model::way::Way;

use crate::handler::OsmElementTypeSelection;

const HIGHEST_NODE_ID: i64 = 50_000_000_000; //todo make configurable

pub trait Processor {
    fn handle(&mut self, element: Element) -> Vec<Element> {
        vec![element]
    }

    fn flush(&mut self, elements: Vec<Element>) -> Vec<Element> {
        elements
    }

    fn add_result(&mut self, result: HandlerResult) -> HandlerResult {
        result
    }
}


#[derive(Debug)]
pub struct HandlerResult {
    pub counts: BTreeMap<String, i32>,
    pub other: HashMap<String, String>,
    pub node_ids: BitVec,
}
impl HandlerResult {
    pub(crate) fn default() -> Self {
        Self::with_capacity(HIGHEST_NODE_ID as usize)
    }
    fn with_capacity(nbits: usize) -> Self {
        HandlerResult {
            counts: btreemap! {},
            other: hashmap! {},
            node_ids: BitVec::from_elem(nbits, false),
        }
    }
    pub fn to_string(&mut self) -> String {
        format!("HandlerResult:\n  {:?}\n  {:?}", &self.counts, &self.other)
    }
}


#[derive(Default)]
pub(crate) struct ProcessorChain {
    pub processors: Vec<Box<dyn Processor>>,
}
impl ProcessorChain {
    pub(crate) fn add_processor(mut self, processor: impl Processor + Sized + 'static) -> ProcessorChain {
        self.processors.push(Box::new(processor));
        self
    }
    pub(crate) fn process(&mut self, element: Element) {
        let mut elements = vec![element];
        for processor in &mut self.processors {
            let mut new_collected = vec![];
            for node in elements {
                new_collected.append(&mut processor.handle(node));
            }
            elements = new_collected;
        }
    }

    pub(crate) fn flush(&mut self, mut elements: Vec<Element>) {
        for processor in &mut self.processors {
            let new_collected = processor.flush(elements.clone());
            elements = new_collected;
        }
    }
    pub(crate) fn collect_result(&mut self) -> HandlerResult {
        let mut result = HandlerResult::default();
        for processor in &mut self.processors {
            result = processor.add_result(result);
        }
        result
    }
}


#[derive(Debug)]
pub(crate) enum CountType {
    ALL,
    ACCEPTED,
}


pub(crate) struct ElementCounter {
    pub nodes_count: i32,
    pub ways_count: i32,
    pub relations_count: i32,
    pub result_key: String,
}
impl ElementCounter {
    pub fn new(result_key: &str) -> Self {
        Self {
            nodes_count: 0,
            ways_count: 0,
            relations_count: 0,
            result_key: result_key.to_string(),
        }
    }
}
impl Processor for ElementCounter {
    fn handle(&mut self, element: Element) -> Vec<Element> {
        match element {
            Element::Node { .. } => { self.nodes_count += 1; }
            Element::Way { .. } => { self.ways_count += 1; }
            Element::Relation { .. } => { self.relations_count += 1; }
            Element::Sentinel => {}
        }
        vec![element]
    }
    fn add_result(&mut self, mut result: HandlerResult) -> HandlerResult {
        result.counts.insert(format!("nodes count {}", self.result_key), self.nodes_count);
        result.counts.insert(format!("ways count {}", self.result_key), self.ways_count);
        result.counts.insert(format!("relations count {}", self.result_key), self.relations_count);
        result
    }
}


pub(crate) struct ElementPrinter {
    pub prefix: String,
    pub node_ids: HashSet<i64>,
    pub way_ids: HashSet<i64>,
    pub relation_ids: HashSet<i64>,
    pub handle_types: OsmElementTypeSelection,
}
impl Default for ElementPrinter {
    fn default() -> Self {
        Self {
            prefix: "".to_string(),
            node_ids: HashSet::new(),
            way_ids: HashSet::new(),
            relation_ids: HashSet::new(),
            handle_types: OsmElementTypeSelection::none(),
        }
    }
}
impl ElementPrinter {
    pub fn with_prefix(prefix: String) -> Self {
        Self {
            prefix: prefix,
            ..Self::default()
        }
    }
    pub(crate) fn with_node_ids(mut self, node_ids: HashSet<i64>) -> Self {
        for id in node_ids {
            self.node_ids.insert(id);
            self.handle_types.node = true;
        }
        self
    }
    pub(crate) fn with_way_ids(mut self, way_ids: HashSet<i64>) -> Self {
        for id in way_ids {
            self.way_ids.insert(id);
            self.handle_types.way = true;
        }
        self
    }
    pub(crate) fn with_relation_ids(mut self, relation_ids: HashSet<i64>) -> Self {
        for id in relation_ids {
            self.relation_ids.insert(id);
            self.handle_types.relation = true;
        }
        self
    }
    pub(crate) fn print_node(mut self, id: i64) -> Self {
        self.handle_types.node = true;
        self.node_ids.insert(id);
        self
    }
    pub(crate) fn print_way(mut self, id: i64) -> Self {
        self.handle_types.way = true;
        self.way_ids.insert(id);
        self
    }
    pub(crate) fn print_relation(mut self, id: i64) -> Self {
        self.handle_types.relation = true;
        self.relation_ids.insert(id);
        self
    }

    fn handle_node(&mut self, node: &Node) {
        if self.handle_types.node && self.node_ids.contains(&node.id()) {
            println!("{}node {} visible: {}", &self.prefix, &node.id(), &node.visible());
            println!("  version:    {}", &node.version());
            println!("  coordinate: lat,lon = {},{}", &node.coordinate().lat(), &node.coordinate().lon());
            println!("  changeset:  {}", &node.changeset());
            println!("  timestamp:  {}", &node.timestamp());
            println!("  uid:        {}", &node.uid());
            println!("  user:       {}", &node.user());
            println!("  tags:");
            for tag in node.tags() {
                println!("   '{}' = '{}'", &tag.k(), &tag.v())
            }
        }
    }
    fn handle_way(&mut self, way: &Way) {
        if self.handle_types.way && self.way_ids.contains(&way.id()) {
            println!("{}way {} visible: {}", &self.prefix, &way.id(), &way.visible());
            println!("  version:   {}", &way.version());
            println!("  changeset: {}", &way.changeset());
            println!("  timestamp: {}", &way.timestamp());
            println!("  uid:       {}", &way.uid());
            println!("  user:      {}", &way.user());
            println!("  tags:");
            for tag in way.tags() {
                println!("   '{}' = '{}'", &tag.k(), &tag.v())
            }
            println!("  refs:");
            for id in way.refs() {
                println!("   {}", &id)
            }
        }
    }
    fn handle_relation(&mut self, relation: &Relation) {
        if self.handle_types.relation && self.relation_ids.contains(&relation.id()) {
            println!("{}relation {} visible: {}", &self.prefix, &relation.id(), &relation.visible());
            println!("  version:   {}", &relation.version());
            println!("  changeset: {}", &relation.changeset());
            println!("  timestamp: {}", &relation.timestamp());
            println!("  uid:       {}", &relation.uid());
            println!("  user:      {}", &relation.user());
            println!("  tags:");
            for tag in relation.tags() {
                println!("   '{}' = '{}'", &tag.k(), &tag.v())
            }
            println!("  members:");
            for member in relation.members() {
                println!("   {:?}", &member)
            }
        }
    }
}
impl Processor for ElementPrinter {
    fn handle(&mut self, element: Element) -> Vec<Element> {
        match element {
            Element::Node { node } => {
                self.handle_node(&node);
                vec![Element::Node { node }]
            }
            Element::Way { way } => {
                self.handle_way(&way);
                vec![Element::Way { way }]
            }
            Element::Relation { relation } => {
                self.handle_relation(&relation);
                vec![Element::Relation { relation }]
            }
            Element::Sentinel => { vec![] }
        }
    }
}


#[cfg(test)]
mod tests {
    use bit_vec::BitVec;
    use osm_io::osm::model::coordinate::Coordinate;
    use osm_io::osm::model::element::Element;
    use osm_io::osm::model::node::Node;
    use osm_io::osm::model::relation::Relation;
    use osm_io::osm::model::tag::Tag;
    use osm_io::osm::model::way::Way;

    use crate::processor::{ElementCounter, ElementPrinter, HandlerResult, Processor, ProcessorChain};

    fn existing_tag() -> String { "EXISTING_TAG".to_string() }
    fn missing_tag() -> String { "MISSING_TAG".to_string() }
    fn node_element(
        id: i64,
        version: i32,
        coordinate: Coordinate,
        timestamp: i64,
        changeset: i64,
        uid: i32,
        user: String,
        visible: bool,
        tags: Vec<Tag>,
    ) -> Element {
        Element::Node { node: Node::new(id, version, coordinate, timestamp, changeset, uid, user, visible, tags) }
    }

    pub(crate) struct ElementModifier;  //modify element and return same instance
    pub(crate) struct ElementExchanger; //return a copy of the element
    pub(crate) struct ElementFilter;    //remove an element / return empty vec
    pub(crate) struct ElementAdder;     //receive one element, return two
    pub(crate) struct ElementMixedAdder; //receive one way, return a way and two nodes


    #[derive(Default, Debug)]
    pub(crate) struct TestOnlyElementBuffer { //store received elements, when receiving the 5th, emit all 5 and start buffering again. flush: emit currently buffered. handling the elements (changing) happens before emitting
        nodes: Vec<Node>,
        ways: Vec<Way>,
        relations: Vec<Relation>,
    }
    impl TestOnlyElementBuffer {
        fn flush_nodes(&mut self) -> Vec<Element> {
            let result = self.nodes.iter().map(|node| Element::Node {node: node.clone()}).collect();
            self.nodes.clear();
            result
        }
        fn flush_ways(&mut self) -> Vec<Element> {
            let result = self.ways.iter().map(|way| Element::Way {way: way.clone()}).collect();
            self.ways.clear();
            result
        }
        fn flush_relations(&mut self) -> Vec<Element> {
            let result = self.relations.iter().map(|relation| Element::Relation {relation: relation.clone()}).collect();
            self.relations.clear();
            result
        }
        fn handle_node(&mut self, node: Node) -> Node {
            //todo pass to configured fn/closure or something
            node
        }
    }
    impl Processor for TestOnlyElementBuffer {
        fn handle(&mut self, element: Element) -> Vec<Element> {
            match element {
                Element::Node { node } => {
                    self.nodes.push(node);
                    if self.nodes.len() >= 3 {
                        return self.flush_nodes();
                    }
                    vec![]
                }
                Element::Way { way } => {
                    self.ways.push(way);
                    if self.ways.len() >= 3 {
                        return self.flush_ways();
                    }
                    vec![]
                }
                Element::Relation { relation } => {
                    self.relations.push(relation);
                    if self.relations.len() >= 3 {
                        return self.flush_relations();
                    }
                    vec![]
                }
                Element::Sentinel => { vec![] }
            }
        }
        fn flush(&mut self, elements: Vec<Element>) -> Vec<Element> {
            for element in elements {
                match element {
                    Element::Node { node } => { self.nodes.push(node); }
                    Element::Way { way } => { self.ways.push(way); }
                    Element::Relation { relation } => { self.relations.push(relation); }
                    Element::Sentinel => {}
                }
            }
            let mut flushed = vec![];
            flushed.append(&mut self.flush_nodes());
            flushed.append(&mut self.flush_ways());
            flushed.append(&mut self.flush_relations());
            flushed
        }
    }
    #[derive(Debug)]
    pub(crate) struct TestOnlyIdCollector {
        pub node_ids: BitVec,
        pub way_ids: BitVec,
        pub relation_ids: BitVec,
    }
    impl TestOnlyIdCollector {
        pub fn new(nbits: usize) -> Self {
            TestOnlyIdCollector {
                node_ids: BitVec::from_elem(nbits, false),
                way_ids: BitVec::from_elem(nbits, false),
                relation_ids: BitVec::from_elem(nbits, false),
            }
        }
    }
    impl Processor for TestOnlyIdCollector {
        fn handle(&mut self, element: Element) -> Vec<Element> {
            match element {
                Element::Node { ref node } => { self.node_ids.set(node.id() as usize, true); }
                Element::Way { ref way } => { self.node_ids.set(way.id() as usize, true); }
                Element::Relation { ref relation } => { self.node_ids.set(relation.id() as usize, true); }
                Element::Sentinel => {}
            }
            vec![element]
        }
        fn add_result(&mut self, mut result: HandlerResult) -> HandlerResult {
            result.node_ids = self.node_ids.clone();
            result
        }
    }


    pub(crate) struct TestOnlyOrderRecorder {
        pub received_ids: Vec<String>,
        pub result_key: String,
    }
    impl TestOnlyOrderRecorder {
        pub fn new(result_key: &str) -> Self {
            Self {
                received_ids: vec![],
                result_key: result_key.to_string(),
            }
        }
    }
    impl Processor for TestOnlyOrderRecorder {
        fn handle(&mut self, element: Element) -> Vec<Element> {
            match &element {
                Element::Node { node } => { self.received_ids.push(format!("node#{}", node.id().to_string())); }
                Element::Way { way } => { self.received_ids.push(format!("way#{}", way.id().to_string())); }
                Element::Relation { relation } => { self.received_ids.push(format!("relation#{}", relation.id().to_string())); }
                Element::Sentinel => {}
            }
            vec![element]
        }
        fn add_result(&mut self, mut result: HandlerResult) -> HandlerResult {
            result.other.insert(format!("{}", self.result_key), self.received_ids.join(", "));
            result
        }
    }


    #[test]
    fn test_chain() {
        let captor = TestOnlyElementBuffer::default();
        let id_collector = TestOnlyIdCollector::new(10);
        let mut processor_chain = ProcessorChain::default()
            .add_processor(TestOnlyOrderRecorder::new("1_initial"))
            .add_processor(id_collector)
            .add_processor(ElementPrinter::with_prefix("ElementPrinter final: ".to_string()).with_node_ids(hashset! {8}))
            .add_processor(TestOnlyOrderRecorder::new("9_final"))
            ;
        processor_chain.process(node_element(1, 1, Coordinate::new(1.0f64, 1.1f64), 1, 1, 1, "a".to_string(), true, vec![Tag::new(existing_tag(), "kasper".to_string())]));
        processor_chain.process(node_element(2, 1, Coordinate::new(2.0f64, 1.2f64), 1, 1, 1, "a".to_string(), true, vec![Tag::new(existing_tag(), "seppl".to_string())]));
        processor_chain.process(node_element(6, 1, Coordinate::new(3.0f64, 1.3f64), 1, 1, 1, "a".to_string(), true, vec![Tag::new(existing_tag(), "hotzenplotz".to_string())]));
        processor_chain.process(node_element(8, 1, Coordinate::new(4.0f64, 1.4f64), 1, 1, 1, "a".to_string(), true, vec![Tag::new(existing_tag(), "großmutter".to_string())]));
        processor_chain.flush(vec![]);
        let result = processor_chain.collect_result();
        dbg!(result);
    }

    #[test]
    fn test_chain_with_buffer() {
        let captor = TestOnlyElementBuffer::default();
        let id_collector = TestOnlyIdCollector::new(10);
        let mut processor_chain = ProcessorChain::default()
            .add_processor(TestOnlyOrderRecorder::new("1_initial"))
            .add_processor(TestOnlyElementBuffer::default())
            .add_processor(id_collector)
            .add_processor(ElementPrinter::with_prefix("ElementPrinter final: ".to_string()).with_node_ids(hashset! {1,2,6,8}))
            .add_processor(TestOnlyOrderRecorder::new("9_final"))
            ;
        processor_chain.process(node_element(1, 1, Coordinate::new(1.0f64, 1.1f64), 1, 1, 1, "a".to_string(), true, vec![Tag::new(existing_tag(), "kasper".to_string())]));
        processor_chain.process(node_element(2, 1, Coordinate::new(2.0f64, 1.2f64), 1, 1, 1, "a".to_string(), true, vec![Tag::new(existing_tag(), "seppl".to_string())]));
        processor_chain.process(node_element(6, 1, Coordinate::new(3.0f64, 1.3f64), 1, 1, 1, "a".to_string(), true, vec![Tag::new(existing_tag(), "hotzenplotz".to_string())]));
        processor_chain.process(node_element(8, 1, Coordinate::new(4.0f64, 1.4f64), 1, 1, 1, "a".to_string(), true, vec![Tag::new(existing_tag(), "großmutter".to_string())]));
        processor_chain.flush(vec![]);
        let result = processor_chain.collect_result();
        dbg!(result);
    }
}
