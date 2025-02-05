pub(crate) mod collect;
pub(crate) mod filter;
pub(crate) mod predicate;
pub(crate) mod info;
pub(crate) mod modify;
pub mod geotiff;
pub(crate) mod interpolate;
pub(crate) mod skip_ele;

use std::collections::{BTreeMap, HashMap};

use bit_vec::BitVec;
use osm_io::osm::model::element::Element;
use osm_io::osm::model::node::Node;
use osm_io::osm::model::relation::Relation;
use osm_io::osm::model::way::Way;


const HIGHEST_NODE_ID: i64 = 50_000_000_000;

pub fn format_element_id(element: &Element) -> String {
    match &element {
        Element::Node { node } => { format!("node#{}", node.id().to_string()) }
        Element::Way { way } => { format!("way#{}", way.id().to_string()) }
        Element::Relation { relation } => { format!("relation#{}", relation.id().to_string()) }
        Element::Sentinel => {"sentinel#!".to_string()}
    }
}
pub fn into_node_element(node: Node) -> Element { Element::Node {node} }
pub fn into_way_element(way: Way) -> Element { Element::Way { way } }
pub fn into_relation_element(relation: Relation) -> Element { Element::Relation { relation } }
pub fn into_vec_node_element(node: Node) -> Vec<Element> { vec![into_node_element(node)]}
pub fn into_vec_way_element(way: Way) -> Vec<Element> { vec![into_way_element(way)]}
pub fn into_vec_relation_element(relation: Relation) -> Vec<Element> { vec![into_relation_element(relation)]}
pub trait Handler {

    fn name(&self) -> String;
    #[deprecated]
    fn handle_element(&mut self, element: Element) -> Vec<Element> {
        vec![element]
    }

    fn handle_nodes(&mut self, elements: Vec<Node>) -> Vec<Node> {
        elements
    }

    fn handle_ways(&mut self, mut elements: Vec<Way>) -> Vec<Way> {
        elements
    }

    fn handle_relations(&mut self, elements: Vec<Relation>) -> Vec<Relation> {
        elements
    }

    fn handle_and_flush_nodes(&mut self, elements: Vec<Node>) -> Vec<Node> {
        self.handle_nodes(elements)
    }

    fn handle_and_flush_ways(&mut self, elements: Vec<Way>) -> Vec<Way> {
        self.handle_ways(elements)
    }

    fn handle_and_flush_relations(&mut self, elements: Vec<Relation>) -> Vec<Relation> {
        self.handle_relations(elements)
    }

    #[deprecated]
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

pub(crate) struct OsmElementTypeSelection {
    pub node: bool,
    pub way: bool,
    pub relation: bool,
}
impl OsmElementTypeSelection {
    pub(crate) fn all() -> Self { Self { node: true, way: true, relation: true } }
    pub(crate) fn node_only() -> Self { Self { node: true, way: false, relation: false } }
    pub(crate) fn way_only() -> Self { Self { node: false, way: true, relation: false } }
    pub(crate) fn relation_only() -> Self { Self { node: false, way: false, relation: true } }
    pub(crate) fn none() -> Self { Self { node: false, way: false, relation: false } }
}


#[derive(Debug)]
pub struct HandlerResult {
    pub counts: BTreeMap<String, u64>,
    pub other: HashMap<String, String>,
    pub node_ids: BitVec,
    pub skip_ele: BitVec
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
            skip_ele: BitVec::from_elem(nbits, false),
        }
    }
    pub fn to_string(&self) -> String {
        format!("HandlerResult:\n  counts:{:?}\n  other:{:?}", &self.counts, &self.other)
    }
    pub fn to_string_with_node_ids(&self) -> String {
        let node_ids_len = self.node_ids.len();
        let node_ids_true = self.node_ids.iter().filter(|b| b == &true).count();
        let node_ids_false = node_ids_len - node_ids_true;
        format!("HandlerResult:\n  counts: {:?}\n  other: {:?}\n  node_ids: len={} true={} false={}", &self.counts, &self.other, node_ids_len, node_ids_true, node_ids_false)
    }
}


#[derive(Default)]
pub(crate) struct HandlerChain {
    pub processors: Vec<Box<dyn Handler>>,
    flushed_nodes: bool,
    flushed_ways:  bool,
}
impl HandlerChain {
    pub(crate) fn add(mut self, processor: impl Handler + Sized + 'static) -> HandlerChain {
        self.processors.push(Box::new(processor));
        self
    }
    pub(crate) fn process(&mut self, element: Element) {
        log::trace!("######");
        log::trace!("###### Processing {}", format_element_id(&element));
        log::trace!("######");
        match element {
            Element::Node { node } => {
                self.process_nodes(vec![node])
            },
            Element::Way { way } => {
                if !self.flushed_nodes {
                    self.flush_nodes();
                }
                self.process_ways(vec![way])
            },
            Element::Relation { relation } => {
                if !self.flushed_ways {
                    self.flush_ways();
                }
                self.process_relations(vec![relation])
            },
            _ => (),
        }
    }

    fn process_nodes(&mut self, mut elements: Vec<Node>) {
        for processor in &mut self.processors {
            if elements.len() == 0 {
                break
            }
            elements = processor.handle_nodes(elements);
        }
    }

    fn flush_nodes(&mut self) {
        let mut elements = vec![];
        for processor in &mut self.processors {
            log::trace!("######");
            log::trace!("###### Flushing {} with {} nodes flushed by upstream processors", processor.name(), elements.len());
            log::trace!("######");
            elements = processor.handle_and_flush_nodes(elements);
        }
        self.flushed_nodes = true;
    }

    fn process_ways(&mut self, mut elements: Vec<Way>) {
        for processor in &mut self.processors {
            if elements.len() == 0 {
                break
            }
            elements = processor.handle_ways(elements);
        }
    }

    fn flush_ways(&mut self) {
        let mut elements = vec![];
        for processor in &mut self.processors {
            log::trace!("######");
            log::trace!("###### Flushing {} with {} ways flushed by upstream processors", processor.name(), elements.len());
            log::trace!("######");
            elements = processor.handle_and_flush_ways(elements);
        }
        self.flushed_ways = true;
    }

    fn process_relations(&mut self, mut elements: Vec<Relation>) {
        for processor in &mut self.processors {
            if elements.len() == 0 {
                break
            }
            elements = processor.handle_relations(elements);
        }
    }

    fn flush_relations(&mut self) {
        let mut elements = vec![];
        for processor in &mut self.processors {
            log::trace!("######");
            log::trace!("###### Flushing {} with {} relations flushed by upstream processors", processor.name(), elements.len());
            log::trace!("######");
            elements = processor.handle_and_flush_relations(elements);
        }
    }
    pub(crate) fn flush(&mut self) {
        // only relations, as all other elements have been processed and flushed before relations
        self.flush_relations();
    }
    pub(crate) fn collect_result(&mut self) -> HandlerResult {
        let mut result = HandlerResult::default();
        for processor in &mut self.processors {
            result = processor.add_result(result);
        }
        result
    }
}









#[cfg(test)]
pub(crate) mod tests {
    use std::ops::Add;
    use bit_vec::BitVec;
    use osm_io::osm::model::coordinate::Coordinate;
    use osm_io::osm::model::element::Element;
    use osm_io::osm::model::node::Node;
    use osm_io::osm::model::relation::{Member, MemberData, Relation};
    use osm_io::osm::model::tag::Tag;
    use osm_io::osm::model::way::Way;
    use regex::Regex;
    use simple_logger::SimpleLogger;
    use crate::handler::*;
    use crate::handler::filter::*;
    use crate::handler::info::*;

    fn existing_tag() -> String { "EXISTING_TAG".to_string() }
    fn missing_tag() -> String { "MISSING_TAG".to_string() }
    pub enum MemberType { Node, Way, Relation }
    pub fn simple_node_element(id: i64, tags: Vec<(&str, &str)>) -> Element {
        let tags_obj = tags.iter().map(|(k, v)| Tag::new(k.to_string(), v.to_string())).collect();
        node_element(id, 1, Coordinate::new(1.0f64, 1.1f64), 1, 1, 1, "a".to_string(), true, tags_obj)
    }
    pub fn simple_node(id: i64, tags: Vec<(&str, &str)>) -> Node {
        let tags_obj = tags.iter().map(|(k, v)| Tag::new(k.to_string(), v.to_string())).collect();
        Node::new(id, 1, Coordinate::new(1.0f64, 1.1f64), 1, 1, 1, "a".to_string(), true, tags_obj)
    }
    pub fn simple_way_element(id: i64, refs: Vec<i64>, tags: Vec<(&str, &str)>) -> Element {
        let tags_obj = tags.iter().map(|(k, v)| Tag::new(k.to_string(), v.to_string())).collect();
        way_element(id, 1, 1, 1, 1, "a_user".to_string(), true, refs, tags_obj)
    }
    pub fn simple_way(id: i64, refs: Vec<i64>, tags: Vec<(&str, &str)>) -> Way {
        let tags_obj = tags.iter().map(|&(k, v)| Tag::new(String::from(k), String::from(v))).collect();
        Way::new(id, 1, 1, 1, 1, String::from("a_user"), true, refs, tags_obj)
    }
    pub fn simple_relation_element(id: i64, members: Vec<(MemberType, i64, &str)>, tags: Vec<(&str, &str)>) -> Element {
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
    pub fn node_element(id: i64, version: i32, coordinate: Coordinate, timestamp: i64, changeset: i64, uid: i32, user: String, visible: bool, tags: Vec<Tag>) -> Element {
        Element::Node { node: Node::new(id, version, coordinate, timestamp, changeset, uid, user, visible, tags) }
    }
    pub fn way_element(id: i64, version: i32, timestamp: i64, changeset: i64, uid: i32, user: String, visible: bool, refs: Vec<i64>, tags: Vec<Tag>) -> Element {
        Element::Way { way: Way::new(id, version, timestamp, changeset, uid, user, visible, refs, tags) }
    }
    pub fn relation_element(id: i64, version: i32, timestamp: i64, changeset: i64, uid: i32, user: String, visible: bool, members: Vec<Member>, tags: Vec<Tag>) -> Element {
        Element::Relation { relation: Relation::new(id, version, timestamp, changeset, uid, user, visible, members, tags) }
    }
    pub fn copy_node_with_new_id(node: &Node, new_id: i64) -> Node {
        Node::new(new_id, node.version(), node.coordinate().clone(), node.timestamp(), node.changeset(), node.uid(), node.user().clone(), node.visible(), node.tags().clone())
    }

    ///Modify element and return same instance.
    #[derive(Debug, Default)]
    pub(crate) struct TestOnlyElementModifier;
    impl TestOnlyElementModifier {
        fn handle_node(&mut self, mut node: &mut Node)  {
            let id = node.id();
            let tags = node.tags_mut();
            if id % 2 == 0 {
                tags.push(Tag::new("added".to_string(), "yes".to_string()));
            }
        }
    }
    impl Handler for TestOnlyElementModifier {
        fn name(&self) -> String { "TestOnlyElementModifier".to_string() }

        fn handle_nodes(&mut self, mut elements: Vec<Node>) -> Vec<Node> {
            elements.iter_mut().for_each(|node| self.handle_node(node));
            elements
        }
    }

    ///Return a copy of the element, e.g. a different instance.
    #[derive(Debug, Default)]
    pub(crate) struct TestOnlyElementReplacer;
    impl Handler for TestOnlyElementReplacer {
        fn name(&self) -> String { "TestOnlyElementReplacer".to_string() }

        fn handle_nodes(&mut self, mut elements: Vec<Node>) -> Vec<Node> {
            elements.iter().map(|node| if node.id() == 6 {simple_node(66, vec![("who", "dimpfelmoser")])} else {node.clone()}).collect()
        }
    }

    ///Remove an element / return empty vec.
    #[derive(Debug, Default)]
    pub(crate) struct TestOnlyElementFilter;
    impl Handler for TestOnlyElementFilter {
        fn name(&self) -> String { "TestOnlyElementFilter".to_string() }

        fn handle_nodes(&mut self, mut elements: Vec<Node>) -> Vec<Node> {
            elements.retain(|node| node.id() % 2 != 0 );
            elements
        }
    }

    ///Receive one element, return two of the same type.
    #[derive(Debug, Default)]
    pub(crate) struct TestOnlyElementAdder;
    impl TestOnlyElementAdder {
        fn handle_node(&self, node: Node) -> Vec<Node> {
            let node_clone = copy_node_with_new_id(&node, node.id().add(100));
            vec![node_clone, node]
        }
    }
    impl Handler for TestOnlyElementAdder {
        fn name(&self) -> String { "TestOnlyElementAdder".to_string() }

        fn handle_nodes(&mut self, elements: Vec<Node>) -> Vec<Node> {
            let mut result = Vec::new();
            elements.iter().for_each(|node| result.extend(self.handle_node(node.clone())));
            result
        }
    }

    ///Receive one way, return a way and a new node for each ref of the way.
    #[derive(Debug, Default)]
    pub(crate) struct TestOnlyElementMixedAdder;
    impl Handler for TestOnlyElementMixedAdder {
        fn name(&self) -> String { "TestOnlyElementMixedAdder".to_string() }
        fn handle_element(&mut self, element: Element) -> Vec<Element> {
            match element {
                Element::Node { .. } => { vec![element] }
                Element::Way { way } => {
                    let mut elements: Vec<Element> = way.refs().iter().map(|id| simple_node_element(id.clone(), vec![("added", "by handler")])).collect();
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
        fn handle_node(&self, node: Node) -> Vec<Node> {
            let mut node_clone = copy_node_with_new_id(&node, node.id().add(100));
            node_clone.tags_mut().push(Tag::new("elevation".to_string(), "default-elevation".to_string()));
            vec![node, node_clone]
        }
    }
    impl Handler for TestOnlyElementBufferingDuplicatingEditingProcessor {
        // fn struct_name() -> &'static str { "TestOnlyElementBuffer" }
        fn name(&self) -> String { "TestOnlyElementBuffer".to_string() }
        fn handle_nodes(&mut self, mut elements: Vec<Node>) -> Vec<Node> {
            self.nodes.append(&mut elements);
            if self.nodes.len() >= 3 {
                return self.handle_and_flush_nodes(elements);
            }
            elements
        }

        fn handle_ways(&mut self, mut elements: Vec<Way>) -> Vec<Way> {
            self.ways.append(&mut elements);
            if self.ways.len() >= 3 {
                return self.handle_and_flush_ways(Vec::new());
            }
            elements
        }

        fn handle_relations(&mut self, mut elements: Vec<Relation>) -> Vec<Relation> {
            self.relations.append(&mut elements);
            if self.relations.len() >= 3 {
                return self.handle_and_flush_relations(Vec::new());
            }
            elements
        }

        fn handle_and_flush_nodes(&mut self, mut elements: Vec<Node> ) -> Vec<Node> {
            let mut result = Vec::new();
            self.nodes.append(&mut elements);
            self.nodes.iter().for_each(|node| result.extend(self.handle_node(node.clone())));
            self.nodes.clear();
            result
        }
        fn handle_and_flush_ways(&mut self, mut elements: Vec<Way>) -> Vec<Way> {
            // modifying the elements is tested in handle_and_flush_nodes -> should be the same here
            let mut result = Vec::new();
            result.append(&mut self.ways);
            result.append(&mut elements);
            result
        }
        fn handle_and_flush_relations(&mut self, mut elements: Vec<Relation>) -> Vec<Relation> {
            // modifying the elements is tested in handle_and_flush_nodes -> should be the same here
            let mut result = Vec::new();
            result.append(&mut self.relations);
            result.append(&mut elements);
            result
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
    impl Handler for TestOnlyIdCollector {
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
        fn handle_nodes(&mut self, elements: Vec<Node>) -> Vec<Node> {
            elements.iter().for_each(|element| self.node_ids.set(element.id() as usize, true));
            elements
        }

        fn handle_ways(&mut self, elements: Vec<Way>) -> Vec<Way> {
            elements.iter().for_each(|element| self.way_ids.set(element.id() as usize, true));
            elements
        }

        fn handle_relations(&mut self, elements: Vec<Relation>) -> Vec<Relation> {
            elements.iter().for_each(|element| self.relation_ids.set(element.id() as usize, true));
            elements
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

        fn handle_element(&mut self, element: Element) {
            self.received_ids.push(format_element_id(&element));
        }
    }
    impl Handler for TestOnlyOrderRecorder {
        fn name(&self) -> String { format!("TestOnlyOrderRecorder {}", self.result_key) }

        fn handle_nodes(&mut self, mut elements: Vec<Node>) -> Vec<Node> {
            elements.iter().for_each(|element| self.handle_element(into_node_element(element.clone())));
            elements
        }

        fn handle_ways(&mut self, mut elements: Vec<Way>) -> Vec<Way> {
            elements.iter().for_each(|element| self.handle_element(into_way_element(element.clone())));
            elements
        }

        fn handle_relations(&mut self, mut elements: Vec<Relation>) -> Vec<Relation> {
            elements.iter().for_each(|element| self.handle_element(into_relation_element(element.clone())));
            elements
        }

        fn add_result(&mut self, mut result: HandlerResult) -> HandlerResult {
            result.other.insert(format!("{}", self.name()), self.received_ids.join(", "));
            result
        }
    }


    #[test]
    /// Assert that it is possible to run a chain of processors.
    fn test_chain() {
        let mut processor_chain = HandlerChain::default()
            .add(ElementCounter::new("initial"))
            .add(TestOnlyOrderRecorder::new("initial"))

            .add(TestOnlyIdCollector::new(10))

            .add(ElementPrinter::with_prefix("final: ".to_string()).with_node_ids(hashset! {8}))
            .add(TestOnlyOrderRecorder::new("final"))
            .add(ElementCounter::new("final"))
            ;
        processor_chain.process(simple_node_element(1, vec![("who", "kasper")]));
        processor_chain.process(simple_node_element(2, vec![("who", "seppl")]));
        processor_chain.process(simple_node_element(6, vec![("who", "hotzenplotz")]));
        processor_chain.process(simple_node_element(8, vec![("who", "großmutter")]));
        processor_chain.flush();
        let result = processor_chain.collect_result();

        assert_eq!(&result.counts.get("nodes count initial").unwrap().clone(), &4,);
        assert_eq!(&result.counts.get("nodes count final").unwrap().clone(), &4);
        assert_eq!(&result.counts.get("relations count initial").unwrap().clone(), &0,);
        assert_eq!(&result.counts.get("relations count final").unwrap().clone(), &0,);
        assert_eq!(&result.counts.get("ways count initial").unwrap().clone(), &0,);
        assert_eq!(&result.counts.get("ways count final").unwrap().clone(), &0,);
        assert_eq!(&result.other.get("TestOnlyOrderRecorder initial").unwrap().clone(), "node#1, node#2, node#6, node#8");
        assert_eq!(&result.other.get("TestOnlyOrderRecorder final").unwrap().clone(), "node#1, node#2, node#6, node#8");
        assert!(&result.node_ids.get(1).unwrap());
        assert!(&result.node_ids.get(2).unwrap());
        assert!(&result.node_ids.get(6).unwrap());
        assert!(&result.node_ids.get(8).unwrap());
        assert!( ! &result.node_ids.get(3).unwrap());
    }

    #[test]
    /// Assert that it is possible to run the chain and let processors buffer elements:
    /// E.g. keeping received elements and postpone their handling until a specifig number of
    /// elements are collected. Then process the buffered elements in a batch and pass them
    /// to downstream processors.
    /// Assert that after the last element was pusehd into the pipeline, the elements that are
    /// still buffered will be flushed: handled and passed to downstream processors.
    /// The test uses TestOnlyElementBufferingDuplicatingEditingProcessor for this.
    fn test_chain_with_buffer() {
        let _ = SimpleLogger::new().init();
        let mut processor_chain = HandlerChain::default()
            .add(ElementCounter::new("initial"))
            .add(TestOnlyOrderRecorder::new("initial"))

            .add(TestOnlyElementBufferingDuplicatingEditingProcessor::default())

            .add(ElementPrinter::with_prefix("final".to_string()).with_node_ids((1..=200).collect()))
            .add(TestOnlyOrderRecorder::new("final"))
            .add(ElementCounter::new("final"))
            ;

        processor_chain.process(simple_node_element(1, vec![("who", "kasper")]));
        processor_chain.process(simple_node_element(2, vec![("who", "seppl")]));
        processor_chain.process(simple_node_element(6, vec![("who", "hotzenplotz")]));
        processor_chain.process(simple_node_element(8, vec![("who", "großmutter")]));
        processor_chain.process(simple_way_element(23, vec![1, 2, 8, 6], vec![("who", "kasper")]));
        processor_chain.process(simple_relation_element(66, vec![(MemberType::Way, 23, "kasper&seppl brign großmutter to hotzenplotz")], vec![("who", "großmutter")]));
        processor_chain.flush();
        let result = processor_chain.collect_result();

        assert_eq!(&result.counts.get("nodes count initial").unwrap().clone(), &4,);
        assert_eq!(&result.counts.get("nodes count final").unwrap().clone(), &8);
        assert_eq!(&result.counts.get("relations count initial").unwrap().clone(), &1,);
        assert_eq!(&result.counts.get("relations count final").unwrap().clone(), &1,);
        assert_eq!(&result.counts.get("ways count initial").unwrap().clone(), &1,);
        assert_eq!(&result.counts.get("ways count final").unwrap().clone(), &1,);
        assert_eq!(&result.other.get("TestOnlyOrderRecorder initial").unwrap().clone(), "node#1, node#2, node#6, node#8, way#23, relation#66");
        assert_eq!(&result.other.get("TestOnlyOrderRecorder final").unwrap().clone(), "node#1, node#101, node#2, node#102, node#6, node#106, node#8, node#108, way#23, relation#66");
    }

    #[test]
    #[ignore]//this functionality is unsupported in current handler implementation
    /// Assert that it is possible to run the chain and let processors receive one element
    /// and add additional elements of a different type to the processing chain
    /// that are processed by downstream processors.
    /// The test uses TestOnlyElementMixedAdder for this.
    fn test_chain_with_mixed_element_adder() {
        let _ = SimpleLogger::new().init();
        let mut processor_chain = HandlerChain::default()
            .add(ElementCounter::new("initial"))
            .add(TestOnlyOrderRecorder::new("initial"))

            .add(TestOnlyElementMixedAdder::default())

            .add(ElementPrinter::with_prefix("final".to_string()).with_node_ids((1..=200).collect()))
            .add(TestOnlyOrderRecorder::new("final"))
            .add(ElementCounter::new("final"))
            ;

        processor_chain.process(simple_way_element(22, vec![], vec![]));
        processor_chain.process(simple_way_element(23, vec![1, 2, 8, 6], vec![("way", "kasper-hotzenplotz")]));
        processor_chain.flush();
        let result = processor_chain.collect_result();

        assert_eq!(&result.counts.get("nodes count initial").unwrap().clone(), &0,);
        assert_eq!(&result.counts.get("nodes count final").unwrap().clone(), &4);
        assert_eq!(&result.counts.get("relations count initial").unwrap().clone(), &0,);
        assert_eq!(&result.counts.get("relations count final").unwrap().clone(), &0,);
        assert_eq!(&result.counts.get("ways count initial").unwrap().clone(), &2,);
        assert_eq!(&result.counts.get("ways count final").unwrap().clone(), &2,);
        assert_eq!(&result.other.get("TestOnlyOrderRecorder initial").unwrap().clone(), "way#22, way#23");
        assert_eq!(&result.other.get("TestOnlyOrderRecorder final").unwrap().clone(), "way#22, node#1, node#2, node#8, node#6, way#23");
    }

    #[test]
    /// Assert that it is possible to run the chain and let processors receive one element
    /// and add additional elements of the same type to the processing chain
    /// that are processed by downstream processors.
    /// The test uses TestOnlyElementAdder for this.
    fn test_chain_with_element_adder() {
        let _ = SimpleLogger::new().init();
        let mut processor_chain = HandlerChain::default()
            .add(ElementCounter::new("initial"))
            .add(TestOnlyOrderRecorder::new("initial"))

            .add(TestOnlyElementAdder::default())

            .add(ElementPrinter::with_prefix("final".to_string()).with_node_ids((1..=200).collect()))
            .add(TestOnlyOrderRecorder::new("final"))
            .add(ElementCounter::new("final"))
            ;

        processor_chain.process(simple_way_element(23, vec![1, 2, 8, 6], vec![("who", "kasper")]));
        processor_chain.process(simple_node_element(1, vec![("who", "kasper")]));
        processor_chain.process(simple_node_element(2, vec![("who", "seppl")]));
        processor_chain.process(simple_node_element(6, vec![("who", "hotzenplotz")]));
        processor_chain.process(simple_node_element(8, vec![("who", "großmutter")]));
        processor_chain.flush();
        let result = processor_chain.collect_result();

        assert_eq!(&result.counts.get("nodes count initial").unwrap().clone(), &4,);
        assert_eq!(&result.counts.get("nodes count final").unwrap().clone(), &8);
        assert_eq!(&result.counts.get("relations count initial").unwrap().clone(), &0,);
        assert_eq!(&result.counts.get("relations count final").unwrap().clone(), &0,);
        assert_eq!(&result.counts.get("ways count initial").unwrap().clone(), &1,);
        assert_eq!(&result.counts.get("ways count final").unwrap().clone(), &1,);
        assert_eq!(&result.other.get("TestOnlyOrderRecorder initial").unwrap().clone(), "way#23, node#1, node#2, node#6, node#8");
        assert_eq!(&result.other.get("TestOnlyOrderRecorder final").unwrap().clone(), "way#23, node#101, node#1, node#102, node#2, node#106, node#6, node#108, node#8");
    }

    #[test]
    /// Assert that it is possible to run the chain and let processors permanently filter (remove) elements.
    /// The test uses TestOnlyElementFilter for this, which filters nodes with an even id.
    fn test_chain_with_element_filter() {
        let _ = SimpleLogger::new().init();
        let mut processor_chain = HandlerChain::default()
            .add(ElementCounter::new("initial"))
            .add(TestOnlyOrderRecorder::new("initial"))

            .add(TestOnlyElementFilter::default())

            .add(ElementPrinter::with_prefix("final".to_string()).with_node_ids((1..=200).collect()))
            .add(TestOnlyOrderRecorder::new("final"))
            .add(ElementCounter::new("final"))
            ;

        processor_chain.process(simple_node_element(1, vec![("who", "kasper")]));
        processor_chain.process(simple_node_element(2, vec![("who", "seppl")]));
        processor_chain.process(simple_node_element(6, vec![("who", "hotzenplotz")]));
        processor_chain.process(simple_node_element(8, vec![("who", "großmutter")]));
        processor_chain.flush();
        let result = processor_chain.collect_result();

        assert_eq!(&result.counts.get("nodes count initial").unwrap().clone(), &4,);
        assert_eq!(&result.counts.get("nodes count final").unwrap().clone(), &1);
        assert_eq!(&result.counts.get("relations count initial").unwrap().clone(), &0,);
        assert_eq!(&result.counts.get("relations count final").unwrap().clone(), &0,);
        assert_eq!(&result.counts.get("ways count initial").unwrap().clone(), &0,);
        assert_eq!(&result.counts.get("ways count final").unwrap().clone(), &0,);
        assert_eq!(&result.other.get("TestOnlyOrderRecorder initial").unwrap().clone(), "node#1, node#2, node#6, node#8");
        assert_eq!(&result.other.get("TestOnlyOrderRecorder final").unwrap().clone(), "node#1");
    }

    #[test]
    /// Assert that it is possible to run the chain and let processors return new instances,
    /// e.g. copies of received elements.
    /// The test uses TestOnlyElementReplacer for this, which replaces node#6 with a new instance.
    fn test_chain_with_element_replacer() {
        let _ = SimpleLogger::new().init();
        let mut processor_chain = HandlerChain::default()
            .add(ElementCounter::new("initial"))
            .add(TestOnlyOrderRecorder::new("initial"))

            .add(TestOnlyElementReplacer::default())

            .add(ElementPrinter::with_prefix("final".to_string()).with_node_ids((1..=200).collect()))
            .add(TestOnlyOrderRecorder::new("final"))
            .add(ElementCounter::new("final"))
            ;

        processor_chain.process(simple_node_element(1, vec![("who", "kasper")]));
        processor_chain.process(simple_node_element(2, vec![("who", "seppl")]));
        processor_chain.process(simple_node_element(6, vec![("who", "hotzenplotz")]));
        processor_chain.process(simple_node_element(8, vec![("who", "großmutter")]));
        processor_chain.flush();
        let result = processor_chain.collect_result();

        assert_eq!(&result.counts.get("nodes count initial").unwrap().clone(), &4,);
        assert_eq!(&result.counts.get("nodes count final").unwrap().clone(), &4);
        assert_eq!(&result.counts.get("relations count initial").unwrap().clone(), &0,);
        assert_eq!(&result.counts.get("relations count final").unwrap().clone(), &0,);
        assert_eq!(&result.counts.get("ways count initial").unwrap().clone(), &0,);
        assert_eq!(&result.counts.get("ways count final").unwrap().clone(), &0,);
        assert_eq!(&result.other.get("TestOnlyOrderRecorder initial").unwrap().clone(), "node#1, node#2, node#6, node#8");
        assert_eq!(&result.other.get("TestOnlyOrderRecorder final").unwrap().clone(), "node#1, node#2, node#66, node#8");
    }

    #[test]
    /// Assert that it is possible to run the chain and let processors modify received instances,
    /// e.g. without cloning.
    /// The test uses
    /// - TestOnlyElementModifier, which adds a tag "added"="yes" to nodes with even id and
    /// - TagKeyBasedOsmElementsFilter, which only accepts elements with this tag.
    /// The TestOnlyElementModifier also changes values of tags "who" to upper case,
    /// which is not explicitly asserted.
    fn test_chain_with_element_modifier() {
        let _ = SimpleLogger::new().init();
        let mut processor_chain = HandlerChain::default()
            .add(ElementCounter::new("initial"))
            .add(TestOnlyOrderRecorder::new("initial"))

            .add(TestOnlyElementModifier::default())
            .add(TagKeyBasedOsmElementsFilter::new(OsmElementTypeSelection::node_only(), vec!["added".to_string()], FilterType::AcceptMatching))

            .add(ElementPrinter::with_prefix("final".to_string()).with_node_ids((1..=200).collect()))
            .add(TestOnlyOrderRecorder::new("final"))
            .add(ElementCounter::new("final"))
            ;

        processor_chain.process(simple_node_element(1, vec![("who", "kasper")]));
        processor_chain.process(simple_node_element(2, vec![("who", "seppl")]));
        processor_chain.process(simple_node_element(6, vec![("who", "hotzenplotz")]));
        processor_chain.process(simple_node_element(8, vec![("who", "großmutter")]));
        processor_chain.flush();
        let result = processor_chain.collect_result();

        assert_eq!(&result.counts.get("nodes count initial").unwrap().clone(), &4,);
        //kasper with odd id was not modified and later filtered:
        assert_eq!(&result.counts.get("nodes count final").unwrap().clone(), &3);
        assert_eq!(&result.counts.get("relations count initial").unwrap().clone(), &0,);
        assert_eq!(&result.counts.get("relations count final").unwrap().clone(), &0,);
        assert_eq!(&result.counts.get("ways count initial").unwrap().clone(), &0,);
        assert_eq!(&result.counts.get("ways count final").unwrap().clone(), &0,);
        assert_eq!(&result.other.get("TestOnlyOrderRecorder initial").unwrap().clone(), "node#1, node#2, node#6, node#8");
        assert_eq!(&result.other.get("TestOnlyOrderRecorder final").unwrap().clone(), "node#2, node#6, node#8");
    }







    #[test]
    fn handler_chain() {
        let _ = SimpleLogger::new().init();
        let chain = HandlerChain::default()
            .add(ElementCounter::new("initial"))
            .add(TagValueBasedOsmElementsFilter::new(
                OsmElementTypeSelection::node_only(),
                existing_tag(),
                Regex::new(".*p.*").unwrap(),
                FilterType::AcceptMatching))
            .add(TagValueBasedOsmElementsFilter::new(
                OsmElementTypeSelection::node_only(),
                existing_tag(),
                Regex::new(".*z.*").unwrap(),
                FilterType::RemoveMatching))
            .add(ElementCounter::new("final"))
            .add(TestOnlyIdCollector::new(100));

        handle_test_nodes_and_verify_result(chain);
    }

    #[test]
    fn handler_chain_with_node_id_filter() {
        let _ = SimpleLogger::new().init();
        let mut node_ids = BitVec::from_elem(10usize, false);
        node_ids.set(1usize, true);
        node_ids.set(2usize, true);
        let chain = HandlerChain::default()
            .add(ElementCounter::new("initial"))
            .add(NodeIdFilter { node_ids: node_ids.clone() })
            .add(ElementCounter::new("final"))
            .add(TestOnlyIdCollector::new(100));

        handle_test_nodes_and_verify_result(chain);
    }

    fn handle_test_nodes_and_verify_result(mut handler_chain: HandlerChain) {
        handler_chain.process(simple_node_element(1, vec![(existing_tag().as_str(), "kasper")]));
        handler_chain.process(simple_node_element(2, vec![(existing_tag().as_str(), "seppl")]));
        handler_chain.process(simple_node_element(3, vec![(existing_tag().as_str(), "hotzenplotz")]));
        handler_chain.process(simple_node_element(4, vec![(existing_tag().as_str(), "großmutter")]));

        handler_chain.flush();
        let result = handler_chain.collect_result();

        assert_eq!(&result.counts.get("nodes count initial").unwrap().clone(), &4,);
        assert_eq!(&result.counts.get("nodes count final").unwrap().clone(), &2);
        assert_eq!(result.node_ids[0], false);
        assert_eq!(result.node_ids[1], true);
        assert_eq!(result.node_ids[2], true);
        assert_eq!(result.node_ids[3], false);
        assert_eq!(result.node_ids[4], false);
    }
}
