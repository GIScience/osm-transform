mod collect;
mod filter;
mod predicate;
mod info;
mod modify;
pub mod geotiff;
mod interpolate;

use std::collections::{BTreeMap, HashMap, HashSet};

use bit_vec::BitVec;
use osm_io::osm::model::element::Element;
use osm_io::osm::model::node::Node;
use osm_io::osm::model::relation::Relation;
use osm_io::osm::model::tag::Tag;
use osm_io::osm::model::way::Way;


const HIGHEST_NODE_ID: i64 = 50_000_000_000; //todo make configurable

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
    pub fn to_string(&self) -> String {
        format!("HandlerResult:\n  {:?}\n  {:?}", &self.counts, &self.other)
    }
}


#[derive(Default)]
pub(crate) struct ProcessorChain {
    pub processors: Vec<Box<dyn Processor>>,
}
impl ProcessorChain {
    pub(crate) fn add(mut self, processor: impl Processor + Sized + 'static) -> ProcessorChain {
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









#[cfg(test)]
pub(crate) mod tests {
    use std::ops::Add;
    use bit_vec::BitVec;
    use log4rs::append::Append;
    use osm_io::osm::model::coordinate::Coordinate;
    use osm_io::osm::model::element::Element;
    use osm_io::osm::model::node::Node;
    use osm_io::osm::model::relation::{Member, MemberData, Relation};
    use osm_io::osm::model::tag::Tag;
    use osm_io::osm::model::way::Way;
    use regex::Regex;
    use simple_logger::SimpleLogger;
    use crate::processor::*;
    use crate::processor::filter::*;
    use crate::processor::info::*;

    fn existing_tag() -> String { "EXISTING_TAG".to_string() }
    fn missing_tag() -> String { "MISSING_TAG".to_string() }
    pub enum MemberType { Node, Way, Relation }
    pub fn simple_node_element(id: i64, tags: Vec<(&str, &str)>) -> Element {
        let tags_obj = tags.iter().map(|(k, v)| Tag::new(k.to_string(), v.to_string())).collect();
        node_element(id, 1, Coordinate::new(1.0f64, 1.1f64), 1, 1, 1, "a".to_string(), true, tags_obj)
    }
    pub fn simple_way_element(id: i64, refs: Vec<i64>, tags: Vec<(&str, &str)>) -> Element {
        let tags_obj = tags.iter().map(|(k, v)| Tag::new(k.to_string(), v.to_string())).collect();
        way_element(id, 1, 1, 1, 1, "a_user".to_string(), true, refs, tags_obj)
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
    impl Processor for TestOnlyElementModifier {
        fn name(&self) -> String { "TestOnlyElementModifier".to_string() }
        fn handle_element(&mut self, element: Element) -> Vec<Element> {
            match element {
                Element::Node { mut node} => {
                    let id = node.id().clone();
                    let tags = node.tags_mut();
                    if id % 2 == 0 {
                        tags.push(Tag::new("added".to_string(), "yes".to_string()));
                    }
                    if let Some(tag_to_modify_pos) = tags.iter().position(|tag| tag.k() == "who"){
                        let tag_to_modify = tags.remove(tag_to_modify_pos);
                        tags.push(Tag::new(tag_to_modify.k().clone(), tag_to_modify.v().to_string().to_uppercase()));
                    }
                    vec![into_node_element(node)]
                }
                _ => vec![element]
            }
        }
    }

    ///Return a copy of the element, e.g. a different instance.
    #[derive(Debug, Default)]
    pub(crate) struct TestOnlyElementReplacer;
    impl Processor for TestOnlyElementReplacer {
        fn name(&self) -> String { "TestOnlyElementReplacer".to_string() }
        fn handle_element(&mut self, element: Element) -> Vec<Element> {
            match element {
                Element::Node { node} => {
                    if node.id() == 6 {
                        vec![
                            simple_node_element(66, vec![("who", "dimpfelmoser")])
                        ]
                    } else {
                        vec![into_node_element(node)]
                    }
                }
                _ => vec![element]
            }
        }
    }

    ///Remove an element / return empty vec.
    #[derive(Debug, Default)]
    pub(crate) struct TestOnlyElementFilter;
    impl Processor for TestOnlyElementFilter {
        fn name(&self) -> String { "TestOnlyElementFilter".to_string() }
        fn handle_element(&mut self, element: Element) -> Vec<Element> {
            match element {
                Element::Node { node } => {
                    if node.id() % 2 == 0 {
                        vec![]
                    } else {
                        vec![Element::Node {node}]
                    }
                }
                _ => vec![element]
            }
        }
    }

    ///Receive one element, return two of the same type.
    #[derive(Debug, Default)]
    pub(crate) struct TestOnlyElementAdder;
    impl Processor for TestOnlyElementAdder {
        fn name(&self) -> String { "TestOnlyElementAdder".to_string() }
        fn handle_element(&mut self, element: Element) -> Vec<Element> {
            match element {
                Element::Node { node } => {
                    let mut elements = vec![
                        Element::Node { node: copy_node_with_new_id(&node, node.id().clone().add(100))}
                    ];
                    elements.push(Element::Node{ node});
                    elements
                }
                _ => vec![element]
            }
        }
    }

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
    /// Assert that it is possible to run a chain of processors.
    fn test_chain() {
        let mut processor_chain = ProcessorChain::default()
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
        processor_chain.flush(vec![]);
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
        SimpleLogger::new().init();
        let mut processor_chain = ProcessorChain::default()
            .add(ElementCounter::new("initial"))
            .add(TestOnlyOrderRecorder::new("initial"))

            .add(TestOnlyElementBufferingDuplicatingEditingProcessor::default())

            .add(ElementPrinter::with_prefix("final".to_string()).with_node_ids((1..=200).collect()))
            .add(TestOnlyOrderRecorder::new("final"))
            .add(ElementCounter::new("final"))
            ;

        processor_chain.process(simple_way_element(23, vec![1, 2, 8, 6], vec![("who", "kasper")]));
        processor_chain.process(simple_node_element(1, vec![("who", "kasper")]));
        processor_chain.process(simple_node_element(2, vec![("who", "seppl")]));
        processor_chain.process(simple_node_element(6, vec![("who", "hotzenplotz")]));
        processor_chain.process(simple_node_element(8, vec![("who", "großmutter")]));
        processor_chain.process(simple_relation_element(66, vec![(MemberType::Way, 23, "kasper&seppl brign großmutter to hotzenplotz")], vec![("who", "großmutter")]));
        processor_chain.flush(vec![]);
        let result = processor_chain.collect_result();

        assert_eq!(&result.counts.get("nodes count initial").unwrap().clone(), &4,);
        assert_eq!(&result.counts.get("nodes count final").unwrap().clone(), &8);
        assert_eq!(&result.counts.get("relations count initial").unwrap().clone(), &1,);
        assert_eq!(&result.counts.get("relations count final").unwrap().clone(), &1,);
        assert_eq!(&result.counts.get("ways count initial").unwrap().clone(), &1,);
        assert_eq!(&result.counts.get("ways count final").unwrap().clone(), &1,);
        assert_eq!(&result.other.get("TestOnlyOrderRecorder initial").unwrap().clone(), "way#23, node#1, node#2, node#6, node#8, relation#66");
        assert_eq!(&result.other.get("TestOnlyOrderRecorder final").unwrap().clone(), "node#1, node#101, node#2, node#102, node#6, node#106, node#8, node#108, way#23, relation#66");
    }

    #[test]
    /// Assert that it is possible to run the chain and let processors receive one element
    /// and add additional elements of a different type to the processing chain
    /// that are processed by downstream processors.
    /// The test uses TestOnlyElementMixedAdder for this.
    fn test_chain_with_mixed_element_adder() {
        SimpleLogger::new().init();
        let mut processor_chain = ProcessorChain::default()
            .add(ElementCounter::new("initial"))
            .add(TestOnlyOrderRecorder::new("initial"))

            .add(TestOnlyElementMixedAdder::default())

            .add(ElementPrinter::with_prefix("final".to_string()).with_node_ids((1..=200).collect()))
            .add(TestOnlyOrderRecorder::new("final"))
            .add(ElementCounter::new("final"))
            ;

        processor_chain.process(simple_way_element(22, vec![], vec![]));
        processor_chain.process(simple_way_element(23, vec![1, 2, 8, 6], vec![("way", "kasper-hotzenplotz")]));
        processor_chain.flush(vec![]);
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
        SimpleLogger::new().init();
        let mut processor_chain = ProcessorChain::default()
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
        processor_chain.flush(vec![]);
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
        SimpleLogger::new().init();
        let mut processor_chain = ProcessorChain::default()
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
        processor_chain.flush(vec![]);
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
        SimpleLogger::new().init();
        let mut processor_chain = ProcessorChain::default()
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
        processor_chain.flush(vec![]);
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
        SimpleLogger::new().init();
        let mut processor_chain = ProcessorChain::default()
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
        processor_chain.flush(vec![]);
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
        SimpleLogger::new().init();
        let chain = ProcessorChain::default()
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
        SimpleLogger::new().init();
        let mut node_ids = BitVec::from_elem(10usize, false);
        node_ids.set(1usize, true);
        node_ids.set(2usize, true);
        let chain = ProcessorChain::default()
            .add(ElementCounter::new("initial"))
            .add(NodeIdFilter { node_ids: node_ids.clone() })
            .add(ElementCounter::new("final"))
            .add(TestOnlyIdCollector::new(100));

        handle_test_nodes_and_verify_result(chain);
    }

    fn handle_test_nodes_and_verify_result(mut handler_chain: ProcessorChain) {
        handler_chain.process(simple_node_element(1, vec![(existing_tag().as_str(), "kasper")]));
        handler_chain.process(simple_node_element(2, vec![(existing_tag().as_str(), "seppl")]));
        handler_chain.process(simple_node_element(3, vec![(existing_tag().as_str(), "hotzenplotz")]));
        handler_chain.process(simple_node_element(4, vec![(existing_tag().as_str(), "großmutter")]));

        handler_chain.flush(vec![]);
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