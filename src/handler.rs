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
use crate::Config;

pub(crate) const HIGHEST_NODE_ID: i64 = 50_000_000_000;

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

    fn handle_elements(&mut self, nodes: Vec<Node>, ways: Vec<Way>, relations: Vec<Relation>) -> (Vec<Node>, Vec<Way>, Vec<Relation>){
        log::trace!("Handler.handle_elements() called with {} nodes, {} ways, {} relations", nodes.len(), ways.len(), relations.len());
        (self.handle_nodes(nodes), self.handle_ways(ways), self.handle_relations(relations))
    }

    fn handle_and_flush_elements(&mut self, nodes: Vec<Node>, ways: Vec<Way>, relations: Vec<Relation>) -> (Vec<Node>, Vec<Way>, Vec<Relation>) {
        log::trace!("Handler.handle_and_flush_elements() called with {} nodes, {} ways, {} relations", nodes.len(), ways.len(), relations.len());
        (self.handle_and_flush_nodes(nodes), self.handle_and_flush_ways(ways), self.handle_and_flush_relations(relations))
    }

    fn handle_nodes(&mut self, elements: Vec<Node>) -> Vec<Node> {
        elements
    }

    fn handle_ways(&mut self, elements: Vec<Way>) -> Vec<Way> {
        elements
    }

    fn handle_relations(&mut self, elements: Vec<Relation>) -> Vec<Relation> {
        elements
    }

    fn handle_and_flush_nodes(&mut self, elements: Vec<Node>) -> Vec<Node> {
        log::trace!("Handler.handle_and_flush_nodes() called with {} nodes", elements.len());
        self.handle_nodes(elements)
    }

    fn handle_and_flush_ways(&mut self, elements: Vec<Way>) -> Vec<Way> {
        log::trace!("Handler.handle_and_flush_ways() called with {} ways", elements.len());
        self.handle_ways(elements)
    }

    fn handle_and_flush_relations(&mut self, elements: Vec<Relation>) -> Vec<Relation> {
        log::trace!("Handler.handle_and_flush_relations() called with {} relations", elements.len());
        self.handle_relations(elements)
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
#[allow(dead_code)]
impl OsmElementTypeSelection {
    pub(crate) fn all() -> Self { Self { node: true, way: true, relation: true } }
    pub(crate) fn node_only() -> Self { Self { node: true, way: false, relation: false } }
    pub(crate) fn way_only() -> Self { Self { node: false, way: true, relation: false } }
    pub(crate) fn relation_only() -> Self { Self { node: false, way: false, relation: true } }
    pub(crate) fn none() -> Self { Self { node: false, way: false, relation: false } }
}


pub struct HandlerResult {
    pub other: HashMap<String, String>,
    pub node_ids: BitVec,
    pub skip_ele: BitVec,
    
    // InputCount,
    pub input_node_count: u64,
    pub input_way_count: u64,
    pub input_relation_count: u64,

    // AcceptedCount,
    pub accepted_node_count: u64,      //Referenced by accepted ways or relations
    pub accepted_way_count: u64,       //Not filtered by tags
    pub accepted_relation_count: u64,  //Not filtered by tags

    pub country_not_found_node_count: u64,
    pub elevation_not_found_node_count: u64,
    pub elevation_not_relevant_node_count: u64,

    pub splitted_way_count: u64,
    pub added_node_count: u64,

    // OutputCount,
    pub output_node_count: u64,
    pub output_way_count: u64,
    pub output_relation_count: u64,

    pub available_elevation_tiff_count: u64,
    pub used_elevation_tiff_count: u64,
    pub elevation_buffer_flush_count_buffer_max_reached: u64,
    pub elevation_buffer_flush_count_total_max_reached: u64,
}
impl HandlerResult {
    pub(crate) fn default() -> Self {
        Self::with_capacity(HIGHEST_NODE_ID as usize)
    }
    fn with_capacity(nbits: usize) -> Self {
        HandlerResult {
            other: hashmap! {},
            node_ids: BitVec::from_elem(nbits, false),
            skip_ele: BitVec::from_elem(nbits, false),
            input_node_count: 0,
            input_way_count: 0,
            input_relation_count: 0,
            accepted_node_count: 0,
            accepted_way_count: 0,
            accepted_relation_count: 0,
            country_not_found_node_count: 0,
            elevation_not_found_node_count: 0,
            elevation_not_relevant_node_count: 0,
            splitted_way_count: 0,
            added_node_count: 0,
            output_node_count: 0,
            output_way_count: 0,
            output_relation_count: 0,
            available_elevation_tiff_count: 0,
            used_elevation_tiff_count: 0,
            elevation_buffer_flush_count_buffer_max_reached: 0,
            elevation_buffer_flush_count_total_max_reached: 0,
        }
    }

    pub(crate) fn format_multi_line(&self) -> String {
        let input_node_count = self.input_node_count;
        let input_way_count = self.input_way_count;
        let input_relation_count = self.input_relation_count;
        let accepted_node_count = self.accepted_node_count;
        let accepted_way_count = self.accepted_way_count;
        let accepted_relation_count = self.accepted_relation_count;
        let country_not_found_node_count = self.country_not_found_node_count;
        let elevation_not_found_node_count = self.elevation_not_found_node_count;
        let elevation_not_relevant_node_count = self.elevation_not_relevant_node_count;
        let splitted_way_count = self.splitted_way_count;
        let added_node_count = self.added_node_count;
        let output_node_count = self.output_node_count;
        let output_way_count = self.output_way_count;
        let output_relation_count = self.output_relation_count;
        let available_elevation_tiff_count = self.available_elevation_tiff_count;
        let used_elevation_tiff_count = self.used_elevation_tiff_count;
        let elevation_buffer_flush_count_buffer_max_reached = self.elevation_buffer_flush_count_buffer_max_reached;
        let elevation_buffer_flush_count_total_max_reached = self.elevation_buffer_flush_count_total_max_reached;
            format!(r#"input_node_count={input_node_count}
input_way_count={input_way_count}
input_relation_count={input_relation_count}
accepted_node_count={accepted_node_count}
accepted_way_count={accepted_way_count}
accepted_relation_count={accepted_relation_count}
country_not_found_node_count={country_not_found_node_count}
elevation_not_found_node_count={elevation_not_found_node_count}
elevation_not_relevant_node_count={elevation_not_relevant_node_count}
splitted_way_count={splitted_way_count}
added_node_count={added_node_count}
output_node_count={output_node_count}
output_way_count={output_way_count}
output_relation_count={output_relation_count}
available_elevation_tiff_count={available_elevation_tiff_count}
used_elevation_tiff_count={used_elevation_tiff_count}
elevation_buffer_flush_count_buffer_max_reached={elevation_buffer_flush_count_buffer_max_reached}
elevation_buffer_flush_count_total_max_reached={elevation_buffer_flush_count_total_max_reached}"#)
    }

    pub fn format_one_line(&self) -> String {
        self.format_multi_line().replace("\n", ", ")
    }
    pub fn statistics(&self, config: &Config) -> String {
        let input_node_count = self.input_node_count;
        let input_way_count = self.input_way_count;
        let input_relation_count = self.input_relation_count;
        let accepted_node_count = self.accepted_node_count;
        let accepted_way_count = self.accepted_way_count;
        let accepted_relation_count = self.accepted_relation_count;
        let country_not_found_node_count = self.country_not_found_node_count;
        let elevation_not_found_node_count = self.elevation_not_found_node_count;
        let elevation_not_relevant_node_count = self.elevation_not_relevant_node_count;
        let splitted_way_count = self.splitted_way_count;
        let added_node_count = self.added_node_count;
        let output_node_count = self.output_node_count;
        let output_way_count = self.output_way_count;
        let output_relation_count = self.output_relation_count;
        let available_elevation_tiff_count = self.available_elevation_tiff_count;
        let used_elevation_tiff_count = self.used_elevation_tiff_count;
        let elevation_buffer_flush_count_buffer_max_reached = self.elevation_buffer_flush_count_buffer_max_reached;
        let elevation_buffer_flush_count_total_max_reached = self.elevation_buffer_flush_count_total_max_reached;

        // derived values
        let added_nodes = output_node_count - accepted_node_count;

        format!("Element counts at specific processing stages:
                    nodes            ways       relations
read:     {input_node_count:>15} {input_way_count:>15} {input_relation_count:>15}
accepted: {accepted_node_count:>15} {accepted_way_count:>15} {accepted_relation_count:>15}
added:    {added_nodes:>15}               -               -
written:  {output_node_count:>15} {output_way_count:>15} {output_relation_count:>15}

country_not_found_node_count={country_not_found_node_count}
elevation_not_found_node_count={elevation_not_found_node_count}
elevation_not_relevant_node_count={elevation_not_relevant_node_count}
splitted_way_count={splitted_way_count}
added_node_count={added_node_count}

available_elevation_tiff_count={available_elevation_tiff_count}
used_elevation_tiff_count={used_elevation_tiff_count}
elevation_buffer_flush_count_buffer_max_reached={elevation_buffer_flush_count_buffer_max_reached}
elevation_buffer_flush_count_total_max_reached={elevation_buffer_flush_count_total_max_reached}
")
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
                self.handle_element(vec![node], vec![], vec![]);
            },
            Element::Way { way } => {
                if !self.flushed_nodes {
                    self.flush_handlers();
                    self.flushed_nodes = true;
                }
                self.handle_element(vec![], vec![way], vec![])
            },
            Element::Relation { relation } => {
                if !self.flushed_ways {
                    self.flush_handlers();
                    self.flushed_ways = true;
                }
                self.handle_element(vec![], vec![], vec![relation])
            },
            _ => (),
        }
    }

    fn handle_element(&mut self, mut nodes: Vec<Node>, mut ways: Vec<Way>, mut relations: Vec<Relation>) {
        log::trace!("HandlerChain.handle_elements called with {} nodes {} ways {} relations", nodes.len(), ways.len(), relations.len());
        for processor in &mut self.processors {
            if nodes.len() == 0 && ways.len() == 0 && relations.len() == 0 {
                break
            }
            log::trace!("HandlerChain calling {}", processor.name());
            (nodes, ways, relations) = processor.handle_elements(nodes, ways, relations);
        }
    }

    pub(crate) fn flush_handlers(&mut self) {
        log::trace!("HandlerChain.flush_handlers called");
        let mut nodes = vec![];
        let mut ways = vec![];
        let mut relations = vec![];
        for processor in &mut self.processors {
            log::trace!("######");
            log::trace!("###### Flushing {} with {} nodes {} ways {} relations flushed by upstream processors", processor.name(), nodes.len(), ways.len(), relations.len());
            log::trace!("######");
            (nodes, ways, relations) = processor.handle_and_flush_elements(nodes, ways, relations);
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
#[allow(unused_variables)]
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
    use crate::handler::geotiff::{BufferingElevationEnricher, GeoTiffManager, LocationWithElevation};
    use crate::handler::info::*;

    fn existing_tag() -> String { "EXISTING_TAG".to_string() }
    #[allow(dead_code)]
    fn missing_tag() -> String { "MISSING_TAG".to_string() }
    #[allow(dead_code)]
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
    pub fn as_node_element(node: Node) -> Element {
        Element::Node { node: node }
    }
    pub fn as_way_element(way: Way) -> Element {
        Element::Way { way: way }
    }
    pub fn as_relation_element(relation: Relation) -> Element {
        Element::Relation { relation: relation }
    }

    pub fn node_with_ele_from_location(id: i64, location: LocationWithElevation, tags: Vec<(&str, &str)>) -> Node {
        let mut tags_obj: Vec<Tag> = tags.iter().map(|(k, v)| Tag::new(k.to_string(), v.to_string())).collect();
        if &location.ele() != &0.0 { tags_obj.push(Tag::new("ele".to_string(), location.ele().to_string())); }
        Node::new(id, 1, location.get_coordinate(), 1, 1, 1, "a".to_string(), true, tags_obj)
    }
    pub fn node_without_ele_from_location(id: i64, location: LocationWithElevation, tags: Vec<(&str, &str)>) -> Node {
        let tags_obj: Vec<Tag> = tags.iter().map(|(k, v)| Tag::new(k.to_string(), v.to_string())).collect();
        Node::new(id, 1, location.get_coordinate(), 1, 1, 1, "a".to_string(), true, tags_obj)
    }

    fn location_with_elevation_hd_philosophers_way_start() -> LocationWithElevation  { LocationWithElevation::new(8.693313002586367, 49.41470412961422, 125.0)}
    fn location_with_elevation_hd_philosophers_way_end() -> LocationWithElevation  { LocationWithElevation::new(8.707872033119203, 49.41732503427102, 200.0)}

    ///Modify element and return same instance.
    #[derive(Debug, Default)]
    pub(crate) struct TestOnlyElementModifier;
    impl TestOnlyElementModifier {
        fn handle_node(&mut self, node: &mut Node)  {
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

        fn handle_nodes(&mut self, elements: Vec<Node>) -> Vec<Node> {
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

        fn handle_nodes(&mut self, elements: Vec<Node>) -> Vec<Node> {
            elements.iter().for_each(|element| self.handle_element(into_node_element(element.clone())));
            elements
        }

        fn handle_ways(&mut self, elements: Vec<Way>) -> Vec<Way> {
            elements.iter().for_each(|element| self.handle_element(into_way_element(element.clone())));
            elements
        }

        fn handle_relations(&mut self, elements: Vec<Relation>) -> Vec<Relation> {
            elements.iter().for_each(|element| self.handle_element(into_relation_element(element.clone())));
            elements
        }

        fn add_result(&mut self, mut result: HandlerResult) -> HandlerResult {
            result.other.insert(format!("{}", self.name()), self.received_ids.join(", "));
            result
        }
    }

    pub(crate) struct ElementEvaluator {
        id: String,
        node_evaluator: Box<dyn Fn(&Node) -> String>,
        way_evaluator: Box<dyn Fn(&Way) -> String>,
        relation_evaluator: Box<dyn Fn(&Relation) -> String>,
        node_results: BTreeMap<i64, String>,
        way_results: BTreeMap<i64, String>,
        relation_results: BTreeMap<i64, String>,
    }
    impl ElementEvaluator {
        fn new(id: &str,
               node_evaluator: Box<dyn Fn(&Node) -> String>,
               way_evaluator: Box<dyn Fn(&Way) -> String>,
               relation_evaluator: Box<dyn Fn(&Relation) -> String>) -> Self {
            ElementEvaluator {
                id: id.to_string(),
                node_evaluator,
                way_evaluator,
                relation_evaluator,
                node_results: BTreeMap::new(),
                way_results: BTreeMap::new(),
                relation_results: BTreeMap::new(),
            }
        }
    }
    impl Handler for ElementEvaluator {
        fn name(&self) -> String { format!("ElementEvaluator#{}", self.id.clone()) }

        fn handle_nodes(&mut self, elements: Vec<Node>) -> Vec<Node> {
            elements.iter().for_each(|node| {
                self.node_results.insert(node.id(), (self.node_evaluator)(node));
            });
            elements
        }

        fn handle_ways(&mut self, elements: Vec<Way>) -> Vec<Way> {
            elements.iter().for_each(|way| {
                self.way_results.insert(way.id(), (self.way_evaluator)(way));
            });
            elements
        }

        fn handle_relations(&mut self, elements: Vec<Relation>) -> Vec<Relation> {
            elements.iter().for_each(|relation| {
                self.relation_results.insert(relation.id(), (self.relation_evaluator)(relation));
            });
            elements
        }

        fn add_result(&mut self, mut result: HandlerResult) -> HandlerResult {
            result.other.insert(format!("{} node results", self.name()), self.node_results.iter().map(|(k, v)| format!("{}:{}", k, v)).collect::<Vec<String>>().join(", "));
            result.other.insert(format!("{} way results", self.name()), self.way_results.iter().map(|(k, v)| format!("{}:{}", k, v)).collect::<Vec<String>>().join(", "));
            result.other.insert(format!("{} relation results", self.name()), self.relation_results.iter().map(|(k, v)| format!("{}:{}", k, v)).collect::<Vec<String>>().join(", "));

            self.node_results.iter().for_each(|(k, v)| {
                result.other.insert(format!("{}:node#{}", self.name(), k), v.clone());
            });
            self.way_results.iter().for_each(|(k, v)| {
                result.other.insert(format!("{}:way#{}", self.name(), k), v.clone());
            });
            self.relation_results.iter().for_each(|(k, v)| {
                result.other.insert(format!("{}:relation#{}", self.name(), k), v.clone());
            });
            result
        }
    }

    #[test]
    /// Assert that it is possible to run a chain of processors.
    fn test_chain() {
        let mut processor_chain = HandlerChain::default()
            .add(ElementCounter::new(ElementCountResultType::InputCount))
            .add(TestOnlyOrderRecorder::new("initial"))

            .add(TestOnlyIdCollector::new(10))

            .add(ElementPrinter::with_prefix("final: ".to_string()).with_node_ids(hashset! {8}))
            .add(TestOnlyOrderRecorder::new("final"))
            .add(ElementCounter::new(ElementCountResultType::OutputCount))
            ;
        processor_chain.process(simple_node_element(1, vec![("who", "kasper")]));
        processor_chain.process(simple_node_element(2, vec![("who", "seppl")]));
        processor_chain.process(simple_node_element(6, vec![("who", "hotzenplotz")]));
        processor_chain.process(simple_node_element(8, vec![("who", "großmutter")]));
        processor_chain.flush_handlers();
        let result = processor_chain.collect_result();

        assert_element_counts(&result, 4, 4,
                              0, 0,
                              0, 0);
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
            .add(ElementCounter::new(ElementCountResultType::InputCount))
            .add(TestOnlyOrderRecorder::new("initial"))

            .add(TestOnlyElementBufferingDuplicatingEditingProcessor::default())

            .add(ElementPrinter::with_prefix("final".to_string()).with_node_ids((1..=200).collect()))
            .add(TestOnlyOrderRecorder::new("final"))
            .add(ElementCounter::new(ElementCountResultType::OutputCount))
            ;

        processor_chain.process(simple_node_element(1, vec![("who", "kasper")]));
        processor_chain.process(simple_node_element(2, vec![("who", "seppl")]));
        processor_chain.process(simple_node_element(6, vec![("who", "hotzenplotz")]));
        processor_chain.process(simple_node_element(8, vec![("who", "großmutter")]));
        processor_chain.process(simple_way_element(23, vec![1, 2, 8, 6], vec![("who", "kasper")]));
        processor_chain.process(simple_relation_element(66, vec![(MemberType::Way, 23, "kasper&seppl brign großmutter to hotzenplotz")], vec![("who", "großmutter")]));
        processor_chain.flush_handlers();
        let result = processor_chain.collect_result();

        assert_element_counts(&result, 4, 8,
                              1, 1,
                              1, 1);
        assert_eq!(&result.other.get("TestOnlyOrderRecorder initial").unwrap().clone(), "node#1, node#2, node#6, node#8, way#23, relation#66");
        assert_eq!(&result.other.get("TestOnlyOrderRecorder final").unwrap().clone(), "node#1, node#101, node#2, node#102, node#6, node#106, node#8, node#108, way#23, relation#66");
    }

    #[test]
    /// Assert that it is possible to run the chain and let processors receive one element
    /// and add additional elements of the same type to the processing chain
    /// that are processed by downstream processors.
    /// The test uses TestOnlyElementAdder for this.
    fn test_chain_with_element_adder() {
        let _ = SimpleLogger::new().init();
        let mut processor_chain = HandlerChain::default()
            .add(ElementCounter::new(ElementCountResultType::InputCount))
            .add(TestOnlyOrderRecorder::new("initial"))

            .add(TestOnlyElementAdder::default())

            .add(ElementPrinter::with_prefix("final".to_string()).with_node_ids((1..=200).collect()))
            .add(TestOnlyOrderRecorder::new("final"))
            .add(ElementCounter::new(ElementCountResultType::OutputCount))
            ;

        processor_chain.process(simple_way_element(23, vec![1, 2, 8, 6], vec![("who", "kasper")]));
        processor_chain.process(simple_node_element(1, vec![("who", "kasper")]));
        processor_chain.process(simple_node_element(2, vec![("who", "seppl")]));
        processor_chain.process(simple_node_element(6, vec![("who", "hotzenplotz")]));
        processor_chain.process(simple_node_element(8, vec![("who", "großmutter")]));
        processor_chain.flush_handlers();
        let result = processor_chain.collect_result();

        assert_element_counts(&result, 4, 8,
                              0, 0,
                              1, 1);
        assert_eq!(&result.other.get("TestOnlyOrderRecorder initial").unwrap().clone(), "way#23, node#1, node#2, node#6, node#8");
        assert_eq!(&result.other.get("TestOnlyOrderRecorder final").unwrap().clone(), "way#23, node#101, node#1, node#102, node#2, node#106, node#6, node#108, node#8");
    }

    #[test]
    /// Assert that it is possible to run the chain and let processors permanently filter (remove) elements.
    /// The test uses TestOnlyElementFilter for this, which filters nodes with an even id.
    fn test_chain_with_element_filter() {
        let _ = SimpleLogger::new().init();
        let mut processor_chain = HandlerChain::default()
            .add(ElementCounter::new(ElementCountResultType::InputCount))
            .add(TestOnlyOrderRecorder::new("initial"))

            .add(TestOnlyElementFilter::default())

            .add(ElementPrinter::with_prefix("final".to_string()).with_node_ids((1..=200).collect()))
            .add(TestOnlyOrderRecorder::new("final"))
            .add(ElementCounter::new(ElementCountResultType::OutputCount))
            ;

        processor_chain.process(simple_node_element(1, vec![("who", "kasper")]));
        processor_chain.process(simple_node_element(2, vec![("who", "seppl")]));
        processor_chain.process(simple_node_element(6, vec![("who", "hotzenplotz")]));
        processor_chain.process(simple_node_element(8, vec![("who", "großmutter")]));
        processor_chain.flush_handlers();
        let result = processor_chain.collect_result();

        assert_element_counts(&result, 4, 1,
                              0, 0,
                              0, 0);
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
            .add(ElementCounter::new(ElementCountResultType::InputCount))
            .add(TestOnlyOrderRecorder::new("initial"))

            .add(TestOnlyElementReplacer::default())

            .add(ElementPrinter::with_prefix("final".to_string()).with_node_ids((1..=200).collect()))
            .add(TestOnlyOrderRecorder::new("final"))
            .add(ElementCounter::new(ElementCountResultType::OutputCount))
            ;

        processor_chain.process(simple_node_element(1, vec![("who", "kasper")]));
        processor_chain.process(simple_node_element(2, vec![("who", "seppl")]));
        processor_chain.process(simple_node_element(6, vec![("who", "hotzenplotz")]));
        processor_chain.process(simple_node_element(8, vec![("who", "großmutter")]));
        processor_chain.flush_handlers();
        let result = processor_chain.collect_result();

        assert_element_counts(&result,
                              4, 4,
                              0, 0,
                              0, 0);
        assert_eq!(&result.other.get("TestOnlyOrderRecorder initial").unwrap().clone(), "node#1, node#2, node#6, node#8");
        assert_eq!(&result.other.get("TestOnlyOrderRecorder final").unwrap().clone(), "node#1, node#2, node#66, node#8");
    }

    fn assert_element_counts(result: &HandlerResult, input_node_count: u64, output_node_count: u64, input_relation_count: u64, output_relation_count: u64, input_way_count: u64, output_way_count: u64) {
        assert_eq!(&result.input_node_count, &input_node_count);
        assert_eq!(&result.output_node_count, &output_node_count);
        assert_eq!(&result.input_relation_count, &input_relation_count);
        assert_eq!(&result.output_relation_count, &output_relation_count);
        assert_eq!(&result.input_way_count, &input_way_count);
        assert_eq!(&result.output_way_count, &output_way_count);
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
            .add(ElementCounter::new(ElementCountResultType::InputCount))
            .add(TestOnlyOrderRecorder::new("initial"))

            .add(TestOnlyElementModifier::default())
            .add(TagKeyBasedOsmElementsFilter::new(OsmElementTypeSelection::node_only(), vec!["added".to_string()], FilterType::AcceptMatching))

            .add(ElementPrinter::with_prefix("final".to_string()).with_node_ids((1..=200).collect()))
            .add(TestOnlyOrderRecorder::new("final"))
            .add(ElementCounter::new(ElementCountResultType::OutputCount))
            ;

        processor_chain.process(simple_node_element(1, vec![("who", "kasper")]));
        processor_chain.process(simple_node_element(2, vec![("who", "seppl")]));
        processor_chain.process(simple_node_element(6, vec![("who", "hotzenplotz")]));
        processor_chain.process(simple_node_element(8, vec![("who", "großmutter")]));
        processor_chain.flush_handlers();
        let result = processor_chain.collect_result();

        assert_element_counts(&result, 4,
                              //kasper with odd id was not modified and later filtered:
                              3,
                              0, 0,
                              0, 0);
        assert_eq!(&result.other.get("TestOnlyOrderRecorder initial").unwrap().clone(), "node#1, node#2, node#6, node#8");
        assert_eq!(&result.other.get("TestOnlyOrderRecorder final").unwrap().clone(), "node#2, node#6, node#8");
    }

    #[test]
    fn handler_chain() {
        let _ = SimpleLogger::new().init();
        let chain = HandlerChain::default()
            .add(ElementCounter::new(ElementCountResultType::InputCount))
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
            .add(ElementCounter::new(ElementCountResultType::OutputCount))
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
            .add(ElementCounter::new(ElementCountResultType::InputCount))
            .add(NodeIdFilter { node_ids: node_ids.clone() })
            .add(ElementCounter::new(ElementCountResultType::OutputCount))
            .add(TestOnlyIdCollector::new(100));

        handle_test_nodes_and_verify_result(chain);
    }

    fn handle_test_nodes_and_verify_result(mut handler_chain: HandlerChain) {
        handler_chain.process(simple_node_element(1, vec![(existing_tag().as_str(), "kasper")]));
        handler_chain.process(simple_node_element(2, vec![(existing_tag().as_str(), "seppl")]));
        handler_chain.process(simple_node_element(3, vec![(existing_tag().as_str(), "hotzenplotz")]));
        handler_chain.process(simple_node_element(4, vec![(existing_tag().as_str(), "großmutter")]));

        handler_chain.flush_handlers();
        let result = handler_chain.collect_result();
        assert_element_counts(&result,
                              4, 2,
                              0, 0,
                              0, 0);
        assert_eq!(result.node_ids[0], false);
        assert_eq!(result.node_ids[1], true);
        assert_eq!(result.node_ids[2], true);
        assert_eq!(result.node_ids[3], false);
        assert_eq!(result.node_ids[4], false);
    }

    #[test]
    fn handler_chain_with_buffering_elevation_enricher_adds_new_nodes_with_elevation() {
        let _ = SimpleLogger::new().init();
        let mut node_ids = BitVec::from_elem(10usize, false);
        node_ids.set(1usize, true);
        node_ids.set(2usize, true);
        let mut handler = BufferingElevationEnricher::new(
            GeoTiffManager::with_file_pattern("test/region*.tif"),
            5,
            6,
            BitVec::from_elem(10usize, false),
            true,
            0.01,
            0.01);

        let mut handler_chain = HandlerChain::default()
            .add(ElementCounter::new(ElementCountResultType::InputCount))
            .add(TestOnlyOrderRecorder::new("initial"))
            .add(handler)
            .add(TestOnlyOrderRecorder::new("final"))
            .add(ElementCounter::new(ElementCountResultType::OutputCount))
            .add(ElementEvaluator::new("elevation",
                                       Box::new(|node| node.tags().iter().any(|tag| tag.k() == "ele").to_string()),
                                       Box::new(|_| "".to_string()),
                                       Box::new(|_| "".to_string())))
            .add(ElementEvaluator::new("way_refs",
                                       Box::new(|_| "".to_string()),
                                       Box::new(|way| way.refs().iter().map(|id| id.to_string()).collect::<Vec<String>>().join(",")),
                                       Box::new(|_| "".to_string())))
            ;

        handler_chain.process(as_node_element(node_without_ele_from_location(101, location_with_elevation_hd_philosophers_way_start(), vec![])));
        handler_chain.process(as_node_element(node_without_ele_from_location(102, location_with_elevation_hd_philosophers_way_end(), vec![])));
        handler_chain.process(as_way_element(simple_way(201, vec![101, 102], vec![])));
        handler_chain.flush_handlers();
        let result = handler_chain.collect_result();

        // dbg!(&result); // This causes the test to run eternally...?!

        assert_element_counts(&result, 2, 3,
                              0, 0,
                              1, 1);
        assert_eq!(&result.other.get("TestOnlyOrderRecorder initial").unwrap().clone(), "node#101, node#102, way#201");
        assert_eq!(&result.other.get("TestOnlyOrderRecorder final").unwrap().clone(), format!("node#101, node#102, node#{}, way#201", HIGHEST_NODE_ID+1).as_str());
        assert_eq!(&result.other.get("ElementEvaluator#elevation node results").unwrap().clone(), format!("101:true, 102:true, {}:true", HIGHEST_NODE_ID+1).as_str() );
        assert_eq!(&result.other.get("ElementEvaluator#way_refs:way#201").unwrap().clone(), format!("101,{},102",HIGHEST_NODE_ID+1).as_str());
    }
}
