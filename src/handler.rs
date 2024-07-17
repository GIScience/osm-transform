pub mod geotiff;

use std::collections::{HashMap, HashSet};
use std::ops::Deref;
use bit_vec::BitVec;
use osm_io::osm::model::node::Node;
use osm_io::osm::model::relation::{Member, Relation};
use osm_io::osm::model::tag::Tag;
use osm_io::osm::model::way::Way;
use regex::Regex;
use serde::de::Unexpected::Str;

const HIGHEST_NODE_ID: i64 = 50_000_000_000;//todo make configurable

#[derive(Debug)]
pub struct HandlerResult {//todo add HashMap to add results with configurable keys
    pub count_all_nodes: i32,
    pub count_accepted_nodes: i32,
    pub node_ids: BitVec,
}
impl HandlerResult {
    fn default() -> Self {
        Self::with_capacity(HIGHEST_NODE_ID as usize)
    }
    fn with_capacity(nbits: usize) -> Self {
        HandlerResult {
            count_accepted_nodes: 0,
            count_all_nodes: 0,
            node_ids: BitVec::from_elem(nbits, false)
        }
    }
    pub fn to_string(&mut self) -> String {
        format!("HandlerResult: count_all_nodes={} count_accepted_nodes={}", &self.count_all_nodes, &self.count_accepted_nodes)
    }
}
pub trait Handler {

    fn handle_node(&mut self, node: Node) -> Option<Node> {
        Some(node)
    }

    fn handle_way(&mut self, way: Way) -> Option<Way> {
        Some(way)
    }

    fn handle_relation(&mut self, relation: Relation) -> Option<Relation> {
        Some(relation)
    }

    fn add_result(&mut self, result: HandlerResult) -> HandlerResult {
        result
    }
}

#[derive(Default)]
pub(crate) struct HandlerChain {
    pub handlers: Vec<Box<dyn Handler>>
}
impl HandlerChain {
    pub(crate) fn process_node(&mut self, mut node: Node) {
        for processor in &mut self.handlers {
            let optional_node = processor.handle_node(node);
            match optional_node {
                None => { break }
                Some(result) => { node = result }
            }
        }
    }
    pub(crate) fn process_way(&mut self, mut way: Way) {
        for processor in &mut self.handlers {
            let optional_way = processor.handle_way(way);
            match optional_way {
                None => { break }
                Some(result) => { way = result }
            }
        }
    }
    pub(crate) fn process_relation(&mut self, mut relation: Relation) {
        for processor in &mut self.handlers {
            let optional_relation = processor.handle_relation(relation);
            match optional_relation {
                None => { break }
                Some(result) => { relation = result }
            }
        }
    }
    pub(crate) fn collect_result(&mut self) -> HandlerResult {
        let mut result = HandlerResult::default();
        for processor in &mut self.handlers {
            result = processor.add_result(result);
        }
        result
    }
    pub(crate) fn add(mut self, handler: impl Handler + Sized + 'static) -> HandlerChain {
        self.handlers.push(Box::new(handler));
        self
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
    pub handle_types: OsmElementTypeSelection,
    pub count_type: CountType,
}
impl ElementCounter {
    pub fn new(handle_types: OsmElementTypeSelection, count_type: CountType) -> Self {
        Self {
            nodes_count: 0,
            ways_count: 0,
            relations_count: 0,
            handle_types,
            count_type,
        }
    }
}
impl Handler for ElementCounter {
    fn handle_node(&mut self, node: Node) -> Option<Node> {
        if self.handle_types.node {
            self.nodes_count += 1;
        }
        Some(node)
    }
    fn handle_way(&mut self, way: Way) -> Option<Way> {
        if self.handle_types.way {
            self.ways_count += 1;
        }
        Some(way)
    }
    fn handle_relation(&mut self, relation: Relation) -> Option<Relation> {
        if self.handle_types.relation {
            self.relations_count += 1;
        }
        Some(relation)
    }

    fn add_result(&mut self, mut result: HandlerResult) -> HandlerResult {
        match self.count_type {
            CountType::ALL => { result.count_all_nodes = self.nodes_count }
            CountType::ACCEPTED => { result.count_accepted_nodes = self.nodes_count }
        }
        result
    }
}

#[derive(Debug)]
pub(crate) enum FilterType {
    AcceptMatching,
    RemoveMatching,
}

struct TagValueBasedOsmElementsFilter {
    pub handle_types: OsmElementTypeSelection,
    pub tag_key: String,
    pub tag_value_regex: Regex,
    pub filter_type: FilterType,
}
impl TagValueBasedOsmElementsFilter {
    fn new(handle_types: OsmElementTypeSelection, tag_key: String, tag_value_regex: Regex, filter_type: FilterType) -> Self {
        Self {
            handle_types,
            tag_key,
            tag_value_regex,
            filter_type,
        }
    }

    fn accept_by_tags(&mut self, tags: &Vec<Tag>) -> bool {
        let mut accept = false;
        match self.filter_type {
            FilterType::AcceptMatching => {
                accept = false;
                for tag in tags {
                    if self.tag_key.eq(tag.k()) && self.tag_value_regex.is_match(tag.v()) {
                        accept = true;
                        break
                    }
                }
            }
            FilterType::RemoveMatching => {
                for tag in tags {
                    accept = true;
                    if self.tag_key.eq(tag.k()) && self.tag_value_regex.is_match(tag.v()) {
                        accept = false;
                        break;
                    }
                }
            }
        }
        accept
    }
}
impl Handler for TagValueBasedOsmElementsFilter {
    fn handle_node(&mut self, node: Node) -> Option<Node> {
        if !self.handle_types.node  {
            return Some(node)
        }
        match self.accept_by_tags(&node.tags()) {
            true => {Some(node)}
            false => {None}
        }
    }
    fn handle_way(&mut self, way: Way) -> Option<Way> {
        if !self.handle_types.way  {
            return Some(way)
        }
        match self.accept_by_tags(&way.tags()) {
            true => {Some(way)}
            false => {None}
        }
    }
    fn handle_relation(&mut self, relation: Relation) -> Option<Relation> {
        if !self.handle_types.relation  {
            return Some(relation)
        }
        match self.accept_by_tags(&relation.tags()) {
            true => {Some(relation)}
            false => {None}
        }
    }
}






struct TagKeyBasedOsmElementsFilter {
    pub handle_types: OsmElementTypeSelection,
    pub tag_keys: Vec<String>,
    pub filter_type: FilterType,
}
impl TagKeyBasedOsmElementsFilter {
    fn new(handle_types: OsmElementTypeSelection, tag_keys: Vec<String>, filter_type: FilterType) -> Self {
        Self {
            handle_types,
            tag_keys,
            filter_type,
        }
    }
    fn accept_by_tags(&mut self, tags: &Vec<Tag>) -> bool {
        let contains_any_key = tags.iter().any(|tag| self.tag_keys.contains(tag.k()));
        match self.filter_type {
            FilterType::AcceptMatching => {
                return contains_any_key
            }
            FilterType::RemoveMatching => {
                return !contains_any_key
            }
        }
    }
}
impl Handler for TagKeyBasedOsmElementsFilter {
    fn handle_node(&mut self, node: Node) -> Option<Node> {
        if ! self.handle_types.node {
            return Some(node)
        }
        match self.accept_by_tags(&node.tags()) {
            true => {Some(node)}
            false => {None}
        }
    }
    fn handle_way(&mut self, way: Way) -> Option<Way> {
        if !self.handle_types.way  {
            return Some(way)
        }
        match self.accept_by_tags(&way.tags()) {
            true => {Some(way)}
            false => {None}
        }
    }
    fn handle_relation(&mut self, relation: Relation) -> Option<Relation> {
        if !self.handle_types.relation  {
            return Some(relation)
        }
        match self.accept_by_tags(&relation.tags()) {
            true => {Some(relation)}
            false => {None}
        }
    }

}


pub(crate) struct AllElementsFilter {
    pub handle_types: OsmElementTypeSelection,
}
impl Handler for AllElementsFilter {
    fn handle_node(&mut self, node: Node) -> Option<Node> {
        match self.handle_types.node {
            true => {None}
            false => {Some(node)}
        }
    }

    fn handle_way(&mut self, way: Way) -> Option<Way> {
        match self.handle_types.way {
            true => {None}
            false => {Some(way)}
        }
    }

    fn handle_relation(&mut self, relation: Relation) -> Option<Relation> {
        match self.handle_types.relation {
            true => {None}
            false => {Some(relation)}
        }
    }
}

pub(crate) struct NodeIdFilter {
    pub(crate) node_ids: BitVec
}

impl NodeIdFilter {
    fn default() -> Self {
        Self::with_capacity(HIGHEST_NODE_ID as usize)
    }
    fn with_capacity(nbits: usize) -> Self {
        NodeIdFilter {
            node_ids: BitVec::from_elem(nbits, false)
        }
    }
}

impl Handler for NodeIdFilter {
    fn handle_node(&mut self, node: Node) -> Option<Node> {
        match self.node_ids.get(node.id().clone() as usize).unwrap_or(false) {
            true => {
                log::trace!("node {} found in bitmap", &node.id().clone());
                Some(node)
            }
            false => {
                log::trace!("node {} is not in bitmap - filtering", &node.id().clone());
                None
            }
        }
    }
}

pub(crate) struct ReferencedNodeIdCollector {//todo this is an initial implementation to complete the handler chain. replace with efficient implementation, e.g. bitmap based
    referenced_node_ids: BitVec
}

impl ReferencedNodeIdCollector {
    pub(crate) fn default() -> Self {
        Self::with_capacity(HIGHEST_NODE_ID as usize)
    }
    fn with_capacity(nbits: usize) -> Self {
        ReferencedNodeIdCollector {
            referenced_node_ids: BitVec::from_elem(nbits, false)
        }
    }
}

impl Handler for ReferencedNodeIdCollector {
    fn handle_way(&mut self, way: Way) -> Option<Way> {
        log::trace!("xxxxxxxxxxxxxxxxx way");
        for id in way.refs() {
            let idc = id.clone();
            self.referenced_node_ids.set(idc as usize, true);
        }
        Some(way)
    }

    fn handle_relation(&mut self, relation: Relation) -> Option<Relation> {
        log::trace!("xxxxxxxxxxxxxxxxx relation");
        for member in relation.members() {
            match member {
                Member::Node { member } => {
                    log::trace!("relation {} references node {} - set true in bitmap", &relation.id(), &member.id());
                    self.referenced_node_ids.set(member.id().clone() as usize, true);
                }
                Member::Way { .. } => {}
                Member::Relation { .. } => {}
            }

        }
        Some(relation)
    }

    fn add_result(&mut self, mut result: HandlerResult) -> HandlerResult {
        log::debug!("cloning node_ids of ReferencedNodeIdCollector with len={} into HandlerResult ", self.referenced_node_ids.len());
        result.node_ids = self.referenced_node_ids.clone();
        result
    }
}

pub(crate) struct ComplexElementsFilter {
    pub has_good_key_predicate: HasOneOfTagKeysPredicate,
    pub has_good_key_value_predicate: HasTagKeyValuePredicate,
    pub has_bad_key_predicate: HasNoneOfTagKeysPredicate,
}
impl ComplexElementsFilter {
    pub(crate) fn new(
                      has_good_key_predicate: HasOneOfTagKeysPredicate,
                      has_good_key_value_predicate: HasTagKeyValuePredicate,
                      has_bad_key_predicate: HasNoneOfTagKeysPredicate) -> Self {
        Self {
            has_good_key_predicate,
            has_good_key_value_predicate,
            has_bad_key_predicate,
        }
    }

    pub(crate) fn ors_default() -> Self{
        let mut key_values = HashMap::new();
        key_values.insert("railway".to_string(), "platform".to_string());
        key_values.insert("public_transport".to_string(), "platform".to_string());
        key_values.insert("man_made".to_string(), "pier".to_string());

        ComplexElementsFilter::new(
            HasOneOfTagKeysPredicate { keys: vec!["highway".to_string(), "route".to_string()] },
            HasTagKeyValuePredicate { key_values: key_values },
            HasNoneOfTagKeysPredicate {
                keys: vec![
                    "building".to_string(),
                    "landuse".to_string(),
                    "boundary".to_string(),
                    "natural".to_string(),
                    "place".to_string(),
                    "waterway".to_string(),
                    "aeroway".to_string(),
                    "aviation".to_string(),
                    "military".to_string(),
                    "power".to_string(),
                    "communication".to_string(),
                    "man_made".to_string()]
            })
    }

    fn accept_by_tags(&mut self, tags: &Vec<Tag>) -> bool {
         self.has_good_key_predicate.test(tags) ||
            self.has_good_key_value_predicate.test(tags) ||
            self.has_bad_key_predicate.test(tags)
    }
}
impl Handler for ComplexElementsFilter {
    fn handle_way(&mut self, way: Way) -> Option<Way> {
        match self.accept_by_tags(&way.tags()) {
            true => {
                log::trace!("accepting way {}", way.id());
                Some(way)
            }
            false => {
                log::trace!("removing way {}", way.id());
                None
            }
        }
    }
    fn handle_relation(&mut self, relation: Relation) -> Option<Relation> {
        match self.accept_by_tags(&relation.tags()) {
            true => {
                log::trace!("accepting relation {}", relation.id());
                Some(relation)
            }
            false => {
                log::trace!("removing relation {}", relation.id());
                None
            }
        }
    }
}


struct HasOneOfTagKeysPredicate {
    pub keys: Vec<String>
}
impl HasOneOfTagKeysPredicate {
    fn test(&mut self, tags: &Vec<Tag>) -> bool {
        tags.iter().any(|tag| self.keys.contains(tag.k()))
    }
}


struct HasTagKeyValuePredicate {
    pub key_values: HashMap<String,String>
}
impl HasTagKeyValuePredicate {
    fn test(&mut self, tags: &Vec<Tag>) -> bool {
        for tag in tags {
            if let Some(match_value) = self.key_values.get(tag.k()) {
                if tag.v() == match_value {
                    return true;
                }
            }
        }
        false
    }
}


struct HasNoneOfTagKeysPredicate {
    pub keys: Vec<String>
}
impl HasNoneOfTagKeysPredicate {
    fn test(&mut self, tags: &Vec<Tag>) -> bool {
        tags.iter().all(|tag| !self.keys.contains(tag.k()))
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

pub(crate) struct TagFilterByKey {
    pub handle_types: OsmElementTypeSelection,
    pub key_regex: Regex,
    pub filter_type: FilterType,
}
impl TagFilterByKey {
    pub(crate) fn new(handle_types: OsmElementTypeSelection, key_regex: Regex, filter_type: FilterType) -> Self {
        Self {
            handle_types,
            key_regex,
            filter_type,
        }
    }

    fn filter_tags(&mut self, tags: &mut Vec<Tag>) {
        match self.filter_type {
            FilterType::AcceptMatching => {
                tags.retain(|tag| self.key_regex.is_match(&tag.k()));
            }
            FilterType::RemoveMatching => {
                tags.retain(|tag| !self.key_regex.is_match(&tag.k()));
            }
        }
    }
}
impl Handler for TagFilterByKey {
    fn handle_node(&mut self, mut node: Node) -> Option<Node> {
        if self.handle_types.node  {
            self.filter_tags(&mut node.tags_mut());
        }
        Some(node)
    }
    fn handle_relation(&mut self, mut relation: Relation) -> Option<Relation> {
        if self.handle_types.relation  {
            self.filter_tags(&mut relation.tags_mut());
        }
        Some(relation)
    }
    fn handle_way(&mut self, mut way: Way) -> Option<Way> {
        if self.handle_types.way  {
            self.filter_tags(&mut way.tags_mut());
        }
        Some(way)
    }
}






pub(crate) struct ElementPrinter {
    pub prefix: String,
    pub node_ids: HashSet<i64>,
    pub way_ids: HashSet<i64>,
    pub relation_ids: HashSet<i64>,
    pub handle_types: OsmElementTypeSelection
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
}
impl Handler for ElementPrinter {
    fn handle_node(&mut self, node: Node) -> Option<Node> {
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
        Some(node)
    }
    fn handle_way(&mut self, way: Way) -> Option<Way> {
        if self.handle_types.way &&  self.way_ids.contains(&way.id()) {
            println!("{}way {} visible: {}", &self.prefix ,&way.id(), &way.visible());
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
        Some(way)
    }
    fn handle_relation(&mut self, relation: Relation) -> Option<Relation> {
        if self.handle_types.relation &&  self.relation_ids.contains(&relation.id()) {
            println!("{}relation {} visible: {}", &self.prefix ,&relation.id(), &relation.visible());
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
        Some(relation)
    }
}

#[derive(Default)]
pub(crate) struct MetadataRemover;

impl Handler for MetadataRemover {
    fn handle_node(&mut self, node: Node) -> Option<Node> {
        Some(Node::new(node.id(), 0, node.coordinate().clone(), 0, 0, 0, String::default(), node.visible(), node.tags().clone()))
    }

    fn handle_way(&mut self, way: Way) -> Option<Way> {
        Some(Way::new(way.id(), 0, 0, 0, 0, String::default(), way.visible(), way.refs().clone(), way.tags().clone()))
    }

    fn handle_relation(&mut self, relation: Relation) -> Option<Relation> {
        Some(Relation::new(relation.id(), 0, 0, 0, 0, String::default(), relation.visible(), relation.members().clone(), relation.tags().clone()))
    }
}


#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use bit_vec::BitVec;
    use osm_io::osm::model::coordinate::Coordinate;
    use osm_io::osm::model::node::Node;
    use osm_io::osm::model::relation::{Member, MemberData, Relation};
    use osm_io::osm::model::tag::Tag;
    use osm_io::osm::model::way::Way;
    use regex::Regex;
    use simple_logger::SimpleLogger;

    use crate::handler::{ComplexElementsFilter, CountType, ElementCounter, ElementPrinter, FilterType, Handler, HandlerChain, HandlerResult, HasNoneOfTagKeysPredicate, HasOneOfTagKeysPredicate, HasTagKeyValuePredicate, HIGHEST_NODE_ID, MetadataRemover, NodeIdFilter, OsmElementTypeSelection, TagFilterByKey, TagKeyBasedOsmElementsFilter, TagValueBasedOsmElementsFilter};

    const EXISTING_TAG: &str = "EXISTING_TAG";
    const MISSING_TAG: &str = "MISSING_TAG";

    fn existing_tag() -> String { "EXISTING_TAG".to_string() }

    fn missing_tag() -> String { "MISSING_TAG".to_string() }

    #[test]
    fn handler_chain() {
        SimpleLogger::new().init();
        let chain = HandlerChain::default()
            .add(ElementCounter::new(
                OsmElementTypeSelection::node_only(),
                CountType::ALL))
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
            .add(ElementCounter::new(
                OsmElementTypeSelection::node_only(),
                CountType::ACCEPTED))
            .add(TestOnlyNodeIdCollector::new(100));

        handle_test_nodes_and_verify_result(chain);
    }


    #[test]
    fn handler_chain_with_node_id_filter() {
        SimpleLogger::new().init();
        let mut node_ids = BitVec::from_elem(10usize, false);
        node_ids.set(1usize, true);
        node_ids.set(2usize, true);
        let chain = HandlerChain::default()
            .add(ElementCounter::new(
                OsmElementTypeSelection::node_only(),
                CountType::ALL))
            .add(NodeIdFilter { node_ids: node_ids.clone() })
            .add(ElementCounter::new(
                OsmElementTypeSelection::node_only(),
                CountType::ACCEPTED))
            .add(TestOnlyNodeIdCollector::new(100));

        handle_test_nodes_and_verify_result(chain);
    }

    fn handle_test_nodes_and_verify_result(mut handler_chain: HandlerChain) {
        handler_chain.process_node(Node::new(1, 1, Coordinate::new(1.0f64, 1.1f64), 1, 1, 1, "a".to_string(), true, vec![Tag::new(existing_tag(), "kasper".to_string())]));
        handler_chain.process_node(Node::new(2, 1, Coordinate::new(2.0f64, 1.2f64), 1, 1, 1, "a".to_string(), true, vec![Tag::new(existing_tag(), "seppl".to_string())]));
        handler_chain.process_node(Node::new(3, 1, Coordinate::new(3.0f64, 1.3f64), 1, 1, 1, "a".to_string(), true, vec![Tag::new(existing_tag(), "hotzenplotz".to_string())]));
        handler_chain.process_node(Node::new(4, 1, Coordinate::new(4.0f64, 1.4f64), 1, 1, 1, "a".to_string(), true, vec![Tag::new(existing_tag(), "großmutter".to_string())]));

        let mut result = &handler_chain.collect_result();

        assert_eq!(result.count_all_nodes, 4);
        assert_eq!(result.count_accepted_nodes, 2);
        assert_eq!(result.node_ids[0], false);
        assert_eq!(result.node_ids[1], true);
        assert_eq!(result.node_ids[2], true);
        assert_eq!(result.node_ids[3], false);
    }

    pub(crate) struct TestOnlyNodeIdCollector {
        pub node_ids: BitVec,
    }
    impl TestOnlyNodeIdCollector {
        pub fn new(nbits: usize) -> Self {
            TestOnlyNodeIdCollector {
                node_ids: BitVec::from_elem(nbits, false)
            }
        }
    }
    impl Handler for TestOnlyNodeIdCollector {
        fn handle_node(&mut self, node: Node) -> Option<Node> {
            self.node_ids.set(node.id() as usize, true);
            Some(node)
        }
        fn add_result(&mut self, mut result: HandlerResult) -> HandlerResult {
            result.node_ids = self.node_ids.clone();
            result
        }
    }

    #[test]
    fn test_tag_filter_by_key_with_remove_matching() {
        let mut tag_filter = TagFilterByKey::new(
            OsmElementTypeSelection::node_only(),
            Regex::new(".*bad.*").unwrap(),
            FilterType::RemoveMatching);

        let node_option = tag_filter.handle_node(Node::new(1, 1, Coordinate::new(1.0f64, 1.1f64), 1, 1, 1, "a".to_string(), true,
                                                               vec![
                                                                   Tag::new("bad".to_string(), "hotzenplotz".to_string()),
                                                                   Tag::new("good".to_string(), "kasper".to_string()),
                                                                   Tag::new("more-bad".to_string(), "vader".to_string()),
                                                                   Tag::new("more-good".to_string(), "grandma".to_string()),
                                                                   Tag::new("badest".to_string(), "voldemort".to_string()),
                                                               ]));
        match node_option {
            None => panic!("The element itself should not be filtered, only tags!"),
            Some(node) => {
                assert_eq!(node.tags().len(), 2);
                assert_eq!(node.tags()[0].k(), &"good");
                assert_eq!(node.tags()[0].v(), &"kasper");
                assert_eq!(node.tags()[1].k(), &"more-good");
                assert_eq!(node.tags()[1].v(), &"grandma");
            }
        }
    }

    #[test]
    fn test_tag_filter_by_key_with_remove_matching_complex_regex() {
        let mut tag_filter = TagFilterByKey::new(
            OsmElementTypeSelection::node_only(),
            Regex::new("(.*:)?source(:.*)?|(.*:)?note(:.*)?|url|created_by|fixme|wikipedia").unwrap(),
            FilterType::RemoveMatching);

        let node_option = tag_filter.handle_node(Node::new(1, 1, Coordinate::new(1.0f64, 1.1f64), 1, 1, 1, "a".to_string(), true,
                                                               vec![
                                                                   Tag::new("closed:source".to_string(), "bad".to_string()),
                                                                   Tag::new("source".to_string(), "bad".to_string()),
                                                                   Tag::new("source:x".to_string(), "bad".to_string()),
                                                                   Tag::new("x:source:y".to_string(), "bad".to_string()),
                                                                   Tag::new("opensource".to_string(), "bad".to_string()), //really?
                                                                   Tag::new("note".to_string(), "bad".to_string()),
                                                                   Tag::new("url".to_string(), "bad".to_string()),
                                                                   Tag::new("created_by".to_string(), "bad".to_string()),
                                                                   Tag::new("fixme".to_string(), "bad".to_string()),
                                                                   Tag::new("wikipedia".to_string(), "bad".to_string()),
                                                                   Tag::new("wikimedia".to_string(), "good".to_string()),
                                                               ]));
        match node_option {
            None => panic!("The element itself should not be filtered, only tags!"),
            Some(node) => {
                dbg!(&node);
                assert_eq!(node.tags().len(), 1);
                for tag in node.tags() {
                    assert_eq!(tag.v(), "good")
                }
            }
        }
    }

    #[test]
    fn test_tag_filter_by_key_with_keep_matching() {
        let mut tag_filter = TagFilterByKey::new(
            OsmElementTypeSelection::all(),
            Regex::new(".*good.*").unwrap(),
            FilterType::AcceptMatching);

        let node_option = tag_filter.handle_node(Node::new(1, 1, Coordinate::new(1.0f64, 1.1f64), 1, 1, 1, "a".to_string(), true,
                                                               vec![
                                                                   Tag::new("bad".to_string(), "hotzenplotz".to_string()),
                                                                   Tag::new("good".to_string(), "kasper".to_string()),
                                                                   Tag::new("more-bad".to_string(), "vader".to_string()),
                                                                   Tag::new("more-good".to_string(), "grandma".to_string()),
                                                                   Tag::new("badest".to_string(), "voldemort".to_string()),
                                                               ]));
        match node_option {
            None => panic!("The element itself should not be filtered, only tags!"),
            Some(node) => {
                dbg!(&node);
                assert_eq!(node.tags().len(), 2);
                assert_eq!(node.tags()[0].k(), &"good");
                assert_eq!(node.tags()[0].v(), &"kasper");
                assert_eq!(node.tags()[1].k(), &"more-good");
                assert_eq!(node.tags()[1].v(), &"grandma");
            }
        }
    }
    #[test]
    fn test_tag_filter_by_key_with_node_not_handled() {
        let mut tag_filter = TagFilterByKey::new(
            OsmElementTypeSelection::way_only(),
            Regex::new(".*").unwrap(),
            FilterType::RemoveMatching);

        let node_option = tag_filter.handle_node(Node::new(1, 1, Coordinate::new(1.0f64, 1.1f64), 1, 1, 1, "a".to_string(), true,
                                                               vec![
                                                                   Tag::new("a".to_string(), "1".to_string()),
                                                                   Tag::new("b".to_string(), "2".to_string()),
                                                                   Tag::new("c".to_string(), "3".to_string()),
                                                               ]));
        match node_option {
            None => panic!("The element itself should not be filtered, only tags!"),
            Some(node) => {
                assert_eq!(node.tags().len(), 3);
            }
        }
    }
    #[test]
    fn test_tag_filter_by_key_with_node_handled() {
        let mut tag_filter = TagFilterByKey::new(
            OsmElementTypeSelection::all(),
            Regex::new(".*").unwrap(),
            FilterType::RemoveMatching);

        let node_option = tag_filter.handle_node(Node::new(1, 1, Coordinate::new(1.0f64, 1.1f64), 1, 1, 1, "a".to_string(), true,
                                                               vec![
                                                                   Tag::new("a".to_string(), "1".to_string()),
                                                                   Tag::new("b".to_string(), "2".to_string()),
                                                                   Tag::new("c".to_string(), "3".to_string()),
                                                               ]));
        match node_option {
            None => panic!("The element itself should not be filtered, only tags!"),
            Some(node) => {
                dbg!(&node);
                assert_eq!(node.tags().len(), 0);
            }
        }
    }
    #[test]
    fn filter_elements_remove_by_keys() {
        let mut filter = TagKeyBasedOsmElementsFilter::new(
            OsmElementTypeSelection::all(),
            vec!["bad".to_string(), "ugly".to_string()],
            FilterType::RemoveMatching);
        assert!(filter.handle_node(Node::new(1, 1, Coordinate::new(1.0f64, 1.1f64), 1, 1, 1, "a".to_string(), true,
                                             vec![
                                                 Tag::new("good".to_string(), "1".to_string()),
                                                 Tag::new("bad".to_string(), "2".to_string()),
                                             ]))
            .is_none());
        assert!(filter.handle_node(Node::new(1, 1, Coordinate::new(1.0f64, 1.1f64), 1, 1, 1, "a".to_string(), true,
                                             vec![
                                                 Tag::new("good".to_string(), "1".to_string()),
                                                 Tag::new("nice".to_string(), "2".to_string()),
                                             ]))
            .is_some());
        assert!(filter.handle_node(Node::new(1, 1, Coordinate::new(1.0f64, 1.1f64), 1, 1, 1, "a".to_string(), true,
                                             vec![
                                                 Tag::new("ugly".to_string(), "1".to_string()),
                                                 Tag::new("bad".to_string(), "2".to_string()),
                                             ]))
            .is_none());
    }

    #[test]
    fn filter_elements_accept_by_keys() {
        let mut filter = TagKeyBasedOsmElementsFilter::new(
            OsmElementTypeSelection::all(),
            vec!["bad".to_string(), "ugly".to_string()],
            FilterType::AcceptMatching,
        );

        assert!(filter.handle_node(Node::new(1, 1, Coordinate::new(1.0f64, 1.1f64), 1, 1, 1, "a".to_string(), true,
                                             vec![
                                                 Tag::new("good".to_string(), "1".to_string()),
                                                 Tag::new("bad".to_string(), "2".to_string()),
                                             ]))
            .is_some());
        assert!(filter.handle_node(Node::new(1, 1, Coordinate::new(1.0f64, 1.1f64), 1, 1, 1, "a".to_string(), true,
                                             vec![
                                                 Tag::new("good".to_string(), "1".to_string()),
                                                 Tag::new("nice".to_string(), "2".to_string()),
                                             ]))
            .is_none());
        assert!(filter.handle_node(Node::new(1, 1, Coordinate::new(1.0f64, 1.1f64), 1, 1, 1, "a".to_string(), true,
                                             vec![
                                                 Tag::new("ugly".to_string(), "1".to_string()),
                                                 Tag::new("bad".to_string(), "2".to_string()),
                                             ]))
            .is_some());
    }

    #[test]
    fn has_one_of_tag_keys_predicate_with_only_matching_tags() {
        let mut predicate = HasOneOfTagKeysPredicate { keys: vec!["good".to_string(), "nice".to_string()] };
        assert_eq!(true, predicate.test(&vec![
            Tag::new("good".to_string(), "1".to_string()),
        ]));
    }
    #[test]
    fn has_one_of_tag_keys_predicate_with_only_all_matching_tags() {
        let mut predicate = HasOneOfTagKeysPredicate { keys: vec!["good".to_string(), "nice".to_string()] };
        assert_eq!(true, predicate.test(&vec![
            Tag::new("good".to_string(), "1".to_string()),
            Tag::new("nice".to_string(), "2".to_string()),
        ]));
    }
    #[test]
    fn has_one_of_tag_keys_predicate_with_also_matching_tags() {
        let mut predicate = HasOneOfTagKeysPredicate { keys: vec!["good".to_string(), "nice".to_string()] };
        assert_eq!(true, predicate.test(&vec![
            Tag::new("good".to_string(), "1".to_string()),
            Tag::new("bad".to_string(), "2".to_string()),
        ]));
    }
    #[test]
    fn has_one_of_tag_keys_predicate_with_no_matching_tags() {
        let mut predicate = HasOneOfTagKeysPredicate { keys: vec!["good".to_string(), "nice".to_string()] };
        assert_eq!(false, predicate.test(&vec![
            Tag::new("ugly".to_string(), "1".to_string()),
            Tag::new("bad".to_string(), "2".to_string()),
        ]));
    }

    #[test]
    fn has_tag_key_value_predicate_with_no_matching_tag() {
        let mut key_values = HashMap::new();
        key_values.insert("good".to_string(), "good".to_string());
        key_values.insert("nice".to_string(), "nice".to_string());
        let mut predicate = HasTagKeyValuePredicate { key_values: key_values };
        assert_eq!(false, predicate.test(&vec![
            Tag::new("bad".to_string(), "1".to_string()),
            Tag::new("ugly".to_string(), "1".to_string()),
        ]));
    }
    #[test]
    fn has_tag_key_value_predicate_with_only_tag_with_wrong_value() {
        let mut key_values = HashMap::new();
        key_values.insert("good".to_string(), "good".to_string());
        key_values.insert("nice".to_string(), "nice".to_string());
        let mut predicate = HasTagKeyValuePredicate { key_values: key_values };
        assert_eq!(false, predicate.test(&vec![
            Tag::new("good".to_string(), "1".to_string()),
        ]));
    }
    #[test]
    fn has_tag_key_value_predicate_with_also_tag_with_wrong_value() {
        let mut key_values = HashMap::new();
        key_values.insert("good".to_string(), "good".to_string());
        key_values.insert("nice".to_string(), "nice".to_string());
        let mut predicate = HasTagKeyValuePredicate { key_values: key_values };
        assert_eq!(true, predicate.test(&vec![
            Tag::new("bad".to_string(), "1".to_string()),
            Tag::new("good".to_string(), "1".to_string()),
            Tag::new("nice".to_string(), "nice".to_string()),
        ]));
    }
    #[test]
    fn has_tag_key_value_predicate_with_only_tag_with_matching_value() {
        let mut key_values = HashMap::new();
        key_values.insert("good".to_string(), "good".to_string());
        key_values.insert("nice".to_string(), "nice".to_string());
        let mut predicate = HasTagKeyValuePredicate { key_values: key_values };
        assert_eq!(true, predicate.test(&vec![
            Tag::new("good".to_string(), "good".to_string()),
        ]));
    }
    #[test]
    fn has_tag_key_value_predicate_with_also_tag_with_matching_value() {
        let mut key_values = HashMap::new();
        key_values.insert("good".to_string(), "good".to_string());
        key_values.insert("nice".to_string(), "nice".to_string());
        let mut predicate = HasTagKeyValuePredicate { key_values: key_values };
        assert_eq!(true, predicate.test(&vec![
            Tag::new("bad".to_string(), "1".to_string()),
            Tag::new("good".to_string(), "good".to_string()),
        ]));
    }

    #[test]
    fn has_none_of_tag_keys_predicate_with_only_non_matching_tag() {
        let mut predicate = HasNoneOfTagKeysPredicate { keys: vec!["bad".to_string(), "ugly".to_string()] };
        assert_eq!(true, predicate.test(&vec![
            Tag::new("good".to_string(), "1".to_string()),
        ]));
    }
    #[test]
    fn has_none_of_tag_keys_predicate_also_matching_tag() {
        let mut predicate = HasNoneOfTagKeysPredicate { keys: vec!["bad".to_string(), "ugly".to_string()] };
        assert_eq!(false, predicate.test(&vec![
            Tag::new("good".to_string(), "1".to_string()),
            Tag::new("bad".to_string(), "1".to_string()),
        ]));
    }
    #[test]
    fn has_none_of_tag_keys_predicate_only_matching_tags() {
        let mut predicate = HasNoneOfTagKeysPredicate { keys: vec!["bad".to_string(), "ugly".to_string()] };
        assert_eq!(false, predicate.test(&vec![
            Tag::new("ugly".to_string(), "1".to_string()),
            Tag::new("bad".to_string(), "1".to_string()),
        ]));
    }

    #[test]
    fn complex_filter_with_ors_default() {
        let mut filter = ComplexElementsFilter::ors_default();
        // has key to keep and key-value to keep, bad key 'building' should not take effect => should be accepted
        assert!(filter.handle_way(Way::new(1, 1, 1, 1, 1, "a".to_string(), true, vec![],
                                           vec![
                                                 Tag::new("route".to_string(), "xyz".to_string()),
                                                 Tag::new("railway".to_string(), "platform".to_string()),
                                                 Tag::new("building".to_string(), "x".to_string()),
                                             ])).is_some());

        // has key to keep, bad key 'building' should not take effect => should be accepted
        assert!(filter.handle_way(Way::new(2, 1, 1, 1, 1, "a".to_string(), true, vec![],
                                             vec![
                                                 Tag::new("route".to_string(), "xyz".to_string()),
                                                 Tag::new("building".to_string(), "x".to_string()),
                                             ])).is_some());

        // has key-value to keep, bad key 'building' should not take effect => should be accepted
        assert!(filter.handle_way(Way::new(3, 1, 1, 1, 1, "a".to_string(), true, vec![],
                                             vec![
                                                 Tag::new("railway".to_string(), "platform".to_string()),
                                                 Tag::new("building".to_string(), "x".to_string()),
                                             ])).is_some());

        // has no key or key-value to keep, but also no bad key => should be accepted
        assert!(filter.handle_way(Way::new(4, 1, 1, 1, 1, "a".to_string(), true, vec![],
                                             vec![
                                                 Tag::new("railway".to_string(), "wrong-value".to_string()),
                                                 Tag::new("something".to_string(), "else".to_string()),
                                             ])).is_some());

        // has no key or key-value to keep, some other key, but also one bad key => should be filtered
        assert!(filter.handle_way(Way::new(5, 1, 1, 1, 1, "a".to_string(), true, vec![],
                                             vec![
                                                 Tag::new("railway".to_string(), "wrong-value".to_string()),
                                                 Tag::new("something".to_string(), "else".to_string()),
                                                 Tag::new("building".to_string(), "x".to_string()),
                                             ])).is_none());

        // has only one bad key => should be filtered
        assert!(filter.handle_way(Way::new(6, 1, 1, 1, 1, "a".to_string(), true, vec![],
                                             vec![
                                                 Tag::new("building".to_string(), "x".to_string()),
                                             ])).is_none());

        // has only one other key => should be accepted
        assert!(filter.handle_way(Way::new(7, 1, 1, 1, 1, "a".to_string(), true, vec![],
                                             vec![
                                                 Tag::new("something".to_string(), "x".to_string()),
                                             ])).is_some());
    }
    #[test]
    fn element_printer(){
        let mut printer = ElementPrinter::default().print_node(2);

        // has only one bad key => should be filtered
        assert!(printer.handle_node(Node::new(1, 1, Coordinate::new(1.0f64, 1.1f64), 1, 1, 1, "a".to_string(), true,
                                              vec![
                                                 Tag::new("building".to_string(), "x".to_string()),
                                             ])).is_some());

        // has only one other key => should be accepted
        assert!(printer.handle_node(Node::new(2, 1, Coordinate::new(1.0f64, 1.1f64), 1, 1, 1, "a".to_string(), true,
                                              vec![
                                                 Tag::new("something".to_string(), "x".to_string()),
                                             ])).is_some());

    }

    #[test]
    fn node_id_collector(){
        let mut collector = TestOnlyNodeIdCollector::new(10);
        assert_eq!(10, collector.node_ids.len());
        collector.handle_node(Node::new(2, 1, Coordinate::new(1.0f64, 1.1f64), 1, 1, 1, "a".to_string(), true,vec![]));
        assert_eq!(false, collector.node_ids.get(0).unwrap_or(false));
        assert_eq!(false, collector.node_ids.get(1).unwrap_or(false));
        assert_eq!(true, collector.node_ids.get(2).unwrap_or(false));
    }
    #[test]
    #[should_panic(expected = "index out of bounds: 12 >= 10")]
    fn node_id_collector_out_of_bounds(){
        let mut collector = TestOnlyNodeIdCollector::new(10);
        collector.handle_node(Node::new(12, 1, Coordinate::new(1.0f64, 1.1f64), 1, 1, 1, "a".to_string(), true,vec![]));
    }
    #[test]
    fn node_id_collector_out_of_bounds_real(){
        let mut collector = TestOnlyNodeIdCollector::new(HIGHEST_NODE_ID as usize);

        collector.handle_node(Node::new(1, 1, Coordinate::new(1.0f64, 1.1f64), 1, 1, 1, "a".to_string(), true,vec![]));
        assert_eq!(false, collector.node_ids.get(0).unwrap_or(false));
        assert_eq!(true, collector.node_ids.get(1).unwrap_or(false));
        assert_eq!(false, collector.node_ids.get(2).unwrap_or(false));
        assert_eq!(false, collector.node_ids.get(11414456780).unwrap_or(false));

        collector.handle_node(Node::new(11414456780, 1, Coordinate::new(1.0f64, 1.1f64), 1, 1, 1, "a".to_string(), true,vec![]));
        assert_eq!(true, collector.node_ids.get(11414456780).unwrap_or(false));
    }

    #[test]
    fn metadata_remover_node() {
        let mut metadata_remover = MetadataRemover::default();
        let node = metadata_remover.handle_node(Node::new(1, 1, Coordinate::new(1.0f64, 1.1f64), 1, 1, 1, "a".to_string(), true,vec![
            Tag::new("a".to_string(), "x".to_string()),
            Tag::new("b".to_string(), "y".to_string()),
        ])).unwrap();
        assert_eq!(node.id(), 1);
        assert_eq!(node.version(), 0);
        assert_eq!(node.coordinate().lat(), 1.0f64);
        assert_eq!(node.coordinate().lon(), 1.1f64);
        assert_eq!(node.timestamp(), 0);
        assert_eq!(node.changeset(), 0);
        assert_eq!(node.uid(), 0);
        assert_eq!(node.user(), &String::default());
        assert_eq!(node.visible(), true);
        assert_eq!(node.tags()[0].k(), &"a".to_string());
        assert_eq!(node.tags()[0].v(), &"x".to_string());
        assert_eq!(node.tags()[1].k(), &"b".to_string());
        assert_eq!(node.tags()[1].v(), &"y".to_string());
    }

    #[test]
    fn metadata_remover_way() {
        let mut metadata_remover = MetadataRemover::default();
        let way = metadata_remover.handle_way(Way::new(1, 1, 1, 1, 1, "user".to_string(), true, vec![4, 6], vec![
            Tag::new("a".to_string(), "x".to_string()),
            Tag::new("b".to_string(), "y".to_string()),
        ])).unwrap();
        assert_eq!(way.id(), 1);
        assert_eq!(way.version(), 0);
        assert_eq!(way.timestamp(), 0);
        assert_eq!(way.changeset(), 0);
        assert_eq!(way.uid(), 0);
        assert_eq!(way.user(), &String::default());
        assert_eq!(way.visible(), true);
        assert_eq!(way.refs()[0], 4);
        assert_eq!(way.refs()[1], 6);
        assert_eq!(way.tags()[0].k(), &"a".to_string());
        assert_eq!(way.tags()[0].v(), &"x".to_string());
        assert_eq!(way.tags()[1].k(), &"b".to_string());
        assert_eq!(way.tags()[1].v(), &"y".to_string());
    }

    #[test]
    fn metadata_remover_relation() {
        let mut metadata_remover = MetadataRemover::default();
        let relation = metadata_remover.handle_relation(Relation::new(1, 1, 1, 1, 1, "user".to_string(), true, vec![
            Member::Node {member: MemberData::new(5, "a".to_string())},
            Member::Node {member: MemberData::new(6, "b".to_string())},
            Member::Way {member: MemberData::new(10, "b".to_string())},
            Member::Relation {member: MemberData::new(20, "b".to_string())},
        ], vec![
            Tag::new("a".to_string(), "x".to_string()),
            Tag::new("b".to_string(), "y".to_string()),
        ])).unwrap();
        assert_eq!(relation.id(), 1);
        assert_eq!(relation.version(), 0);
        assert_eq!(relation.timestamp(), 0);
        assert_eq!(relation.changeset(), 0);
        assert_eq!(relation.uid(), 0);
        assert_eq!(relation.user(), &String::default());
        assert_eq!(relation.visible(), true);
        assert_eq!(relation.members()[0], Member::Node {member: MemberData::new(5, "a".to_string())});
        assert_eq!(relation.members()[1], Member::Node {member: MemberData::new(6, "b".to_string())});
        assert_eq!(relation.members()[2], Member::Way {member: MemberData::new(10, "b".to_string())});
        assert_eq!(relation.members()[3], Member::Relation {member: MemberData::new(20, "b".to_string())});
        assert_eq!(relation.tags()[0].k(), &"a".to_string());
        assert_eq!(relation.tags()[0].v(), &"x".to_string());
        assert_eq!(relation.tags()[1].k(), &"b".to_string());
        assert_eq!(relation.tags()[1].v(), &"y".to_string());
    }
    
}