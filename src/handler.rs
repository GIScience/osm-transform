use std::collections::HashMap;
use osm_io::osm::model::node::Node;
use osm_io::osm::model::way::Way;
use osm_io::osm::model::relation::Relation;
use osm_io::osm::model::tag::Tag;
use regex::Regex;

#[derive(Default,Debug)]
pub struct HandlerResult {
    pub count_all_nodes: i32,
    pub count_accepted_nodes: i32,
    pub node_ids: Vec<i64>,
    pub bbox_max_lon: f64,
    pub bbox_min_lon: f64,
    pub bbox_max_lat: f64,
    pub bbox_min_lat: f64,
}

pub trait Handler {

    fn process_node_owned(&mut self, node: Node) -> Option<Node> {
        Some(node)
    }

    fn process_node(&mut self, node: &mut Node) -> bool {true}

    fn handle_node_chained_owned(&mut self, node: Node) {
        let node = self.process_node_owned(node);
        if node.is_some() {
            if let Some(next) = &mut self.get_next() {
                next.handle_node_chained_owned(node.unwrap());
            }
        }
    }

    fn handle_node_chained(&mut self, node: &mut Node) {
        if self.process_node(node) {
            if let Some(next) = &mut self.get_next() {
                next.handle_node_chained(node);
            }
        }
    }

    fn process_way(&mut self, way: &mut Way) -> bool {true}

    fn handle_way_chained(&mut self, way: &mut Way) {
        if self.process_way(way) {
            if let Some(next) = &mut self.get_next() {
                next.handle_way_chained(way);
            }
        }
    }
    fn process_relation(&mut self, relation: &mut Relation) -> bool {true}

    fn handle_relation_chained(&mut self, relation: &mut Relation) {
        if self.process_relation(relation) {
            if let Some(next) = &mut self.get_next() {
                next.handle_relation_chained(relation);
            }
        }
    }

    fn get_next(&mut self) -> &mut Option<Box<dyn Handler>>;

    fn process_results(&mut self, res: &mut HandlerResult) {}

    fn get_results_chained(&mut self, res: &mut HandlerResult) {
        self.process_results(res);
        if let Some(next) = &mut self.get_next() {
            next.get_results_chained(res);
        }
    }
}

pub fn into_next(handler: impl Handler + Sized + 'static) -> Option<Box<dyn Handler>> {
    Some(Box::new(handler))
}

pub(crate) struct FinalHandler {
    next: Option<Box<dyn Handler>>,
}
impl FinalHandler {
    pub(crate) fn new() -> Self {
        FinalHandler { next: None }
    }
}
impl Handler for FinalHandler {
    fn get_next(&mut self) -> &mut Option<Box<dyn Handler>> {
        &mut self.next
    }
}

#[derive(Debug)]
pub(crate) enum CountType {
    ALL,
    ACCEPTED,
}

#[derive(Default)]
pub(crate) struct NodeIdCollector {
    pub next: Option<Box<dyn Handler>>,
    pub node_ids: Vec<i64>,
}
impl NodeIdCollector {
    pub(crate) fn new(next: impl Handler + 'static) -> Self {
        Self {
            next: into_next(next),
            node_ids: Vec::new(),
        }
    }
}
impl Handler for NodeIdCollector {
    fn process_node(&mut self, node: &mut Node) -> bool {
        self.node_ids.push(node.id());
        true
    }

    fn get_next(&mut self) -> &mut Option<Box<dyn Handler>> {
        return &mut self.next;
    }

    fn process_results(&mut self, res: &mut HandlerResult) {
        res.node_ids = self.node_ids.clone();//todo optimize
    }
}

pub(crate) struct ElementCounter {
    pub next: Option<Box<dyn Handler>>,
    pub nodes_count: i32,
    pub ways_count: i32,
    pub relations_count: i32,
    pub handle_types: OsmElementTypeSelection,
    pub count_type: CountType,
}
impl ElementCounter {
    pub fn new(handle_types: OsmElementTypeSelection, count_type: CountType, next: impl Handler + 'static) -> Self {
        Self {
            next: into_next(next),
            nodes_count: 0,
            ways_count: 0,
            relations_count: 0,
            handle_types,
            count_type,
        }
    }
}
impl Handler for ElementCounter {
    fn process_node_owned(&mut self, node: Node) -> Option<Node> {
        if self.handle_types.node {
            self.nodes_count += 1;
        }
        Some(node)
    }
    fn process_node(&mut self, node: &mut Node) -> bool {
        if self.handle_types.node {
            self.nodes_count += 1;
        }
        true
    }
    fn process_way(&mut self, way: &mut Way) -> bool {
        if self.handle_types.way {
            self.ways_count += 1;
        }
        true
    }
    fn process_relation(&mut self, relation: &mut Relation) -> bool {
        if self.handle_types.relation {
            self.relations_count += 1;
        }
        true
    }

    fn get_next(&mut self) -> &mut Option<Box<dyn Handler>> {
        return &mut self.next;
    }
    fn process_results(&mut self, mut result: &mut HandlerResult) {
        match self.count_type {
            CountType::ALL => { result.count_all_nodes = self.nodes_count }
            CountType::ACCEPTED => { result.count_accepted_nodes = self.nodes_count }
        }
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
    pub next: Option<Box<dyn Handler>>,
}
impl TagValueBasedOsmElementsFilter {
    fn new(handle_types: OsmElementTypeSelection, tag_key: String, tag_value_regex: Regex, filter_type: FilterType, next: impl Handler + 'static) -> Self {
        Self {
            handle_types,
            tag_key,
            tag_value_regex,
            filter_type,
            next: into_next(next),
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
    fn handle_node_chained(&mut self, node: &mut Node) {
        let mut accept = true;
        if self.handle_types.node  {
            accept = self.accept_by_tags(&node.tags())
        }
        if accept {
            if let Some(next_handler) = self.get_next() {
                next_handler.handle_node_chained(node)
            }
        }
    }
    fn handle_way_chained(&mut self, way: &mut Way) {
        let mut accept = true;
        if self.handle_types.way  {
            accept = self.accept_by_tags(&way.tags())
        }
        if accept {
            if let Some(next_handler) = self.get_next() {
                next_handler.handle_way_chained(way)
            }
        }
    }
    fn handle_relation_chained(&mut self, relation: &mut Relation) {
        let mut accept = true;
        if self.handle_types.relation  {
            accept = self.accept_by_tags(&relation.tags())
        }
        if accept {
            if let Some(next_handler) = self.get_next() {
                next_handler.handle_relation_chained(relation)
            }
        }
    }

    fn get_next(&mut self) -> &mut Option<Box<dyn Handler>> {
        return &mut self.next;
    }
}






struct TagKeyBasedOsmElementsFilter {
    pub handle_types: OsmElementTypeSelection,
    pub tag_keys: Vec<String>,
    pub filter_type: FilterType,
    pub next: Option<Box<dyn Handler>>,
}
impl TagKeyBasedOsmElementsFilter {
    fn new(handle_types: OsmElementTypeSelection, tag_keys: Vec<String>, filter_type: FilterType, next: impl Handler + 'static) -> Self {
        Self {
            handle_types,
            tag_keys,
            filter_type,
            next: into_next(next),
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
    fn handle_node_chained(&mut self, node: &mut Node) {
        let mut accept = true;
        if self.handle_types.node  {
            accept = self.accept_by_tags(&node.tags())
        }
        if accept {
            if let Some(next_handler) = self.get_next() {
                next_handler.handle_node_chained(node)
            }
        }
    }

    fn get_next(&mut self) -> &mut Option<Box<dyn Handler>> {
        return &mut self.next;
    }
}






struct ComplexElementsFilter {
    pub handle_types: OsmElementTypeSelection,
    pub has_good_key_predicate: HasOneOfTagKeysPredicate,
    pub has_good_key_value_predicate: HasTagKeyValuePredicate,
    pub has_bad_key_predicate: HasNoneOfTagKeysPredicate,
    pub next: Option<Box<dyn Handler>>,
}
impl ComplexElementsFilter {
    fn new(handle_types: OsmElementTypeSelection,
           has_good_key_predicate: HasOneOfTagKeysPredicate,
           has_good_key_value_predicate: HasTagKeyValuePredicate,
           has_bad_key_predicate: HasNoneOfTagKeysPredicate,
           next: impl Handler + 'static) -> Self {
        Self {
            handle_types,
            has_good_key_predicate,
            has_good_key_value_predicate,
            has_bad_key_predicate,
            next: into_next(next),
        }
    }
    fn accept_by_tags(&mut self, tags: &Vec<Tag>) -> bool {
        ( self.has_good_key_predicate.test(tags) ||
            self.has_good_key_value_predicate.test(tags) )||
            self.has_bad_key_predicate.test(tags)
    }
}
impl Handler for ComplexElementsFilter {
    fn handle_node_chained(&mut self, node: &mut Node) {
        let mut accept = true;
        if self.handle_types.node  {
            accept = self.accept_by_tags(&node.tags())
        }
        if accept {
            if let Some(next_handler) = self.get_next() {
                next_handler.handle_node_chained(node)
            }
        }
    }
    fn handle_way_chained(&mut self, way: &mut Way) {
        let mut accept = true;
        if self.handle_types.way  {
            accept = self.accept_by_tags(&way.tags())
        }
        if accept {
            if let Some(next_handler) = self.get_next() {
                next_handler.handle_way_chained(way)
            }
        }
    }
    fn handle_relation_chained(&mut self, relation: &mut Relation) {
        let mut accept = true;
        if self.handle_types.relation  {
            accept = self.accept_by_tags(&relation.tags())
        }
        if accept {
            if let Some(next_handler) = self.get_next() {
                next_handler.handle_relation_chained(relation)
            }
        }
    }

    fn get_next(&mut self) -> &mut Option<Box<dyn Handler>> {
        return &mut self.next;
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
    fn node_only() -> Self { Self { node: true, way: false, relation: false } }
    fn way_only() -> Self { Self { node: false, way: true, relation: false } }
    fn relation_only() -> Self { Self { node: false, way: false, relation: true } }
}

struct TagFilterByKey {
    pub handle_types: OsmElementTypeSelection,
    pub key_regex: Regex,
    pub filter_type: FilterType,
    pub next: Option<Box<dyn Handler>>,
}
impl TagFilterByKey {
    fn new(handle_types: OsmElementTypeSelection, key_regex: Regex, filter_type: FilterType, next: impl Handler + 'static) -> Self {
        Self {
            handle_types,
            key_regex,
            filter_type,
            next: into_next(next),
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
    fn process_node(&mut self, node: &mut Node) -> bool {
        if self.handle_types.node  {
            self.filter_tags(&mut node.tags_mut());
        }
        true
    }
    fn process_relation(&mut self, relation: &mut Relation) -> bool {
        if self.handle_types.relation  {
            self.filter_tags(&mut relation.tags_mut());
        }
        true
    }
    fn process_way(&mut self, way: &mut Way) -> bool {
        if self.handle_types.way  {
            self.filter_tags(&mut way.tags_mut());
        }
        true
    }
    fn get_next(&mut self) -> &mut Option<Box<dyn Handler>> {
        return &mut self.next;
    }
}



#[derive(Default)]
pub(crate) struct BboxCollector {
    pub next: Option<Box<dyn Handler>>,
    pub min_lat: f64,
    pub min_lon: f64,
    pub max_lat: f64,
    pub max_lon: f64,
}

impl BboxCollector {
    pub(crate) fn new(next: impl Handler + 'static) -> Self {
        Self {
            next: into_next(next),
            min_lat: f64::MAX,
            min_lon: f64::MAX,
            max_lat: f64::MIN,
            max_lon: f64::MIN,
        }
    }
}

impl Handler for BboxCollector {
    fn process_node(&mut self, node: &mut Node) -> bool {
        if &self.min_lat == &0.0 {
            // self.set_min_lat(node.coordinate().lat());
            self.min_lat = node.coordinate().lat();
        }
        if &self.min_lon == &0.0 {
            self.min_lon = node.coordinate().lon()
        }

        if node.coordinate().lat() < self.min_lat {
            // self.set_min_lat(node.coordinate().lat());
            self.min_lat = node.coordinate().lat();
        }
        if node.coordinate().lon() < self.min_lon {
            self.min_lon = node.coordinate().lon()
        }

        if node.coordinate().lat() > self.max_lat {
            self.max_lat = node.coordinate().lat()
        }
        if node.coordinate().lon() > self.max_lon {
            self.max_lon = node.coordinate().lon()
        }
        true
    }
    fn get_next(&mut self) -> &mut Option<Box<dyn Handler>> {
        return &mut self.next;
    }
    fn process_results(&mut self, res: &mut HandlerResult) {
        res.bbox_max_lon = self.max_lon;
        res.bbox_min_lon = self.min_lon;
        res.bbox_max_lat = self.max_lat;
        res.bbox_min_lat = self.min_lat;
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
    use crate::handler::{BboxCollector, CountType, FilterType, Handler, HandlerResult, ElementCounter, TagValueBasedOsmElementsFilter, FinalHandler, NodeIdCollector, TagFilterByKey, OsmElementTypeSelection, TagKeyBasedOsmElementsFilter, HasOneOfTagKeysPredicate, HasNoneOfTagKeysPredicate, HasTagKeyValuePredicate, ComplexElementsFilter};

    const EXISTING_TAG: &str = "EXISTING_TAG";
    const MISSING_TAG: &str = "MISSING_TAG";

    fn existing_tag() -> String { "EXISTING_TAG".to_string() }

    fn missing_tag() -> String { "MISSING_TAG".to_string() }

    #[test]
    fn test_handle_nodes_with_manually_chanied_handlers() {
        SimpleLogger::new().init();
        let mut handler =
            ElementCounter::new(
                OsmElementTypeSelection::node_only(),
                CountType::ALL,
                TagValueBasedOsmElementsFilter::new(
                    OsmElementTypeSelection::node_only(),
                    existing_tag(),
                    Regex::new(".*p.*").unwrap(),
                    FilterType::AcceptMatching,
                    TagValueBasedOsmElementsFilter::new(
                        OsmElementTypeSelection::node_only(),
                        existing_tag(),
                        Regex::new(".*z.*").unwrap(),
                        FilterType::RemoveMatching,
                        BboxCollector::new(
                            ElementCounter::new(
                                OsmElementTypeSelection::node_only(),
                                CountType::ACCEPTED,
                                NodeIdCollector::new(
                                    FinalHandler::new()
                                )
                            )
                        )
                    )
                )
            );
        handle_test_nodes_and_verify_result(&mut handler);
    }

    fn handle_test_nodes_and_verify_result(handler: &mut dyn Handler) {
        handler.handle_node_chained(&mut Node::new(1, 1, Coordinate::new(1.0f64, 1.1f64), 1, 1, 1, "a".to_string(), true, vec![Tag::new(existing_tag(), "kasper".to_string())]));
        handler.handle_node_chained(&mut Node::new(2, 1, Coordinate::new(2.0f64, 1.2f64), 1, 1, 1, "a".to_string(), true, vec![Tag::new(existing_tag(), "seppl".to_string())]));
        handler.handle_node_chained(&mut Node::new(3, 1, Coordinate::new(3.0f64, 1.3f64), 1, 1, 1, "a".to_string(), true, vec![Tag::new(existing_tag(), "hotzenplotz".to_string())]));
        handler.handle_node_chained(&mut Node::new(4, 1, Coordinate::new(4.0f64, 1.4f64), 1, 1, 1, "a".to_string(), true, vec![Tag::new(existing_tag(), "großmutter".to_string())]));

        let mut result = HandlerResult::default();
        handler.get_results_chained(&mut result);
        dbg!(&result);

        assert_eq!(result.count_all_nodes, 4);
        assert_eq!(result.count_accepted_nodes, 2);
        assert_eq!(result.node_ids, vec![1, 2]);
        //BBox based on only filtered (accepted) nodes!
        assert_eq!(result.bbox_min_lat, 1.0f64);
        assert_eq!(result.bbox_min_lon, 1.1f64);
        assert_eq!(result.bbox_max_lat, 2.0f64);
        assert_eq!(result.bbox_max_lon, 1.2f64);
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
        fn process_node(&mut self, node: &mut Node) -> bool {
            self.nodes.push(node.clone());
            true
        }
        fn get_next(&mut self) -> &mut Option<Box<dyn Handler>> {
            &mut self.next
        }
    }

    #[test]
    fn test_tag_filter_by_key__remove_matching() {
        let mut tag_filter = TagFilterByKey::new(
            OsmElementTypeSelection::node_only(),
            Regex::new(".*bad.*").unwrap(),
            FilterType::RemoveMatching,
            FinalHandler::new());

        let mut node = Node::new(1, 1, Coordinate::new(1.0f64, 1.1f64), 1, 1, 1, "a".to_string(), true,
                                 vec![
                                     Tag::new("bad".to_string(), "hotzenplotz".to_string()),
                                     Tag::new("good".to_string(), "kasper".to_string()),
                                     Tag::new("more-bad".to_string(), "vader".to_string()),
                                     Tag::new("more-good".to_string(), "grandma".to_string()),
                                     Tag::new("badest".to_string(), "voldemort".to_string()),
                                 ]);
        tag_filter.process_node(&mut node);
        dbg!(&node);
        assert_eq!(node.tags().len(), 2);
        assert_eq!(node.tags()[0].k(), &"good");
        assert_eq!(node.tags()[0].v(), &"kasper");
        assert_eq!(node.tags()[1].k(), &"more-good");
        assert_eq!(node.tags()[1].v(), &"grandma");
    }

    #[test]
    fn test_tag_filter_by_key__remove_matching_complex_regex() {
        let mut tag_filter = TagFilterByKey::new(
            OsmElementTypeSelection::node_only(),
            Regex::new("(.*:)?source(:.*)?|(.*:)?note(:.*)?|url|created_by|fixme|wikipedia").unwrap(),
            FilterType::RemoveMatching,
            FinalHandler::new());

        let mut node = Node::new(1, 1, Coordinate::new(1.0f64, 1.1f64), 1, 1, 1, "a".to_string(), true,
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
                                 ]);
        tag_filter.process_node(&mut node);
        dbg!(&node);
        assert_eq!(node.tags().len(), 1);
        for tag in node.tags() {
            assert_eq!(tag.v(), "good")
        }
    }

    #[test]
    fn test_tag_filter_by_key__keep_matching() {
        let mut tag_filter = TagFilterByKey::new(
            OsmElementTypeSelection::all(),
            Regex::new(".*good.*").unwrap(),
            FilterType::AcceptMatching,
            FinalHandler::new());

        let mut node = Node::new(1, 1, Coordinate::new(1.0f64, 1.1f64), 1, 1, 1, "a".to_string(), true,
                                 vec![
                                     Tag::new("bad".to_string(), "hotzenplotz".to_string()),
                                     Tag::new("good".to_string(), "kasper".to_string()),
                                     Tag::new("more-bad".to_string(), "vader".to_string()),
                                     Tag::new("more-good".to_string(), "grandma".to_string()),
                                     Tag::new("badest".to_string(), "voldemort".to_string()),
                                 ]);
        tag_filter.process_node(&mut node);
        dbg!(&node);
        assert_eq!(node.tags().len(), 2);
        assert_eq!(node.tags()[0].k(), &"good");
        assert_eq!(node.tags()[0].v(), &"kasper");
        assert_eq!(node.tags()[1].k(), &"more-good");
        assert_eq!(node.tags()[1].v(), &"grandma");
    }
    #[test]
    fn test_tag_filter_by_key__node_not_handled() {
        let mut tag_filter = TagFilterByKey::new(
            OsmElementTypeSelection::way_only(),
            Regex::new(".*").unwrap(),
            FilterType::RemoveMatching,
            FinalHandler::new());

        let mut node = Node::new(1, 1, Coordinate::new(1.0f64, 1.1f64), 1, 1, 1, "a".to_string(), true,
                                 vec![
                                     Tag::new("a".to_string(), "1".to_string()),
                                     Tag::new("b".to_string(), "2".to_string()),
                                     Tag::new("c".to_string(), "3".to_string()),
                                 ]);
        tag_filter.process_node(&mut node);
        assert_eq!(node.tags().len(), 3);
    }
    #[test]
    fn test_tag_filter_by_key__node_handled() {
        let mut tag_filter = TagFilterByKey::new(
            OsmElementTypeSelection::all(),
            Regex::new(".*").unwrap(),
            FilterType::RemoveMatching,
            FinalHandler::new());

        let mut node = Node::new(1, 1, Coordinate::new(1.0f64, 1.1f64), 1, 1, 1, "a".to_string(), true,
                                 vec![
                                     Tag::new("a".to_string(), "1".to_string()),
                                     Tag::new("b".to_string(), "2".to_string()),
                                     Tag::new("c".to_string(), "3".to_string()),
                                 ]);
        tag_filter.process_node(&mut node);
        dbg!(&node);
        assert_eq!(node.tags().len(), 0);
    }
    #[test]
    fn filter_elements_remove_by_keys() {
        let mut filter = TagKeyBasedOsmElementsFilter::new(
            OsmElementTypeSelection::all(),
            vec!["bad".to_string(), "ugly".to_string()],
            FilterType::RemoveMatching,
            ElementCounter::new(
                OsmElementTypeSelection::all(),
                CountType::ALL,
                FinalHandler::new()
            )
        );

        filter.handle_node_chained(&mut Node::new(1, 1, Coordinate::new(1.0f64, 1.1f64), 1, 1, 1, "a".to_string(), true,
                                                  vec![
                                                      Tag::new("good".to_string(), "1".to_string()),
                                                      Tag::new("bad".to_string(), "2".to_string()),
                                                  ]));
        filter.handle_node_chained(&mut Node::new(1, 1, Coordinate::new(1.0f64, 1.1f64), 1, 1, 1, "a".to_string(), true,
                                                  vec![
                                                      Tag::new("good".to_string(), "1".to_string()),
                                                      Tag::new("nice".to_string(), "2".to_string()),
                                                  ]));
        filter.handle_node_chained(&mut Node::new(1, 1, Coordinate::new(1.0f64, 1.1f64), 1, 1, 1, "a".to_string(), true,
                                                  vec![
                                                      Tag::new("ugly".to_string(), "1".to_string()),
                                                      Tag::new("bad".to_string(), "2".to_string()),
                                                  ]));
        let mut result = HandlerResult::default();
        filter.get_results_chained(&mut result);
        dbg!(&result);

        assert_eq!(result.count_all_nodes, 1);
    }
    #[test]
    fn filter_elements_accept_by_keys() {
        let mut filter = TagKeyBasedOsmElementsFilter::new(
            OsmElementTypeSelection::all(),
            vec!["bad".to_string(), "ugly".to_string()],
            FilterType::AcceptMatching,
            ElementCounter::new(
                OsmElementTypeSelection::all(),
                CountType::ALL,
                FinalHandler::new()
            )
        );

        filter.handle_node_chained(&mut Node::new(1, 1, Coordinate::new(1.0f64, 1.1f64), 1, 1, 1, "a".to_string(), true,
                                                  vec![
                                                      Tag::new("good".to_string(), "1".to_string()),
                                                      Tag::new("bad".to_string(), "2".to_string()),
                                                  ]));
        filter.handle_node_chained(&mut Node::new(1, 1, Coordinate::new(1.0f64, 1.1f64), 1, 1, 1, "a".to_string(), true,
                                                  vec![
                                                      Tag::new("good".to_string(), "1".to_string()),
                                                      Tag::new("nice".to_string(), "2".to_string()),
                                                  ]));
        filter.handle_node_chained(&mut Node::new(1, 1, Coordinate::new(1.0f64, 1.1f64), 1, 1, 1, "a".to_string(), true,
                                                  vec![
                                                      Tag::new("ugly".to_string(), "1".to_string()),
                                                      Tag::new("bad".to_string(), "2".to_string()),
                                                  ]));
        let mut result = HandlerResult::default();
        filter.get_results_chained(&mut result);
        dbg!(&result);

        assert_eq!(result.count_all_nodes, 2);
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

        let mut filter = ElementCounter::new(
            OsmElementTypeSelection::all(),
            CountType::ALL,
            ComplexElementsFilter::new(
                OsmElementTypeSelection::all(),
                HasOneOfTagKeysPredicate { keys: vec!["highway".to_string(), "route".to_string()] },
                HasTagKeyValuePredicate { key_values: key_values },
                HasNoneOfTagKeysPredicate { keys: vec![
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
                    "man_made".to_string()] },
                NodeIdCollector::new(
                    ElementCounter::new(
                        OsmElementTypeSelection::all(),
                        CountType::ACCEPTED,
                        FinalHandler::new())
                )));
        // has key to keep and key-value to keep, bad key 'building' should not take effect => should be accepted
        filter.handle_node_chained(&mut Node::new(1, 1, Coordinate::new(1.0f64, 1.1f64), 1, 1, 1, "a".to_string(), true,
                                                  vec![
                                                      Tag::new("route".to_string(), "xyz".to_string()),
                                                      Tag::new("railway".to_string(), "platform".to_string()),
                                                      Tag::new("building".to_string(), "x".to_string()),
                                                  ]));

        // has key to keep, bad key 'building' should not take effect => should be accepted
        filter.handle_node_chained(&mut Node::new(2, 1, Coordinate::new(1.0f64, 1.1f64), 1, 1, 1, "a".to_string(), true,
                                                  vec![
                                                      Tag::new("route".to_string(), "xyz".to_string()),
                                                      Tag::new("building".to_string(), "x".to_string()),
                                                  ]));

        // has key-value to keep, bad key 'building' should not take effect => should be accepted
        filter.handle_node_chained(&mut Node::new(3, 1, Coordinate::new(1.0f64, 1.1f64), 1, 1, 1, "a".to_string(), true,
                                                  vec![
                                                      Tag::new("railway".to_string(), "platform".to_string()),
                                                      Tag::new("building".to_string(), "x".to_string()),
                                                  ]));

        // has no key or key-value to keep, but also no bad key => should be accepted
        filter.handle_node_chained(&mut Node::new(4, 1, Coordinate::new(1.0f64, 1.1f64), 1, 1, 1, "a".to_string(), true,
                                                  vec![
                                                      Tag::new("railway".to_string(), "wrong-value".to_string()),
                                                      Tag::new("something".to_string(), "else".to_string()),
                                                  ]));

        // has no key or key-value to keep, some other key, but also one bad key => should be filtered
        filter.handle_node_chained(&mut Node::new(5, 1, Coordinate::new(1.0f64, 1.1f64), 1, 1, 1, "a".to_string(), true,
                                                  vec![
                                                      Tag::new("railway".to_string(), "wrong-value".to_string()),
                                                      Tag::new("something".to_string(), "else".to_string()),
                                                      Tag::new("building".to_string(), "x".to_string()),
                                                  ]));

        // has only one bad key => should be filtered
        filter.handle_node_chained(&mut Node::new(6, 1, Coordinate::new(1.0f64, 1.1f64), 1, 1, 1, "a".to_string(), true,
                                                  vec![
                                                      Tag::new("building".to_string(), "x".to_string()),
                                                  ]));

        // has only one other key => should be accepted
        filter.handle_node_chained(&mut Node::new(7, 1, Coordinate::new(1.0f64, 1.1f64), 1, 1, 1, "a".to_string(), true,
                                                  vec![
                                                      Tag::new("something".to_string(), "x".to_string()),
                                                  ]));

        let mut result = HandlerResult::default();
        filter.get_results_chained(&mut result);
        dbg!(&result);

        assert_eq!(result.count_all_nodes, 7);
        assert_eq!(result.count_accepted_nodes, 5);
        assert_eq!(result.node_ids, vec![1, 2, 3, 4, 7])
    }
}