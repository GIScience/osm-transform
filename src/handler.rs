pub(crate) mod collect;
pub(crate) mod filter;
pub(crate) mod predicate;
pub(crate) mod info;
pub(crate) mod modify;
pub mod geotiff;
pub(crate) mod interpolate;
pub(crate) mod skip_ele;

use std::collections::HashMap;

use bit_vec::BitVec;
use log::trace;
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
    fn handle(&mut self, _result: &mut HandlerResult){}

    fn flush(&mut self, result: &mut HandlerResult)  {
        self.handle(result)
    }

    fn close(&mut self, _result: &mut HandlerResult) {}
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

#[derive(Debug)]
pub struct HandlerResult {

    ///Elements to handle
    pub nodes: Vec<Node>,
    pub ways: Vec<Way>,
    pub relations: Vec<Relation>,

    /// Intermediate filter results created by the handlers in the first pass
    /// and consumed by filters in the second pass.
    pub node_ids: BitVec, //todo rename to accept_node_ids
    //todo add: pub accept_way_ids: BitVec,
    //todo add: pub accept_relation_ids: BitVec,
    pub skip_ele: BitVec, //todo rename to nodes_without_elevation_ids
    
    /// InputCount
    pub input_node_count: u64,
    pub input_way_count: u64,
    pub input_relation_count: u64,

    /// AcceptedCount
    pub accepted_node_count: u64,      //Referenced by accepted ways or relations
    pub accepted_way_count: u64,       //Not filtered by tags
    pub accepted_relation_count: u64,  //Not filtered by tags

    /// OutputCount
    pub output_node_count: u64,
    pub output_way_count: u64,
    pub output_relation_count: u64,

    /// Country mapping statistics
    pub country_found_node_count: u64,
    pub country_not_found_node_count: u64,

    /// Elevation enrichment statistics
    pub elevation_found_node_count: u64,
    pub elevation_not_found_node_count: u64,
    pub elevation_not_relevant_node_count: u64,
    pub elevation_flush_count: u64,
    pub elevation_total_buffered_nodes_max_reached_count: u64,
    pub splitted_way_count: u64,
    pub elevation_tiff_count_total: u64,
    pub elevation_tiff_count_used: u64,

    /// Other statistics
    pub other: HashMap<String, String>,
}
impl HandlerResult {
    pub(crate) fn default() -> Self {
        Self::with_capacity(HIGHEST_NODE_ID as usize)
    }
    pub(crate) fn with_capacity(nbits: usize) -> Self {
        HandlerResult {
            nodes: vec![],
            ways: vec![],
            relations: vec![],

            node_ids: BitVec::from_elem(nbits, false),
            skip_ele: BitVec::from_elem(nbits, false),

            input_node_count: 0,
            input_way_count: 0,
            input_relation_count: 0,
            accepted_node_count: 0,
            accepted_way_count: 0,
            accepted_relation_count: 0,
            country_not_found_node_count: 0,
            country_found_node_count: 0,
            elevation_found_node_count: 0,
            elevation_not_found_node_count: 0,
            elevation_not_relevant_node_count: 0,
            splitted_way_count: 0,
            output_node_count: 0,
            output_way_count: 0,
            output_relation_count: 0,
            elevation_tiff_count_total: 0,
            elevation_tiff_count_used: 0,
            elevation_flush_count: 0,
            elevation_total_buffered_nodes_max_reached_count: 0,

            other: hashmap! {},
        }
    }
    pub(crate) fn with_nodes(&self, nodes: Vec<Node>) -> Self {
        let mut result = Self::default();
        result.nodes = nodes;
        result
    }

    pub(crate) fn format_multi_line(&self) -> String {
        let input_node_count = self.input_node_count;
        let input_way_count = self.input_way_count;
        let input_relation_count = self.input_relation_count;
        let accepted_node_count = self.accepted_node_count;
        let accepted_way_count = self.accepted_way_count;
        let accepted_relation_count = self.accepted_relation_count;
        let output_node_count = self.output_node_count;
        let output_way_count = self.output_way_count;
        let output_relation_count = self.output_relation_count;
        let country_not_found_node_count = self.country_not_found_node_count;
        let country_found_node_count = self.country_found_node_count;
        let elevation_found_node_count = self.elevation_found_node_count;
        let elevation_not_found_node_count = self.elevation_not_found_node_count;
        let elevation_not_relevant_node_count = self.elevation_not_relevant_node_count;
        let splitted_way_count = self.splitted_way_count;
        let elevation_tiff_count_total = self.elevation_tiff_count_total;
        let elevation_tiff_count_used = self.elevation_tiff_count_used;
        let elevation_buffer_flush_count_buffer_max_reached = self.elevation_flush_count;
        let elevation_buffer_flush_count_total_max_reached = self.elevation_total_buffered_nodes_max_reached_count;
        let other = self.other.iter().map(|(k, v)| format!("    {}={}", k, v)).collect::<Vec<String>>().join("\n");
            format!(r#"input_node_count={input_node_count}
input_way_count={input_way_count}
input_relation_count={input_relation_count}
accepted_node_count={accepted_node_count}
accepted_way_count={accepted_way_count}
accepted_relation_count={accepted_relation_count}
output_node_count={output_node_count}
output_way_count={output_way_count}
output_relation_count={output_relation_count}
country_not_found_node_count={country_not_found_node_count}
country_found_node_count={country_found_node_count}
elevation_found_node_count={elevation_found_node_count}
elevation_not_found_node_count={elevation_not_found_node_count}
elevation_not_relevant_node_count={elevation_not_relevant_node_count}
splitted_way_count={splitted_way_count}
elevation_tiff_count_total={elevation_tiff_count_total}
elevation_tiff_count_used={elevation_tiff_count_used}
elevation_buffer_flush_count_buffer_max_reached={elevation_buffer_flush_count_buffer_max_reached}
elevation_buffer_flush_count_total_max_reached={elevation_buffer_flush_count_total_max_reached}
other={other}"#)
    }

    pub fn format_one_line(&self) -> String {
        self.format_multi_line().replace("\n", ", ")
    }
    pub fn statistics(&self, config: &Config) -> String {
        if config.statistics_level == 0 {
            return "".to_string()
        }
        let i_node_ct = self.input_node_count;
        let i_way_cnt = self.input_way_count;
        let i_rel_cnt = self.input_relation_count;
        let a_node_ct = self.accepted_node_count;
        let a_way_cnt = self.accepted_way_count;
        let a_rel_cnt = self.accepted_relation_count;
        let o_node_ct = self.output_node_count;
        let o_way_cnt = self.output_way_count;
        let o_rel_cnt = self.output_relation_count;

        let country_not_found_node_count = self.country_not_found_node_count;
        let country_found_node_count = self.country_found_node_count;

        let elevation_found_node_count = self.elevation_found_node_count;
        let elevation_not_found_node_count = self.elevation_not_found_node_count;
        let elevation_not_relevant_node_count = self.elevation_not_relevant_node_count;
        let splitted_way_count = self.splitted_way_count;
        let elevation_tiff_count_total = self.elevation_tiff_count_total;
        let elevation_tiff_count_used = self.elevation_tiff_count_used;
        let elevation_flush_count = self.elevation_flush_count;
        let elevation_total_buffered_nodes_max_reached_count = self.elevation_total_buffered_nodes_max_reached_count;

        // derived values
        let filt_node = (i_node_ct as i64 - a_node_ct as i64) * -1;
        let filt_ways = (i_way_cnt as i64 - a_way_cnt as i64) * -1;
        let filt_rels = (i_rel_cnt as i64 - a_rel_cnt as i64) * -1;
        let addd_node = o_node_ct as i64 - a_node_ct as i64;
        let addd_ways = o_way_cnt as i64 - a_way_cnt as i64;
        let addd_rels = o_rel_cnt as i64 - a_rel_cnt as i64;
        let country_detections = country_found_node_count + country_not_found_node_count;
        let elevation_detections = elevation_found_node_count + elevation_not_found_node_count + elevation_not_relevant_node_count;
        let unsplitted_way_count = o_way_cnt - splitted_way_count;
        let mut formatted_statistics = format!("
Summary:
========

Processing of file TODO (TODO bytes) completed in TODO seconds.

Element counts at specific processing stages:
---------------------------------------------

         |            nodes            |            ways             |          relations
         |                             |                             |
         |         diff |        total |         diff |        total |         diff |        total
---------+--------------+--------------+--------------+--------------+--------------+-------------
    read |{i_node_ct:+13} |{i_node_ct:>13} |{i_way_cnt:+13} |{i_way_cnt:>13} |{i_rel_cnt:+13} |{i_rel_cnt:>13}
TODO add custom filter counts
accepted |{filt_node:+13} |{a_node_ct:>13} |{filt_ways:+13} |{a_way_cnt:>13} |{filt_rels:+13} |{a_rel_cnt:>13}
 written |{addd_node:+13} |{o_node_ct:>13} |{addd_ways:+13} |{o_way_cnt:>13} |{addd_rels:+13} |{o_rel_cnt:>13}

 ");
        if config.statistics_level == 1 {
            return formatted_statistics
        }

        match &config.country_csv {
            Some(_) => {
                formatted_statistics.push_str(format!("

Country enrichment:
-------------------
Country detected for      {country_found_node_count:>13} nodes (TODO% of all accepted nodes)
>1 country detected for   TODO nodes
Country not found for     {country_not_found_node_count:>13} nodes
Country detections total: {country_detections:>13}
TODO: nodes per country (ordered by node count, ¹ percentage of all accepted nodes):
         1234567 (0.12%¹) Malta
         1234567 (0.12%¹) Malta
         1234567 (0.12%¹) Malta
").as_str());
            }
            None => {}
        }

        if &config.elevation_tiffs.len() > &0 {
            formatted_statistics.push_str(format!("

Elevation enrichment:
---------------------
Elevation detected for      {elevation_found_node_count:>13} nodes
Elevation not found for     {elevation_not_found_node_count:>13} nodes
Elevation not relevant for  {elevation_not_relevant_node_count:>13} nodes (tunnels, bridges, ...)
Elevation detections total: {elevation_detections:>13}
").as_str());

            if config.statistics_level > 3 {
                formatted_statistics.push_str(format!("
Loaded elevation tiff files:      {elevation_tiff_count_total:>5}
Used elevation tiff files:        {elevation_tiff_count_used:>5}
Elevation buffers flush count:    {elevation_flush_count:>5}
TODO: nodes per tiff file (ordered by node count, ¹ percentage of all accepted nodes):
         1234567 (23.14%¹) srtm_32_03.tif
         1234567 (23.14%¹) srtm_32_03.tif
         1234567 (23.14%¹) srtm_32_03.tif
Total max buffered nodes reached: {elevation_total_buffered_nodes_max_reached_count:>13} times


").as_str())
            }
        if config.elevation_way_splitting {
            formatted_statistics.push_str(format!("

Elevation way splitting:
------------------------
Added {addd_node} nodes to {splitted_way_count:>13} ways (TODO% of all accepted ways)
Unsplitted ways:           {unsplitted_way_count:>13} (TODO% of all accepted ways)
").as_str());
        }

        }
        formatted_statistics
    }

    pub(crate) fn clear(&mut self) {
        self.clear_elements();
        self.clear_ids();
        self.clear_counts();
    }
    pub(crate) fn clear_elements(&mut self) {
        self.nodes.clear();
        self.ways.clear();
        self.relations.clear();
    }
    pub(crate) fn clear_ids(&mut self) {
        self.node_ids.clear();
        //todo self.way_ids.clear();
        //todo self.relation_ids.clear();
        self.skip_ele.clear();
    }
    pub(crate) fn clear_counts(&mut self) {
        self.input_node_count = 0;
        self.input_way_count = 0;
        self.input_relation_count = 0;
        self.clear_non_input_counts();
    }
    pub(crate) fn clear_non_input_counts(&mut self){
        self.accepted_node_count = 0;
        self.accepted_way_count = 0;
        self.accepted_relation_count = 0;
        self.country_not_found_node_count = 0;
        self.country_found_node_count = 0;
        self.elevation_found_node_count = 0;
        self.elevation_not_found_node_count = 0;
        self.elevation_not_relevant_node_count = 0;
        self.splitted_way_count = 0;
        self.output_node_count = 0;
        self.output_way_count = 0;
        self.output_relation_count = 0;
        self.elevation_tiff_count_total = 0;
        self.elevation_tiff_count_used = 0;
        self.elevation_flush_count = 0;
        self.elevation_total_buffered_nodes_max_reached_count = 0;
        self.other.clear();
    }
    pub(crate) fn input_element_count(&self) -> u64 {
        self.input_node_count + self.input_way_count + self.input_relation_count
    }
}


#[derive(Default)]
pub(crate) struct HandlerChain {
    pub processors: Vec<Box<dyn Handler>>,
    first_node_received: bool,
    flushed_nodes: bool,
    flushed_ways:  bool,
}
impl HandlerChain {
    pub(crate) fn add(mut self, processor: impl Handler + Sized + 'static) -> HandlerChain {
        self.processors.push(Box::new(processor));
        self
    }
    pub(crate) fn process(&mut self, element: Element, result: &mut HandlerResult) {
        trace!("######");
        trace!("###### Processing {}", format_element_id(&element));
        trace!("######");
        result.clear_elements();
        match element {
            Element::Node { node } => {
                if !self.first_node_received {
                    log::info!("Reading nodes...");
                    self.first_node_received = true;
                }
                result.nodes.push(node.clone());
                self.handle_element(vec![node], vec![], vec![], result);
            },
            Element::Way { way } => {
                if !self.flushed_nodes {
                    self.flush_handlers(result);
                    self.flushed_nodes = true;
                    log::info!("Reading ways...");
                }
                result.ways.push(way.clone());
                self.handle_element(vec![], vec![way], vec![], result)
            },
            Element::Relation { relation } => {
                if !self.flushed_ways {
                    self.flush_handlers(result);
                    self.flushed_ways = true;
                    log::info!("Reading relations...");
                }
                result.relations.push(relation.clone());
                self.handle_element(vec![], vec![], vec![relation], result)
            },
            _ => (),
        }
        result.clear_elements();
    }

    fn handle_element(&mut self, nodes: Vec<Node>, ways: Vec<Way>, relations: Vec<Relation>, result: &mut HandlerResult) {
        trace!("HandlerChain.handle_elements called with {} nodes {} ways {} relations", nodes.len(), ways.len(), relations.len());
        for processor in &mut self.processors {
            if nodes.len() == 0 && ways.len() == 0 && relations.len() == 0 {
                break
            }
            trace!("HandlerChain.handle_element calling {} with result {}", processor.name(), result.format_one_line());
            // (nodes, ways, relations) = processor.handle_elements(nodes, ways, relations);
            processor.handle(result);
        }
    }

    pub(crate) fn flush_handlers(&mut self, result: &mut HandlerResult) {
        trace!("######");
        trace!("###### HandlerChain.flush_handlers called with {} nodes {} ways {} relations", result.nodes.len(), result.ways.len(), result.relations.len());
        trace!("######");
        for processor in &mut self.processors {
            trace!("HandlerChain.flush_handlers calling {} with result {}", processor.name(), result.format_one_line());
            // (nodes, ways, relations) = processor.handle_and_flush_elements(nodes, ways, relations);
            processor.flush(result)
        }
        trace!("HandlerChain.flush_handlers all handlers have flushed - clearing elements");
        result.clear_elements();
    }

    pub(crate) fn collect_result(&mut self, result: &mut HandlerResult) {
        for processor in &mut self.processors {
            processor.close(result);
        }
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
    use std::collections::BTreeMap;
    use crate::handler::collect::ReferencedNodeIdCollector;

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
        Element::Relation { relation: simple_relation(id, members, tags) }
    }
    pub fn simple_relation(id: i64, members: Vec<(MemberType, i64, &str)>, tags: Vec<(&str, &str)>) -> Relation {
        let members_obj = members.iter().map(|(t, id, role)| {
            match t {
                MemberType::Node => { Member::Node { member: MemberData::new(id.clone(), role.to_string()) } }
                MemberType::Way => { Member::Way { member: MemberData::new(id.clone(), role.to_string()) } }
                MemberType::Relation => { Member::Relation { member: MemberData::new(id.clone(), role.to_string()) } }
            }
        }).collect();
        let tags_obj = tags.iter().map(|(k, v)| Tag::new(k.to_string(), v.to_string())).collect();
        Relation::new(id, 1, 1, 1, 1, "a_user".to_string(), true, members_obj, tags_obj)
    }
    pub fn node_element(id: i64, version: i32, coordinate: Coordinate, timestamp: i64, changeset: i64, uid: i32, user: String, visible: bool, tags: Vec<Tag>) -> Element {
        Element::Node { node: Node::new(id, version, coordinate, timestamp, changeset, uid, user, visible, tags) }
    }
    pub fn way_element(id: i64, version: i32, timestamp: i64, changeset: i64, uid: i32, user: String, visible: bool, refs: Vec<i64>, tags: Vec<Tag>) -> Element {
        Element::Way { way: Way::new(id, version, timestamp, changeset, uid, user, visible, refs, tags) }
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

        fn handle(&mut self, result: &mut HandlerResult) {
            result.nodes.iter_mut().for_each(|node| self.handle_node(node));
        }

    }

    ///Return a copy of the element, e.g. a different instance.
    #[derive(Debug, Default)]
    pub(crate) struct TestOnlyElementReplacer;
    impl Handler for TestOnlyElementReplacer {
        fn name(&self) -> String { "TestOnlyElementReplacer".to_string() }

        fn handle(&mut self, result: &mut HandlerResult) {
            for index in 0..result.nodes.len() {
                if result.nodes[index].id() == 6 {
                    result.nodes[index] = simple_node(66, vec![("who", "dimpfelmoser")]);
                }
            }
        }
    }

    ///Remove an element / return empty vec.
    #[derive(Debug, Default)]
    pub(crate) struct TestOnlyElementFilter;
    impl Handler for TestOnlyElementFilter {
        fn name(&self) -> String { "TestOnlyElementFilter".to_string() }

        fn handle(&mut self, result: &mut HandlerResult) {
            result.nodes.retain(|node| node.id() % 2 != 0 );
        }
    }

    ///Receive one element, return two of the same type.
    #[derive(Debug, Default)]
    pub(crate) struct TestOnlyElementAdder;
    impl TestOnlyElementAdder {}
    impl Handler for TestOnlyElementAdder {
        fn name(&self) -> String { "TestOnlyElementAdder".to_string() }

        fn handle(&mut self, result: &mut HandlerResult) {
            let mut new_nodes = vec![];
            result.nodes.iter().for_each(|node| new_nodes.push(copy_node_with_new_id(node, node.id().add(100))));
            result.nodes.extend(new_nodes);
        }
    }

    #[derive(Default, Debug)]
    pub(crate) struct TestOnlyElementBufferingDuplicatingEditingProcessor { //store received elements, when receiving the 5th, emit all 5 and start buffering again. flush: emit currently buffered. handling the elements (changing) happens before emitting
        nodes_cache: Vec<Node>,
        ways_cache: Vec<Way>,
        relations_cache: Vec<Relation>,
    }
    impl TestOnlyElementBufferingDuplicatingEditingProcessor {
        fn handle_node(&self, node: Node) -> Vec<Node> {
            let mut node_clone = copy_node_with_new_id(&node, node.id().add(100));
            node_clone.tags_mut().push(Tag::new("elevation".to_string(), "default-elevation".to_string()));
            vec![node, node_clone]
        }

        fn handle_nodes(&mut self, result: &mut HandlerResult) {
            self.nodes_cache.append(&mut result.nodes);
            result.nodes.clear();
            if self.nodes_cache.len() >= 3 {
                self.handle_and_flush_nodes(result);
            }
        }

        fn handle_ways(&mut self, result: &mut HandlerResult) {
            self.ways_cache.append(&mut result.ways);
            result.ways.clear();
            if self.ways_cache.len() >= 3 {
                self.flush_ways(result);
            }
        }

        fn flush_ways(&mut self, result: &mut HandlerResult) {
            result.ways.append(&mut self.ways_cache);
            self.ways_cache.clear();
        }


        fn handle_relations(&mut self, result: &mut HandlerResult) {
            self.relations_cache.append(&mut result.relations);
            result.relations.clear();
            if self.relations_cache.len() >= 3 {
                self.flush_relations(result);
            }
        }

        fn flush_relations(&mut self, result: &mut HandlerResult) {
            result.relations.append(&mut self.relations_cache);
            self.relations_cache.clear();
        }

        fn handle_and_flush_nodes(&mut self, result: &mut HandlerResult) {
            let mut result_vec = Vec::new();
            self.nodes_cache.iter().for_each(|node| result_vec.extend(self.handle_node(node.clone())));
            result.nodes = result_vec;
            self.nodes_cache.clear();
        }
    }
    impl Handler for TestOnlyElementBufferingDuplicatingEditingProcessor {
        // fn struct_name() -> &'static str { "TestOnlyElementBuffer" }
        fn name(&self) -> String { "TestOnlyElementBuffer".to_string() }

        fn handle(&mut self, result: &mut HandlerResult) {
            self.handle_nodes(result);
            self.handle_ways(result);
            self.handle_relations(result);
        }
        fn flush(&mut self, result: &mut HandlerResult)  {
            self.handle_and_flush_nodes(result);
            self.flush_ways(result);
            self.flush_relations(result);
        }

    }

    #[derive(Debug)]
    pub(crate) struct TestOnlyIdCollector {
        pub way_ids: BitVec,
        pub relation_ids: BitVec,
    }
    impl TestOnlyIdCollector {
        pub fn new(nbits: usize) -> Self {
            TestOnlyIdCollector {
                way_ids: BitVec::from_elem(nbits, false),
                relation_ids: BitVec::from_elem(nbits, false),
            }
        }
    }
    impl Handler for TestOnlyIdCollector {
        fn name(&self) -> String { "TestOnlyIdCollector".to_string() }

        fn handle(&mut self, result: &mut HandlerResult) {
            result.nodes.iter().for_each(|node| result.node_ids.set(node.id() as usize, true));

            result.ways.iter().for_each(|way| self.way_ids.set(way.id() as usize, true));
            result.relations.iter().for_each(|relation| self.relation_ids.set(relation.id() as usize, true));
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
    impl Handler for TestOnlyOrderRecorder {
        fn name(&self) -> String { format!("TestOnlyOrderRecorder {}", self.result_key) }

        fn handle(&mut self, result: &mut HandlerResult) {
            result.nodes.iter().for_each(|node| self.received_ids.push(format!("node#{}", node.id().to_string())));
            result.ways.iter().for_each(|node| self.received_ids.push(format!("way#{}", node.id().to_string())));
            result.relations.iter().for_each(|node| self.received_ids.push(format!("relation#{}", node.id().to_string())));
        }

        fn close(&mut self, result: &mut HandlerResult) {
            result.other.insert(format!("{}", self.name()), self.received_ids.join(", "));
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

        fn handle(&mut self, result: &mut HandlerResult) {
            result.nodes.iter().for_each(|node| {
                self.node_results.insert(node.id(), (self.node_evaluator)(node));
            });
            result.ways.iter().for_each(|way| {
                self.way_results.insert(way.id(), (self.way_evaluator)(way));
            });
            result.relations.iter().for_each(|relation| {
                self.relation_results.insert(relation.id(), (self.relation_evaluator)(relation));
            });
        }

        fn close(&mut self, result: &mut HandlerResult) {
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
        }
    }

    #[test]
    /// Assert that it is possible to run a chain of processors.
    fn test_chain() {
        let mut result = HandlerResult::default();
        let mut processor_chain = HandlerChain::default()
            .add(ElementCounter::new(ElementCountResultType::InputCount))
            .add(TestOnlyOrderRecorder::new("initial"))

            .add(TestOnlyIdCollector::new(10))

            .add(ElementPrinter::with_prefix("final: ".to_string()).with_node_ids(hashset! {8}))
            .add(TestOnlyOrderRecorder::new("final"))
            .add(ElementCounter::new(ElementCountResultType::OutputCount))
            ;
        processor_chain.process(simple_node_element(1, vec![("who", "kasper")]), &mut result);
        processor_chain.process(simple_node_element(2, vec![("who", "seppl")]), &mut result);
        processor_chain.process(simple_node_element(6, vec![("who", "hotzenplotz")]), &mut result);
        processor_chain.process(simple_node_element(8, vec![("who", "großmutter")]), &mut result);
        processor_chain.flush_handlers(&mut result);
        processor_chain.collect_result(&mut result);

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
        let mut result = HandlerResult::default();
        let mut processor_chain = HandlerChain::default()
            .add(ElementCounter::new(ElementCountResultType::InputCount))
            .add(TestOnlyOrderRecorder::new("initial"))

            .add(TestOnlyElementBufferingDuplicatingEditingProcessor::default())

            .add(ElementPrinter::with_prefix("final".to_string()).with_node_ids((1..=200).collect()))
            .add(TestOnlyOrderRecorder::new("final"))
            .add(ElementCounter::new(ElementCountResultType::OutputCount))
            ;

        processor_chain.process(simple_node_element(1, vec![("who", "kasper")]), &mut result);
        processor_chain.process(simple_node_element(2, vec![("who", "seppl")]), &mut result);
        processor_chain.process(simple_node_element(6, vec![("who", "hotzenplotz")]), &mut result);
        processor_chain.process(simple_node_element(8, vec![("who", "großmutter")]), &mut result);
        processor_chain.process(simple_way_element(23, vec![1, 2, 8, 6], vec![("who", "kasper")]), &mut result);
        processor_chain.process(simple_relation_element(66, vec![(MemberType::Way, 23, "kasper&seppl brign großmutter to hotzenplotz")], vec![("who", "großmutter")]), &mut result);
        processor_chain.flush_handlers(&mut result);
        println!("result nach flush: \n{}", result.format_multi_line());
        processor_chain.collect_result(&mut result);

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
        let mut result = HandlerResult::default();
        let mut processor_chain = HandlerChain::default()
            .add(ElementCounter::new(ElementCountResultType::InputCount))
            .add(TestOnlyOrderRecorder::new("initial"))

            .add(TestOnlyElementAdder::default())

            .add(ElementPrinter::with_prefix("final".to_string()).with_node_ids((1..=200).collect()))
            .add(TestOnlyOrderRecorder::new("final"))
            .add(ElementCounter::new(ElementCountResultType::OutputCount))
            ;

        processor_chain.process(simple_way_element(23, vec![1, 2, 8, 6], vec![("who", "kasper")]), &mut result);
        processor_chain.process(simple_node_element(1, vec![("who", "kasper")]), &mut result);
        processor_chain.process(simple_node_element(2, vec![("who", "seppl")]), &mut result);
        processor_chain.process(simple_node_element(6, vec![("who", "hotzenplotz")]), &mut result);
        processor_chain.process(simple_node_element(8, vec![("who", "großmutter")]), &mut result);
        processor_chain.flush_handlers(&mut result);
        processor_chain.collect_result(&mut result);

        assert_element_counts(&result, 4, 8,
                              0, 0,
                              1, 1);
        assert_eq!(&result.other.get("TestOnlyOrderRecorder initial").unwrap().clone(), "way#23, node#1, node#2, node#6, node#8");
        assert_eq!(&result.other.get("TestOnlyOrderRecorder final").unwrap().clone(), "way#23, node#1, node#101, node#2, node#102, node#6, node#106, node#8, node#108");
    }

    #[test]
    /// Assert that it is possible to run the chain and let processors permanently filter (remove) elements.
    /// The test uses TestOnlyElementFilter for this, which filters nodes with an even id.
    fn test_chain_with_element_filter() {
        let _ = SimpleLogger::new().init();
        let mut result = HandlerResult::default();
        let mut processor_chain = HandlerChain::default()
            .add(ElementCounter::new(ElementCountResultType::InputCount))
            .add(TestOnlyOrderRecorder::new("initial"))

            .add(TestOnlyElementFilter::default())

            .add(ElementPrinter::with_prefix("final".to_string()).with_node_ids((1..=200).collect()))
            .add(TestOnlyOrderRecorder::new("final"))
            .add(ElementCounter::new(ElementCountResultType::OutputCount))
            ;

        processor_chain.process(simple_node_element(1, vec![("who", "kasper")]), &mut result);
        processor_chain.process(simple_node_element(2, vec![("who", "seppl")]), &mut result);
        processor_chain.process(simple_node_element(6, vec![("who", "hotzenplotz")]), &mut result);
        processor_chain.process(simple_node_element(8, vec![("who", "großmutter")]), &mut result);
        processor_chain.flush_handlers(&mut result);
        processor_chain.collect_result(&mut result);

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
        let mut result = HandlerResult::default();
        let mut processor_chain = HandlerChain::default()
            .add(ElementCounter::new(ElementCountResultType::InputCount))
            .add(TestOnlyOrderRecorder::new("initial"))

            .add(TestOnlyElementReplacer::default())

            .add(ElementPrinter::with_prefix("final".to_string()).with_node_ids((1..=200).collect()))
            .add(TestOnlyOrderRecorder::new("final"))
            .add(ElementCounter::new(ElementCountResultType::OutputCount))
            ;

        processor_chain.process(simple_node_element(1, vec![("who", "kasper")]), &mut result);
        processor_chain.process(simple_node_element(2, vec![("who", "seppl")]), &mut result);
        processor_chain.process(simple_node_element(6, vec![("who", "hotzenplotz")]), &mut result);
        processor_chain.process(simple_node_element(8, vec![("who", "großmutter")]), &mut result);
        processor_chain.flush_handlers(&mut result);
        processor_chain.collect_result(&mut result);

        assert_element_counts(&result,
                              4, 4,
                              0, 0,
                              0, 0);
        assert_eq!(&result.other.get("TestOnlyOrderRecorder initial").unwrap().clone(), "node#1, node#2, node#6, node#8");
        assert_eq!(&result.other.get("TestOnlyOrderRecorder final").unwrap().clone(), "node#1, node#2, node#66, node#8");
    }

    fn assert_element_counts(result: & HandlerResult, input_node_count: u64, output_node_count: u64, input_relation_count: u64, output_relation_count: u64, input_way_count: u64, output_way_count: u64) {
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
        let mut result = HandlerResult::default();
        let mut processor_chain = HandlerChain::default()
            .add(ElementCounter::new(ElementCountResultType::InputCount))
            .add(TestOnlyOrderRecorder::new("initial"))

            .add(TestOnlyElementModifier::default())
            .add(TagKeyBasedOsmElementsFilter::new(OsmElementTypeSelection::node_only(), vec!["added".to_string()], FilterType::AcceptMatching))

            .add(ElementPrinter::with_prefix("final".to_string()).with_node_ids((1..=200).collect()))
            .add(TestOnlyOrderRecorder::new("final"))
            .add(ElementCounter::new(ElementCountResultType::OutputCount))
            ;

        processor_chain.process(simple_node_element(1, vec![("who", "kasper")]), &mut result);
        processor_chain.process(simple_node_element(2, vec![("who", "seppl")]), &mut result);
        processor_chain.process(simple_node_element(6, vec![("who", "hotzenplotz")]), &mut result);
        processor_chain.process(simple_node_element(8, vec![("who", "großmutter")]), &mut result);
        processor_chain.flush_handlers(&mut result);
        processor_chain.collect_result(&mut result);

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
        let mut result = HandlerResult::default();
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

        handle_test_nodes_and_verify_result(chain, &mut result);
    }

    #[test]
    fn handler_chain_with_node_id_filter() {
        let _ = SimpleLogger::new().init();
        let mut result = HandlerResult::default();
        result.node_ids.set(1usize, true);
        result.node_ids.set(2usize, true);
        let chain = HandlerChain::default()
            .add(ElementCounter::new(ElementCountResultType::InputCount))
            .add(NodeIdFilter {})
            .add(ElementCounter::new(ElementCountResultType::OutputCount));

        handle_test_nodes_and_verify_result(chain, &mut result);
    }


    #[test]
    fn handler_chains_with_node_id_collector_and_filter() {
        let _ = SimpleLogger::new().init();

        let mut result = HandlerResult::default();
        let mut chain1 = HandlerChain::default()
            .add(ReferencedNodeIdCollector::default());

        let chain2 = HandlerChain::default()
            .add(ElementCounter::new(ElementCountResultType::InputCount))
            .add(NodeIdFilter {})
            .add(ElementCounter::new(ElementCountResultType::OutputCount));

        chain1.process(simple_way_element(23, vec![1, 2], vec![("who", "kasper")]), &mut result);

        handle_test_nodes_and_verify_result(chain2, &mut result);
    }

    fn handle_test_nodes_and_verify_result(mut handler_chain: HandlerChain, result: &mut HandlerResult) {
        handler_chain.process(simple_node_element(1, vec![(existing_tag().as_str(), "kasper")]), result);
        handler_chain.process(simple_node_element(2, vec![(existing_tag().as_str(), "seppl")]), result);
        handler_chain.process(simple_node_element(3, vec![(existing_tag().as_str(), "hotzenplotz")]), result);
        handler_chain.process(simple_node_element(4, vec![(existing_tag().as_str(), "großmutter")]), result);

        handler_chain.flush_handlers(result);
        handler_chain.collect_result(result);
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
        let mut result = HandlerResult::default();
        let mut node_ids = BitVec::from_elem(10usize, false);
        node_ids.set(1usize, true);
        node_ids.set(2usize, true);

        let mut handler_chain = HandlerChain::default()
            .add(ElementCounter::new(ElementCountResultType::InputCount))
            .add(TestOnlyOrderRecorder::new("initial"))
            .add(BufferingElevationEnricher::new(
                GeoTiffManager::with_file_pattern("test/region*.tif"),
                5,
                6,
                BitVec::from_elem(10usize, false),
                true,
                0.01,
                0.01,
                1.0))
            .add(TestOnlyOrderRecorder::new("final"))
            .add(ElementCounter::new(ElementCountResultType::OutputCount))
            .add(ElementEvaluator::new("elevation",
                                       Box::new(|node| node.tags().iter().any(|tag| tag.k() == "ele").to_string()),
                                       Box::new(|_| "".to_string()),
                                       Box::new(|_| "".to_string())))
            .add(ElementEvaluator::new("way_refs",
                                       Box::new(|_| "".to_string()),
                                       Box::new(|way| way.refs().iter().map(|id| id.to_string()).collect::<Vec<String>>().join(",")),
                                       Box::new(|_| "".to_string())));

        handler_chain.process(as_node_element(node_without_ele_from_location(101, location_with_elevation_hd_philosophers_way_start(), vec![])), &mut result);
        handler_chain.process(as_node_element(node_without_ele_from_location(102, location_with_elevation_hd_philosophers_way_end(), vec![])), &mut result);
        handler_chain.process(as_way_element(simple_way(201, vec![101, 102], vec![])), &mut result);
        handler_chain.flush_handlers(&mut result);
        handler_chain.collect_result(&mut result);

        // dbg!(&result); // This causes the test to run eternally...?!

        assert_element_counts(&result, 2, 3,
                              0, 0,
                              1, 1);
        assert_eq!(&result.other.get("TestOnlyOrderRecorder initial").unwrap().clone(), "node#101, node#102, way#201");
        assert_eq!(&result.other.get("TestOnlyOrderRecorder final").unwrap().clone(), format!("node#101, node#102, node#{}, way#201", HIGHEST_NODE_ID+1).as_str());
        assert_eq!(&result.other.get("ElementEvaluator#elevation node results").unwrap().clone(), format!("101:true, 102:true, {}:true", HIGHEST_NODE_ID+1).as_str() );
        assert_eq!(&result.other.get("ElementEvaluator#way_refs:way#201").unwrap().clone(), format!("101,{},102",HIGHEST_NODE_ID+1).as_str());
    }

    fn add_to_result(result: &mut HandlerResult, elements: Vec<(MemberType, i64)>) {
        for (member_type, id) in elements {
            match member_type {
                MemberType::Node => result.nodes.push(simple_node(id, vec![])),
                MemberType::Way => result.ways.push(simple_way(id, vec![], vec![])),
                MemberType::Relation => result.relations.push(simple_relation(id, vec![], vec![])),
            }
        }
    }
    fn process_elements_one_by_one(handler_chain: &mut HandlerChain, result: &mut HandlerResult, elements: Vec<(MemberType, i64)>) {
        for (member_type, id) in elements {
            match member_type {
                MemberType::Node => handler_chain.process(simple_node_element(id, vec![]), result),
                MemberType::Way => handler_chain.process(simple_way_element(id, vec![], vec![]), result),
                MemberType::Relation => handler_chain.process(simple_relation_element(id, vec![], vec![]), result),
            }
        }
        handler_chain.flush_handlers(result);
        handler_chain.collect_result(result);
    }
    fn flush_elements_one_by_one(handler_chain: &mut HandlerChain, result: &mut HandlerResult, elements: Vec<(MemberType, i64)>) {
        for (member_type, id) in elements {
            match member_type {
                MemberType::Node => {
                    result.nodes.push(simple_node(id, vec![]));
                    handler_chain.flush_handlers(result); },
                MemberType::Way => {
                    result.ways.push(simple_way(id, vec![], vec![]));
                    handler_chain.flush_handlers(result);
                },
                MemberType::Relation => {
                    result.relations.push(simple_relation(id, vec![], vec![]));
                    handler_chain.flush_handlers(result);
                },
            }
        }

        handler_chain.collect_result(result);
    }

    fn flush_elements_in_one_call(handler_chain: &mut HandlerChain, result: &mut HandlerResult, elements: Vec<(MemberType, i64)>) {
        add_to_result(result, elements);
        handler_chain.flush_handlers(result);
        handler_chain.collect_result(result);
    }

    #[test]
    fn test_handler_chain_with_all_elements_filter_node_only() {
        let _ = SimpleLogger::new().init();
        let handle_type = OsmElementTypeSelection::node_only();

        let mut result = HandlerResult::default();
        let mut handler_chain = HandlerChain::default()
            .add(TestOnlyOrderRecorder::new("initial"))
            .add(AllElementsFilter{handle_types: handle_type})
            .add(TestOnlyOrderRecorder::new("final"));

        process_elements_one_by_one(&mut handler_chain, &mut result, vec![
            (MemberType::Node, 1), (MemberType::Way, 11), (MemberType::Relation, 21),
            (MemberType::Node, 2), (MemberType::Way, 12), (MemberType::Relation, 22),
            (MemberType::Node, 3), (MemberType::Way, 13), (MemberType::Relation, 23),
        ]);

        assert_eq!("node#1, way#11, relation#21, node#2, way#12, relation#22, node#3, way#13, relation#23", &result.other.get("TestOnlyOrderRecorder initial").unwrap().clone());
        assert_eq!("way#11, relation#21, way#12, relation#22, way#13, relation#23", &result.other.get("TestOnlyOrderRecorder final").unwrap().clone());
    }
    #[test]
    fn test_handler_chain_with_all_elements_filter_way_only() {
        let _ = SimpleLogger::new().init();
        let handle_type = OsmElementTypeSelection::way_only();

        let mut result = HandlerResult::default();
        let mut handler_chain = HandlerChain::default()
            .add(TestOnlyOrderRecorder::new("initial"))
            .add(AllElementsFilter{handle_types: handle_type})
            .add(TestOnlyOrderRecorder::new("final"));

        process_elements_one_by_one(&mut handler_chain, &mut result, vec![
            (MemberType::Node, 1), (MemberType::Way, 11), (MemberType::Relation, 21),
            (MemberType::Node, 2), (MemberType::Way, 12), (MemberType::Relation, 22),
            (MemberType::Node, 3), (MemberType::Way, 13), (MemberType::Relation, 23),
        ]);

        assert_eq!("node#1, way#11, relation#21, node#2, way#12, relation#22, node#3, way#13, relation#23", &result.other.get("TestOnlyOrderRecorder initial").unwrap().clone());
        assert_eq!("node#1, relation#21, node#2, relation#22, node#3, relation#23", &result.other.get("TestOnlyOrderRecorder final").unwrap().clone());
    }
    #[test]
    fn test_handler_chain_process_elements_with_all_elements_filter_all() {
        let _ = SimpleLogger::new().init();
        let handle_type = OsmElementTypeSelection::all();

        let mut result = HandlerResult::default();
        let mut handler_chain = HandlerChain::default()
            .add(TestOnlyOrderRecorder::new("initial"))
            .add(AllElementsFilter{handle_types: handle_type})
            .add(TestOnlyOrderRecorder::new("final"));

        process_elements_one_by_one(&mut handler_chain, &mut result, vec![
            (MemberType::Node, 1), (MemberType::Way, 11), (MemberType::Relation, 21),
            (MemberType::Node, 2), (MemberType::Way, 12), (MemberType::Relation, 22),
            (MemberType::Node, 3), (MemberType::Way, 13), (MemberType::Relation, 23),
        ]);

        assert_eq!("node#1, way#11, relation#21, node#2, way#12, relation#22, node#3, way#13, relation#23", &result.other.get("TestOnlyOrderRecorder initial").unwrap().clone());
        assert_eq!("", &result.other.get("TestOnlyOrderRecorder final").unwrap().clone());
    }
    #[test]
    fn test_handler_chain_flush_one_by_one_with_all_elements_filter_all() {
        let _ = SimpleLogger::new().init();
        let handle_type = OsmElementTypeSelection::all();

        let mut result = HandlerResult::default();
        let mut handler_chain = HandlerChain::default()
            .add(TestOnlyOrderRecorder::new("initial"))
            .add(AllElementsFilter{handle_types: handle_type})
            .add(TestOnlyOrderRecorder::new("final"));

        flush_elements_one_by_one(&mut handler_chain, &mut result, vec![
            (MemberType::Node, 1), (MemberType::Way, 11), (MemberType::Relation, 21),
            (MemberType::Node, 2), (MemberType::Way, 12), (MemberType::Relation, 22),
            (MemberType::Node, 3), (MemberType::Way, 13), (MemberType::Relation, 23),
        ]);

        assert_eq!("node#1, way#11, relation#21, node#2, way#12, relation#22, node#3, way#13, relation#23", &result.other.get("TestOnlyOrderRecorder initial").unwrap().clone());
        assert_eq!("", &result.other.get("TestOnlyOrderRecorder final").unwrap().clone());
    }
    #[test]
    fn test_handler_chain_flush_in_one_call_with_all_elements_filter_all() {
        let _ = SimpleLogger::new().init();
        let handle_type = OsmElementTypeSelection::all();

        let mut result = HandlerResult::default();
        let mut handler_chain = HandlerChain::default()
            .add(TestOnlyOrderRecorder::new("initial"))
            .add(AllElementsFilter{handle_types: handle_type})
            .add(TestOnlyOrderRecorder::new("final"));

        flush_elements_in_one_call(&mut handler_chain, &mut result, vec![
            (MemberType::Node, 1), (MemberType::Way, 11), (MemberType::Relation, 21),
            (MemberType::Node, 2), (MemberType::Way, 12), (MemberType::Relation, 22),
            (MemberType::Node, 3), (MemberType::Way, 13), (MemberType::Relation, 23),
        ]);

        assert_eq!("node#1, node#2, node#3, way#11, way#12, way#13, relation#21, relation#22, relation#23", &result.other.get("TestOnlyOrderRecorder initial").unwrap().clone());
        assert_eq!("", &result.other.get("TestOnlyOrderRecorder final").unwrap().clone());
    }

    #[test]
    fn test_handler_chain_process_with_complex_elements_filter() {
        let _ = SimpleLogger::new().init();

        let mut result = HandlerResult::default();
        let mut handler_chain = HandlerChain::default()
            .add(TestOnlyOrderRecorder::new("initial"))
            .add(ComplexElementsFilter::ors_default())
            .add(TestOnlyOrderRecorder::new("final"));

        //should be removed:
        handler_chain.process(simple_way_element(1, vec![], vec![("building", "x")]), &mut result);
        handler_chain.process(simple_relation_element(1, vec![], vec![("building", "x")]), &mut result);
        //should be accepted:
        handler_chain.process(simple_way_element(2, vec![], vec![("route", "xyz")]), &mut result);
        handler_chain.process(simple_relation_element(2, vec![], vec![("route", "xyz")]), &mut result);
        //nodes should be accepted and passed through:
        handler_chain.process(simple_node_element(1, vec![]), &mut result);

        handler_chain.flush_handlers(&mut result);
        handler_chain.collect_result(&mut result);

        assert_eq!("way#1, relation#1, way#2, relation#2, node#1", &result.other.get("TestOnlyOrderRecorder initial").unwrap().clone());
        assert_eq!("way#2, relation#2, node#1",                    &result.other.get("TestOnlyOrderRecorder final").unwrap().clone());
    }
    #[test]
    fn test_handler_chain_flush_in_one_call_with_complex_elements_filter() {
        let _ = SimpleLogger::new().init();

        let mut result = HandlerResult::default();
        let mut handler_chain = HandlerChain::default()
            .add(TestOnlyOrderRecorder::new("initial"))
            .add(ComplexElementsFilter::ors_default())
            .add(TestOnlyOrderRecorder::new("final"));

        //should be removed:
        result.ways.push(simple_way(1, vec![], vec![("building", "x")]));
        result.relations.push(simple_relation(1, vec![], vec![("building", "x")]));
        //should be accepted:
        result.ways.push(simple_way(2, vec![], vec![("route", "xyz")]));
        result.relations.push(simple_relation(2, vec![], vec![("route", "xyz")]));
        //nodes should be accepted and passed through:
        result.nodes.push(simple_node(1, vec![]));

        handler_chain.flush_handlers(&mut result);
        handler_chain.collect_result(&mut result);

        assert_eq!("node#1, way#1, way#2, relation#1, relation#2", &result.other.get("TestOnlyOrderRecorder initial").unwrap().clone());
        assert_eq!("node#1, way#2, relation#2",                    &result.other.get("TestOnlyOrderRecorder final").unwrap().clone());
    }

}
