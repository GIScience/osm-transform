use std::collections::{BTreeMap, HashMap, HashSet};

use bit_vec::BitVec;
use osm_io::osm::model::element::Element;
use osm_io::osm::model::node::Node;
use osm_io::osm::model::relation::Relation;
use osm_io::osm::model::way::Way;

use crate::handler::OsmElementTypeSelection;

const HIGHEST_NODE_ID: i64 = 50_000_000_000; //todo make configurable

pub fn format_element_id(element: &Element) -> String {
    match &element {
        Element::Node { node } => { format!("node#{}", node.id().to_string()) }
        Element::Way { way } => { format!("way#{}", way.id().to_string()) }
        Element::Relation { relation } => { format!("relation#{}", relation.id().to_string()) }
        Element::Sentinel => {"sentinel#!".to_string()}
    }
}
pub trait Processor {

    fn name(&self) -> String;

    fn handle_element(&mut self, element: Element) -> Vec<Element> {
        vec![element]
    }

    fn handle_and_flush_elements(&mut self, elements: Vec<Element>) -> Vec<Element> {
        let mut handeled = vec![];
        for element in elements {
            handeled.append(&mut self.handle_element(element));
        }
        handeled
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
        log::trace!("######");
        log::trace!("###### Processing {}", format_element_id(&element));
        log::trace!("######");
        let mut elements = vec![element];
        let mut indent = "".to_string();
        for processor in &mut self.processors {
            if (elements.len() == 0) {
                log::trace!("{indent}Skipping processor chain, elements were filtered or buffered?");
                break
            }
            let mut new_collected = vec![];
            for inner_element in elements {
                log::trace!("{indent}Passing {} to processor {}", format_element_id(&inner_element), processor.name());
                let handled_elements = &mut processor.handle_element(inner_element);
                log::trace!("{indent}{} returned {} elements", processor.name(), handled_elements.len());
                new_collected.append(handled_elements);
            }
            log::trace!("{indent}{} returned {} elements in total", processor.name(), new_collected.len());
            elements = new_collected;
            indent += "    ";
        }
    }

    pub(crate) fn flush(&mut self, mut elements: Vec<Element>) {
        for processor in &mut self.processors {
            log::trace!("######");
            log::trace!("###### Flushing {} with {} elements flushed by upstream processors", processor.name(), elements.len());
            log::trace!("######");
            //todo find solution without clone. but flushing is done only once, so it's not THAT important
            let new_collected = processor.handle_and_flush_elements(elements.clone());
            if new_collected.len() > 0 {
                log::trace!("  {} returned {} flushed elements", processor.name(), new_collected.len())
            }
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
    fn name(&self) -> String { format!("ElementCounter {}", self.result_key) }
    fn handle_element(&mut self, element: Element) -> Vec<Element> {
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
            println!("{}: node {} visible: {}", &self.prefix, &node.id(), &node.visible());
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
            println!("{}: way {} visible: {}", &self.prefix, &way.id(), &way.visible());
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
            println!("{}: relation {} visible: {}", &self.prefix, &relation.id(), &relation.visible());
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
    fn name(&self) -> String { format!("ElementPrinter {}", self.prefix) }
    fn handle_element(&mut self, element: Element) -> Vec<Element> {
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
    use std::ops::Add;
    use bit_vec::BitVec;
    use log4rs::append::Append;
    use osm_io::osm::model::coordinate::Coordinate;
    use osm_io::osm::model::element::Element;
    use osm_io::osm::model::node::Node;
    use osm_io::osm::model::relation::{Member, MemberData, Relation};
    use osm_io::osm::model::tag::Tag;
    use osm_io::osm::model::way::Way;
    use simple_logger::SimpleLogger;
    use crate::processor::{ElementCounter, ElementPrinter, format_element_id, HandlerResult, Processor, ProcessorChain};

    fn existing_tag() -> String { "EXISTING_TAG".to_string() }
    fn missing_tag() -> String { "MISSING_TAG".to_string() }
    pub enum MemberType { Node, Way, Relation }
    fn simple_node_element(id: i64, tags: Vec<(&str, &str)>) -> Element {
        let tags_obj = tags.iter().map(|(k, v)| Tag::new(k.to_string(), v.to_string())).collect();
        node_element(id, 1, Coordinate::new(1.0f64, 1.1f64), 1, 1, 1, "a".to_string(), true, tags_obj)
    }
    fn simple_way_element(id: i64, refs: Vec<i64>, tags: Vec<(&str, &str)>) -> Element {
        let tags_obj = tags.iter().map(|(k, v)| Tag::new(k.to_string(), v.to_string())).collect();
        way_element(id, 1, 1, 1, 1, "a_user".to_string(), true, refs, tags_obj)
    }
    fn simple_relation_element(id: i64, members: Vec<(MemberType, i64, &str)>, tags: Vec<(&str, &str)>) -> Element {
        let members_obj = members.iter().map(|(t, id, role)| {
            match t {
                MemberType::Node => { Member::Node { member: MemberData::new(id.clone(), role.to_string()) } }
                MemberType::Way => { Member::Way { member: MemberData::new(id.clone(), role.to_string()) } }
                MemberType::Relation => { Member::Relation { member: MemberData::new(id.clone(), role.to_string()) } }
            }
        }).collect();
        let tags_obj = tags.iter().map(|(k, v)| Tag::new(k.to_string(), v.to_string())).collect();
        relation_element(id, 1, 1, 1, 1, "a_user".to_string(), true, members_obj, tags_obj)
    }
    fn node_element(id: i64, version: i32, coordinate: Coordinate, timestamp: i64, changeset: i64, uid: i32, user: String, visible: bool, tags: Vec<Tag>) -> Element {
        Element::Node { node: Node::new(id, version, coordinate, timestamp, changeset, uid, user, visible, tags) }
    }
    fn way_element(id: i64, version: i32, timestamp: i64, changeset: i64, uid: i32, user: String, visible: bool, refs: Vec<i64>, tags: Vec<Tag>) -> Element {
        Element::Way { way: Way::new(id, version, timestamp, changeset, uid, user, visible, refs, tags) }
    }
    fn relation_element(id: i64, version: i32, timestamp: i64, changeset: i64, uid: i32, user: String, visible: bool, members: Vec<Member>, tags: Vec<Tag>) -> Element {
        Element::Relation { relation: Relation::new(id, version, timestamp, changeset, uid, user, visible, members, tags) }
    }
    fn copy_node_with_new_id(node: &Node, new_id: i64) -> Node {
        Node::new(new_id, node.version(), node.coordinate().clone(), node.timestamp(), node.changeset(), node.uid(), node.user().clone(), node.visible(), node.tags().clone())
    }
    ///Modify element and return same instance.
    pub(crate) struct ElementModifier; //TODO implement
    ///Return a copy of the element.
    pub(crate) struct ElementExchanger; //TODO implement
    ///Remove an element / return empty vec.
    pub(crate) struct ElementFilter; //TODO implement
    ///Receive one element, return two.
    pub(crate) struct ElementAdder; //TODO implement

    ///Receive one way, return a way and a new node for each ref of the way.
    #[derive(Debug, Default)]
    pub(crate) struct TestOnlyElementMixedAdder;
    impl Processor for TestOnlyElementMixedAdder {
        fn name(&self) -> String { "TestOnlyElementMixedAdder".to_string() }
        fn handle_element(&mut self, element: Element) -> Vec<Element> {
            match element {
                Element::Node { .. } => { vec![element] }
                Element::Way { way } => {
                    let mut elements: Vec<Element> = way.refs().iter().map(|id| simple_node_element(id.clone(), vec![("added", "by processor")])).collect();
                    elements.push(Element::Way { way });
                    elements
                }
                Element::Relation { .. } => { vec![element] }
                Element::Sentinel => { vec![] }
            }
        }
    }


    #[derive(Default, Debug)]
    pub(crate) struct TestOnlyElementBufferingDuplicatingEditingProcessor { //store received elements, when receiving the 5th, emit all 5 and start buffering again. flush: emit currently buffered. handling the elements (changing) happens before emitting
        nodes: Vec<Node>,
        ways: Vec<Way>,
        relations: Vec<Relation>,
    }
    impl TestOnlyElementBufferingDuplicatingEditingProcessor {
        fn handle_and_flush_nodes(&mut self) -> Vec<Element> {
            let mut handled_nodes = vec![];
            for node in &self.nodes {
                handled_nodes.extend(self.handle_node(node.clone()));
            }
            let mut flush_nodes = vec![];
            for node in handled_nodes {
                flush_nodes.push(Element::Node { node })
            }
            self.nodes.clear();
            flush_nodes
        }
        fn handle_and_flush_ways(&mut self) -> Vec<Element> {
            //TODO add the tricky part to also change, duplicate, etc. the buffered elements (at this point, elevation processor would do its job
            let result = self.ways.iter().map(|way| Element::Way { way: way.clone() }).collect();
            self.ways.clear();
            result
        }
        fn handle_and_flush_relations(&mut self) -> Vec<Element> {
            //TODO add the tricky part to also change, duplicate, etc. the buffered elements (at this point, elevation processor would do its job
            let result = self.relations.iter().map(|relation| Element::Relation { relation: relation.clone() }).collect();
            self.relations.clear();
            result
        }
        fn handle_node(&self, node: Node) -> Vec<Node> {
            //todo pass to configured fn/closure or something
            let mut node_clone = copy_node_with_new_id(&node, node.id().clone().add(100));
            node_clone.tags_mut().push(Tag::new("elevation".to_string(), "default-elevation".to_string()));
            vec![node, node_clone] //todo remove the clone, thats just an experiment
        }
    }
    impl Processor for TestOnlyElementBufferingDuplicatingEditingProcessor {
        // fn struct_name() -> &'static str { "TestOnlyElementBuffer" }
        fn name(&self) -> String { "TestOnlyElementBuffer".to_string() }
        fn handle_element(&mut self, element: Element) -> Vec<Element> {
            match element {
                Element::Node { node } => {
                    self.nodes.push(node);
                    if self.nodes.len() >= 3 {
                        return self.handle_and_flush_nodes();
                    }
                    vec![]
                }
                Element::Way { way } => {
                    self.ways.push(way);
                    if self.ways.len() >= 3 {
                        return self.handle_and_flush_ways();
                    }
                    vec![]
                }
                Element::Relation { relation } => {
                    self.relations.push(relation);
                    if self.relations.len() >= 3 {
                        return self.handle_and_flush_relations();
                    }
                    vec![]
                }
                Element::Sentinel => { vec![] }
            }
        }
        fn handle_and_flush_elements(&mut self, elements: Vec<Element>) -> Vec<Element> {
            for element in elements {
                match element {
                    Element::Node { node } => { self.nodes.push(node); }
                    Element::Way { way } => { self.ways.push(way); }
                    Element::Relation { relation } => { self.relations.push(relation); }
                    Element::Sentinel => {}
                }
            }
            let mut flushed = vec![];
            flushed.append(&mut self.handle_and_flush_nodes());
            flushed.append(&mut self.handle_and_flush_ways());
            flushed.append(&mut self.handle_and_flush_relations());
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
        fn name(&self) -> String { "TestOnlyIdCollector".to_string() }
        fn handle_element(&mut self, element: Element) -> Vec<Element> {
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
        fn name(&self) -> String { format!("TestOnlyOrderRecorder {}", self.result_key) }
        fn handle_element(&mut self, element: Element) -> Vec<Element> {
            self.received_ids.push(format_element_id(&element));
            vec![element]
        }
        fn add_result(&mut self, mut result: HandlerResult) -> HandlerResult {
            result.other.insert(format!("{}", self.name()), self.received_ids.join(", "));
            result
        }
    }


    #[test]
    fn test_chain() {
        let id_collector = TestOnlyIdCollector::new(10);
        let mut processor_chain = ProcessorChain::default()
            .add_processor(TestOnlyOrderRecorder::new("1_initial"))
            .add_processor(id_collector)
            .add_processor(ElementPrinter::with_prefix("ElementPrinter final: ".to_string()).with_node_ids(hashset! {8}))
            .add_processor(TestOnlyOrderRecorder::new("9_final"))
            ;
        processor_chain.process(simple_node_element(1, vec![("who", "kasper")]));
        processor_chain.process(simple_node_element(2, vec![("who", "seppl")]));
        processor_chain.process(simple_node_element(6, vec![("who", "hotzenplotz")]));
        processor_chain.process(simple_node_element(8, vec![("who", "großmutter")]));
        processor_chain.flush(vec![]);
        let result = processor_chain.collect_result();
        dbg!(result);
    }

    #[test]
    fn test_chain_with_buffer() {
        SimpleLogger::new().init();
        let mut processor_chain = ProcessorChain::default()
            .add_processor(ElementCounter::new("initial"))
            .add_processor(TestOnlyOrderRecorder::new("initial"))
            .add_processor(TestOnlyElementBufferingDuplicatingEditingProcessor::default())
            .add_processor(TestOnlyIdCollector::new(200))
            .add_processor(ElementPrinter::with_prefix("final".to_string()).with_node_ids((1..=200).collect()))
            .add_processor(TestOnlyOrderRecorder::new("final"))
            .add_processor(ElementCounter::new("final"))
            ;

        processor_chain.process(simple_way_element(23, vec![1, 2, 8, 6], vec![("who", "kasper")]));
        processor_chain.process(simple_node_element(1, vec![("who", "kasper")]));
        processor_chain.process(simple_node_element(2, vec![("who", "seppl")]));
        processor_chain.process(simple_node_element(6, vec![("who", "hotzenplotz")]));
        processor_chain.process(simple_node_element(8, vec![("who", "großmutter")]));
        processor_chain.process(simple_relation_element(66, vec![(MemberType::Way, 23, "kasper&seppl brign großmutter to hotzenplotz")], vec![("who", "großmutter")]));
        processor_chain.flush(vec![]);
        let result = processor_chain.collect_result();
        dbg!(&result);
        assert_eq!(&result.counts.get("nodes count final").unwrap().clone(), &8);
        assert_eq!(&result.counts.get("nodes count initial").unwrap().clone(), &4,);
        assert_eq!(&result.counts.get("relations count final").unwrap().clone(), &1,);
        assert_eq!(&result.counts.get("relations count initial").unwrap().clone(), &1,);
        assert_eq!(&result.counts.get("ways count final").unwrap().clone(), &1,);
        assert_eq!(&result.counts.get("ways count initial").unwrap().clone(), &1,);
        assert_eq!(&result.other.get("TestOnlyOrderRecorder initial").unwrap().clone(), "way#23, node#1, node#2, node#6, node#8, relation#66");
        assert_eq!(&result.other.get("TestOnlyOrderRecorder final").unwrap().clone(), "node#1, node#101, node#2, node#102, node#6, node#106, node#8, node#108, way#23, relation#66");
    }
    #[test]
    fn test_chain_with_mixed_element_adder() {
        SimpleLogger::new().init();
        let mut processor_chain = ProcessorChain::default()
            .add_processor(ElementCounter::new("initial"))
            .add_processor(TestOnlyOrderRecorder::new("initial"))
            .add_processor(TestOnlyElementMixedAdder::default())
            .add_processor(TestOnlyIdCollector::new(200))
            .add_processor(ElementPrinter::with_prefix("final".to_string()).with_node_ids((1..=200).collect()))
            .add_processor(TestOnlyOrderRecorder::new("final"))
            .add_processor(ElementCounter::new("final"))
            ;

        processor_chain.process(simple_way_element(23, vec![1, 2, 8, 6], vec![("way", "kasper-hotzenplotz")]));
        processor_chain.flush(vec![]);

        let result = processor_chain.collect_result();
        dbg!(&result);
        assert_eq!(&result.counts.get("nodes count final").unwrap().clone(), &4);
        assert_eq!(&result.counts.get("nodes count initial").unwrap().clone(), &0,);
        assert_eq!(&result.counts.get("relations count final").unwrap().clone(), &0,);
        assert_eq!(&result.counts.get("relations count initial").unwrap().clone(), &0,);
        assert_eq!(&result.counts.get("ways count final").unwrap().clone(), &1,);
        assert_eq!(&result.counts.get("ways count initial").unwrap().clone(), &1,);
        assert_eq!(&result.other.get("TestOnlyOrderRecorder initial").unwrap().clone(), "way#23");
        assert_eq!(&result.other.get("TestOnlyOrderRecorder final").unwrap().clone(), "node#1, node#2, node#8, node#6, way#23");
    }
}