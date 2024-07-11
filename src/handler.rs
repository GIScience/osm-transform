use std::collections::HashMap;
use osm_io::osm::model::node::Node;
use osm_io::osm::model::way::Way;
use osm_io::osm::model::relation::Relation;
use osm_io::osm::model::tag::Tag;
use regex::Regex;

#[derive(Default,Debug)]
pub struct HandlerResult {//todo add HashMap to add results with configurable keys
    pub count_all_nodes: i32,
    pub count_accepted_nodes: i32,
    pub node_ids: Vec<i64>,
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

    fn add_result(&mut self, mut result: HandlerResult) -> HandlerResult {
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
    pub(crate) fn collect_result(&mut self) -> HandlerResult{
        let mut result = HandlerResult::default();
        for processor in &mut self.handlers {
            result = processor.add_result(result);
        }
        result
    }
    pub(crate) fn add(mut self, handler: Box<dyn Handler>) -> HandlerChain {
        self.handlers.push(handler);
        self
    }
    pub(crate) fn add_unboxed(mut self, handler: impl Handler + Sized + 'static) -> HandlerChain {
        self.handlers.push(Box::new(handler));
        self
    }
}


pub(crate) struct FinalHandler { //todo add node/way/relation ids and log/print those elements
}
impl FinalHandler {
    pub(crate) fn new() -> Self {
        FinalHandler { }
    }
}
impl Handler for FinalHandler {
    fn handle_node(&mut self, node: Node) -> Option<Node> {
        None
    }
}

#[derive(Debug)]
pub(crate) enum CountType {
    ALL,
    ACCEPTED,
}

#[derive(Default)]
pub(crate) struct NodeIdCollector {
    pub node_ids: Vec<i64>,
}
impl NodeIdCollector {
    pub(crate) fn new() -> Self {
        Self {
            node_ids: Vec::new(),
        }
    }
}
impl Handler for NodeIdCollector {
    fn handle_node(&mut self, node: Node) -> Option<Node> {
        self.node_ids.push(node.id());
        Some(node)
    }
    fn add_result(&mut self, mut result: HandlerResult) -> HandlerResult {
        result.node_ids = self.node_ids.clone();//todo optimize
        result
    }
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
enum FilterType {
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






struct ComplexElementsFilter {
    pub handle_types: OsmElementTypeSelection,
    pub has_good_key_predicate: HasOneOfTagKeysPredicate,
    pub has_good_key_value_predicate: HasTagKeyValuePredicate,
    pub has_bad_key_predicate: HasNoneOfTagKeysPredicate,
}
impl ComplexElementsFilter {
    fn new(handle_types: OsmElementTypeSelection,
           has_good_key_predicate: HasOneOfTagKeysPredicate,
           has_good_key_value_predicate: HasTagKeyValuePredicate,
           has_bad_key_predicate: HasNoneOfTagKeysPredicate) -> Self {
        Self {
            handle_types,
            has_good_key_predicate,
            has_good_key_value_predicate,
            has_bad_key_predicate,
        }
    }
    fn accept_by_tags(&mut self, tags: &Vec<Tag>) -> bool {
        ( self.has_good_key_predicate.test(tags) ||
            self.has_good_key_value_predicate.test(tags) )||
            self.has_bad_key_predicate.test(tags)
    }
}
impl Handler for ComplexElementsFilter {
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
    fn all() -> Self { Self { node: true, way: true, relation: true } }
    pub(crate) fn node_only() -> Self { Self { node: true, way: false, relation: false } }
    fn way_only() -> Self { Self { node: false, way: true, relation: false } }
    fn relation_only() -> Self { Self { node: false, way: false, relation: true } }
}

struct TagFilterByKey {
    pub handle_types: OsmElementTypeSelection,
    pub key_regex: Regex,
    pub filter_type: FilterType,
}
impl TagFilterByKey {
    fn new(handle_types: OsmElementTypeSelection, key_regex: Regex, filter_type: FilterType) -> Self {
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

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use osm_io::osm::model::coordinate::Coordinate;
    use osm_io::osm::model::node::Node;
    use osm_io::osm::model::tag::Tag;
    use regex::Regex;
    use simple_logger::SimpleLogger;
    use crate::handler::{CountType, FilterType, Handler, HandlerResult, ElementCounter, TagValueBasedOsmElementsFilter, FinalHandler, NodeIdCollector, TagFilterByKey, OsmElementTypeSelection, TagKeyBasedOsmElementsFilter, HasOneOfTagKeysPredicate, HasNoneOfTagKeysPredicate, HasTagKeyValuePredicate, ComplexElementsFilter, HandlerChain};

    const EXISTING_TAG: &str = "EXISTING_TAG";
    const MISSING_TAG: &str = "MISSING_TAG";

    fn existing_tag() -> String { "EXISTING_TAG".to_string() }

    fn missing_tag() -> String { "MISSING_TAG".to_string() }

    #[test]
    fn handler_chain() {
        SimpleLogger::new().init();

        let mut chain = HandlerChain::default()
            .add(Box::new(ElementCounter::new(
                OsmElementTypeSelection::node_only(),
                CountType::ALL)))
            .add_unboxed(TagValueBasedOsmElementsFilter::new(
                OsmElementTypeSelection::node_only(),
                existing_tag(),
                Regex::new(".*p.*").unwrap(),
                FilterType::AcceptMatching))
            .add(Box::new(TagValueBasedOsmElementsFilter::new(
                OsmElementTypeSelection::node_only(),
                existing_tag(),
                Regex::new(".*z.*").unwrap(),
                FilterType::RemoveMatching)))
            .add(Box::new(ElementCounter::new(
                OsmElementTypeSelection::node_only(),
                CountType::ACCEPTED)))
            .add(Box::new(NodeIdCollector::new()))
            .add(Box::new(FinalHandler::new()));

        handle_test_nodes_and_verify_result(chain);
    }

    fn handle_test_nodes_and_verify_result(mut handler_chain: HandlerChain) {
        handler_chain.process_node(Node::new(1, 1, Coordinate::new(1.0f64, 1.1f64), 1, 1, 1, "a".to_string(), true, vec![Tag::new(existing_tag(), "kasper".to_string())]));
        handler_chain.process_node(Node::new(2, 1, Coordinate::new(2.0f64, 1.2f64), 1, 1, 1, "a".to_string(), true, vec![Tag::new(existing_tag(), "seppl".to_string())]));
        handler_chain.process_node(Node::new(3, 1, Coordinate::new(3.0f64, 1.3f64), 1, 1, 1, "a".to_string(), true, vec![Tag::new(existing_tag(), "hotzenplotz".to_string())]));
        handler_chain.process_node(Node::new(4, 1, Coordinate::new(4.0f64, 1.4f64), 1, 1, 1, "a".to_string(), true, vec![Tag::new(existing_tag(), "großmutter".to_string())]));

        let mut result = handler_chain.collect_result();
        dbg!(&result);

        assert_eq!(result.count_all_nodes, 4);
        assert_eq!(result.count_accepted_nodes, 2);
        assert_eq!(result.node_ids, vec![1, 2]);
    }
    pub(crate) struct FinalCaptor {
        pub nodes: Vec<Node>,
        pub next: Option<Box<dyn Handler>>,
    }

    impl FinalCaptor {
        pub(crate) fn new() -> Self {
            FinalCaptor { next: None, nodes: vec![] }
        }
    }
    impl Handler for FinalCaptor {
        fn handle_node(&mut self, node: Node) -> Option<Node> {
            self.nodes.push(node);
            None
        }
    }

    #[test]
    fn test_tag_filter_by_key__remove_matching() {
        let mut tag_filter = TagFilterByKey::new(
            OsmElementTypeSelection::node_only(),
            Regex::new(".*bad.*").unwrap(),
            FilterType::RemoveMatching);

        let mut node_option = tag_filter.handle_node(Node::new(1, 1, Coordinate::new(1.0f64, 1.1f64), 1, 1, 1, "a".to_string(), true,
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
    fn test_tag_filter_by_key__remove_matching_complex_regex() {
        let mut tag_filter = TagFilterByKey::new(
            OsmElementTypeSelection::node_only(),
            Regex::new("(.*:)?source(:.*)?|(.*:)?note(:.*)?|url|created_by|fixme|wikipedia").unwrap(),
            FilterType::RemoveMatching);

        let mut node_option = tag_filter.handle_node(Node::new(1, 1, Coordinate::new(1.0f64, 1.1f64), 1, 1, 1, "a".to_string(), true,
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
    fn test_tag_filter_by_key__keep_matching() {
        let mut tag_filter = TagFilterByKey::new(
            OsmElementTypeSelection::all(),
            Regex::new(".*good.*").unwrap(),
            FilterType::AcceptMatching);

        let mut node_option = tag_filter.handle_node(Node::new(1, 1, Coordinate::new(1.0f64, 1.1f64), 1, 1, 1, "a".to_string(), true,
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
    fn test_tag_filter_by_key__node_not_handled() {
        let mut tag_filter = TagFilterByKey::new(
            OsmElementTypeSelection::way_only(),
            Regex::new(".*").unwrap(),
            FilterType::RemoveMatching);

        let mut node_option = tag_filter.handle_node(Node::new(1, 1, Coordinate::new(1.0f64, 1.1f64), 1, 1, 1, "a".to_string(), true,
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
    fn test_tag_filter_by_key__node_handled() {
        let mut tag_filter = TagFilterByKey::new(
            OsmElementTypeSelection::all(),
            Regex::new(".*").unwrap(),
            FilterType::RemoveMatching);

        let mut node_option = tag_filter.handle_node(Node::new(1, 1, Coordinate::new(1.0f64, 1.1f64), 1, 1, 1, "a".to_string(), true,
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
    fn has_one_of_tag_keys_predicate__only_matching_tags() {
        let mut predicate = HasOneOfTagKeysPredicate { keys: vec!["good".to_string(), "nice".to_string()] };
        assert_eq!(true, predicate.test(&vec![
            Tag::new("good".to_string(), "1".to_string()),
        ]));
    }
    #[test]
    fn has_one_of_tag_keys_predicate__only_all_matching_tags() {
        let mut predicate = HasOneOfTagKeysPredicate { keys: vec!["good".to_string(), "nice".to_string()] };
        assert_eq!(true, predicate.test(&vec![
            Tag::new("good".to_string(), "1".to_string()),
            Tag::new("nice".to_string(), "2".to_string()),
        ]));
    }
    #[test]
    fn has_one_of_tag_keys_predicate__also_matching_tags() {
        let mut predicate = HasOneOfTagKeysPredicate { keys: vec!["good".to_string(), "nice".to_string()] };
        assert_eq!(true, predicate.test(&vec![
            Tag::new("good".to_string(), "1".to_string()),
            Tag::new("bad".to_string(), "2".to_string()),
        ]));
    }
    #[test]
    fn has_one_of_tag_keys_predicate__no_matching_tags() {
        let mut predicate = HasOneOfTagKeysPredicate { keys: vec!["good".to_string(), "nice".to_string()] };
        assert_eq!(false, predicate.test(&vec![
            Tag::new("ugly".to_string(), "1".to_string()),
            Tag::new("bad".to_string(), "2".to_string()),
        ]));
    }

    #[test]
    fn has_tag_key_value_predicate__no_matching_tag() {
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
    fn has_tag_key_value_predicate__only_tag_with_wrong_value() {
        let mut key_values = HashMap::new();
        key_values.insert("good".to_string(), "good".to_string());
        key_values.insert("nice".to_string(), "nice".to_string());
        let mut predicate = HasTagKeyValuePredicate { key_values: key_values };
        assert_eq!(false, predicate.test(&vec![
            Tag::new("good".to_string(), "1".to_string()),
        ]));
    }
    #[test]
    fn has_tag_key_value_predicate__also_tag_with_wrong_value() {
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
    fn has_tag_key_value_predicate__only_tag_with_matching_value() {
        let mut key_values = HashMap::new();
        key_values.insert("good".to_string(), "good".to_string());
        key_values.insert("nice".to_string(), "nice".to_string());
        let mut predicate = HasTagKeyValuePredicate { key_values: key_values };
        assert_eq!(true, predicate.test(&vec![
            Tag::new("good".to_string(), "good".to_string()),
        ]));
    }
    #[test]
    fn has_tag_key_value_predicate__also_tag_with_matching_value() {
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
    fn has_none_of_tag_keys_predicate__only_non_matching_tag() {
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
    fn complex_filter() {
        let mut key_values = HashMap::new();
        key_values.insert("railway".to_string(), "platform".to_string());
        key_values.insert("public_transport".to_string(), "platform".to_string());
        key_values.insert("man_made".to_string(), "pier".to_string());

        let mut filter = ComplexElementsFilter::new(
            OsmElementTypeSelection::all(),
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
            });
        // has key to keep and key-value to keep, bad key 'building' should not take effect => should be accepted
        assert!(filter.handle_node(Node::new(1, 1, Coordinate::new(1.0f64, 1.1f64), 1, 1, 1, "a".to_string(), true,
                                             vec![
                                                 Tag::new("route".to_string(), "xyz".to_string()),
                                                 Tag::new("railway".to_string(), "platform".to_string()),
                                                 Tag::new("building".to_string(), "x".to_string()),
                                             ])).is_some());

        // has key to keep, bad key 'building' should not take effect => should be accepted
        assert!(filter.handle_node(Node::new(2, 1, Coordinate::new(1.0f64, 1.1f64), 1, 1, 1, "a".to_string(), true,
                                             vec![
                                                 Tag::new("route".to_string(), "xyz".to_string()),
                                                 Tag::new("building".to_string(), "x".to_string()),
                                             ])).is_some());

        // has key-value to keep, bad key 'building' should not take effect => should be accepted
        assert!(filter.handle_node(Node::new(3, 1, Coordinate::new(1.0f64, 1.1f64), 1, 1, 1, "a".to_string(), true,
                                             vec![
                                                 Tag::new("railway".to_string(), "platform".to_string()),
                                                 Tag::new("building".to_string(), "x".to_string()),
                                             ])).is_some());

        // has no key or key-value to keep, but also no bad key => should be accepted
        assert!(filter.handle_node(Node::new(4, 1, Coordinate::new(1.0f64, 1.1f64), 1, 1, 1, "a".to_string(), true,
                                             vec![
                                                 Tag::new("railway".to_string(), "wrong-value".to_string()),
                                                 Tag::new("something".to_string(), "else".to_string()),
                                             ])).is_some());

        // has no key or key-value to keep, some other key, but also one bad key => should be filtered
        assert!(filter.handle_node(Node::new(5, 1, Coordinate::new(1.0f64, 1.1f64), 1, 1, 1, "a".to_string(), true,
                                             vec![
                                                 Tag::new("railway".to_string(), "wrong-value".to_string()),
                                                 Tag::new("something".to_string(), "else".to_string()),
                                                 Tag::new("building".to_string(), "x".to_string()),
                                             ])).is_none());

        // has only one bad key => should be filtered
        assert!(filter.handle_node(Node::new(6, 1, Coordinate::new(1.0f64, 1.1f64), 1, 1, 1, "a".to_string(), true,
                                             vec![
                                                 Tag::new("building".to_string(), "x".to_string()),
                                             ])).is_none());

        // has only one other key => should be accepted
        assert!(filter.handle_node(Node::new(7, 1, Coordinate::new(1.0f64, 1.1f64), 1, 1, 1, "a".to_string(), true,
                                             vec![
                                                 Tag::new("something".to_string(), "x".to_string()),
                                             ])).is_some());
    }
}