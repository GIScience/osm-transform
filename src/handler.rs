use osm_io::osm::model::relation::Relation;
use osm_io::osm::model::tag::Tag;
use osm_io::osm::model::way::Way;
use regex::Regex;
use crate::osm_model::MutableNode;

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
    fn process_node(&mut self, node: &mut MutableNode) -> bool {true}

    fn handle_node_chained(&mut self, node: &mut MutableNode) {
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
    fn process_node(&mut self, node: &mut MutableNode) -> bool {
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

pub(crate) struct NodesCounter {
    pub count: i32,
    pub count_type: CountType,
    pub next: Option<Box<dyn Handler>>,
}
impl NodesCounter {
    pub fn new(count_type: CountType, next: impl Handler + 'static) -> Self {
        Self {
            next: into_next(next),
            count: 0,
            count_type,
        }
    }

}
impl Handler for NodesCounter {
    fn process_node(&mut self, node: &mut MutableNode) -> bool {
        self.count += 1;
        true
    }

    fn get_next(&mut self) -> &mut Option<Box<dyn Handler>> {
        return &mut self.next;
    }
    fn process_results(&mut self, mut result: &mut HandlerResult) {
        match self.count_type {
            CountType::ALL => { result.count_all_nodes = self.count }
            CountType::ACCEPTED => { result.count_accepted_nodes = self.count }
        }
    }
}

#[derive(Debug)]
enum FilterType {
    AcceptMatching,
    RemoveMatching,
}

struct TagValueBasedNodesFilter {
    pub tag: String,
    pub value_regex: Regex,
    pub filter_type: FilterType,
    pub next: Option<Box<dyn Handler>>,
}
impl TagValueBasedNodesFilter {
    fn new(tag: String, regex: Regex, filter_type: FilterType, next: impl Handler + 'static) -> Self {
        Self {
            next: into_next(next),
            tag: tag,
            value_regex: regex,
            filter_type: filter_type
        }
    }
}
impl Handler for TagValueBasedNodesFilter {
    fn handle_node_chained(&mut self, node: &mut MutableNode) {
        let mut accept = false;

        match self.filter_type {
            FilterType::AcceptMatching => {
                accept = false;
                for tag in node.tags() {
                    if self.tag.eq(tag.k()) && self.value_regex.is_match(tag.v()) {
                        accept = true;
                        break
                    }
                }
            }
            FilterType::RemoveMatching => {
                for tag in node.tags() {
                    accept = true;
                    if self.tag.eq(tag.k()) && self.value_regex.is_match(tag.v()) {
                        accept = false;
                        break;
                    }
                }
            }
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

struct PbfTypeSwitch {
    pub node: bool,
    pub way: bool,
    pub relation: bool,
}
struct TagFilterByKey {
    pub handle_types: PbfTypeSwitch,
    pub key_regex: Regex,
    pub filter_type: FilterType,
    pub next: Option<Box<dyn Handler>>,
}
impl TagFilterByKey {
    fn new(handle_types: PbfTypeSwitch, key_regex: Regex, filter_type: FilterType, next: impl Handler + 'static) -> Self {
        Self {
            handle_types,
            key_regex,
            filter_type,
            next: into_next(next),
        }
    }

    fn filter_tags(&mut self, tags: &Vec<Tag>) -> Vec<Tag> {
        let mut new_tags = vec![];
        for tag in tags {
            match self.filter_type {
                FilterType::AcceptMatching => {
                    if self.key_regex.is_match(tag.k()) {
                        new_tags.push(tag.clone());
                    }
                }
                FilterType::RemoveMatching => {
                    if !self.key_regex.is_match(tag.k()) {
                        new_tags.push(tag.clone());
                    }
                }
            }
        }
        new_tags
    }
}
impl Handler for TagFilterByKey {
    fn process_node(&mut self, node: &mut MutableNode) -> bool {
        if self.handle_types.node  {
            node.tags = self.filter_tags(&node.tags());
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
    fn process_node(&mut self, node: &mut MutableNode) -> bool {
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
    use osm_io::osm::model::coordinate::Coordinate;
    use osm_io::osm::model::node::Node;
    use osm_io::osm::model::tag::Tag;
    use regex::Regex;
    use simple_logger::SimpleLogger;
    use crate::handler::{BboxCollector, CountType, FilterType, Handler, HandlerResult, NodesCounter, TagValueBasedNodesFilter, FinalHandler, NodeIdCollector, TagFilterByKey, PbfTypeSwitch};
    use crate::osm_model::MutableNode;

    const EXISTING_TAG: &str = "EXISTING_TAG";
    const MISSING_TAG: &str = "MISSING_TAG";

    fn existing_tag() -> String { "EXISTING_TAG".to_string() }

    fn missing_tag() -> String { "MISSING_TAG".to_string() }

    #[test]
    fn test_handle_nodes_with_manually_chanied_handlers() {
        SimpleLogger::new().init();
        let mut handler =
            NodesCounter::new(
                CountType::ALL,
                TagValueBasedNodesFilter::new(
                    existing_tag(),
                    Regex::new(".*p.*").unwrap(),
                    FilterType::AcceptMatching,
                    TagValueBasedNodesFilter::new(
                        existing_tag(),
                        Regex::new(".*z.*").unwrap(),
                        FilterType::RemoveMatching,
                        BboxCollector::new(
                            NodesCounter::new(
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
        handler.handle_node_chained(&mut MutableNode::new(&mut Node::new(1, 1, Coordinate::new(1.0f64, 1.1f64), 1, 1, 1, "a".to_string(), true, vec![Tag::new(existing_tag(), "kasper".to_string())])));
        handler.handle_node_chained(&mut MutableNode::new(&mut Node::new(2, 1, Coordinate::new(2.0f64, 1.2f64), 1, 1, 1, "a".to_string(), true, vec![Tag::new(existing_tag(), "seppl".to_string())])));
        handler.handle_node_chained(&mut MutableNode::new(&mut Node::new(3, 1, Coordinate::new(3.0f64, 1.3f64), 1, 1, 1, "a".to_string(), true, vec![Tag::new(existing_tag(), "hotzenplotz".to_string())])));
        handler.handle_node_chained(&mut MutableNode::new(&mut Node::new(4, 1, Coordinate::new(4.0f64, 1.4f64), 1, 1, 1, "a".to_string(), true, vec![Tag::new(existing_tag(), "großmutter".to_string())])));

        let mut result = HandlerResult::default();
        handler.get_results_chained(&mut result);
        dbg!(&result);

        assert_eq!(result.count_all_nodes, 4);
        assert_eq!(result.count_accepted_nodes, 2);
        assert_eq!(result.node_ids, vec![1,2]);
        //BBox based on only filtered (accepted) nodes!
        assert_eq!(result.bbox_min_lat, 1.0f64);
        assert_eq!(result.bbox_min_lon, 1.1f64);
        assert_eq!(result.bbox_max_lat, 2.0f64);
        assert_eq!(result.bbox_max_lon, 1.2f64);
    }
    pub(crate) struct FinalCaptor {
        pub nodes: Vec<MutableNode>,
        pub next: Option<Box<dyn Handler>>,
    }
    impl FinalCaptor {
        pub(crate) fn new() -> Self {
            FinalCaptor { next: None, nodes: vec![] }
        }
    }
    impl Handler for FinalCaptor {
        fn process_node(&mut self, node: &mut MutableNode) -> bool {
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
            PbfTypeSwitch{node:true, way:false, relation:false},
            Regex::new(".*bad.*").unwrap(),
            FilterType::RemoveMatching,
            FinalHandler::new());

        let mut node = MutableNode::new(&mut Node::new(1, 1, Coordinate::new(1.0f64, 1.1f64), 1, 1, 1, "a".to_string(), true,
                                                       vec![
                                                           Tag::new("bad".to_string(), "hotzenplotz".to_string()),
                                                           Tag::new("good".to_string(), "kasper".to_string()),
                                                           Tag::new("more-bad".to_string(), "vader".to_string()),
                                                           Tag::new("more-good".to_string(), "grandma".to_string()),
                                                           Tag::new("badest".to_string(), "voldemort".to_string()),
                                                       ]));
        tag_filter.process_node(&mut node);
        assert_eq!(node.tags().len(), 2);
        assert_eq!(node.tags()[0].k(), &"good");
        assert_eq!(node.tags()[0].v(), &"kasper");
        assert_eq!(node.tags()[1].k(), &"more-good");
        assert_eq!(node.tags()[1].v(), &"grandma");
    }
    #[test]
    fn test_tag_filter_by_key__keep_matching() {
        let mut tag_filter = TagFilterByKey::new(
            PbfTypeSwitch{node:true, way:false, relation:false},
            Regex::new(".*good.*").unwrap(),
            FilterType::AcceptMatching,
            FinalHandler::new());

        let mut node = MutableNode::new(&mut Node::new(1, 1, Coordinate::new(1.0f64, 1.1f64), 1, 1, 1, "a".to_string(), true,
                                                       vec![
                                                           Tag::new("bad".to_string(), "hotzenplotz".to_string()),
                                                           Tag::new("good".to_string(), "kasper".to_string()),
                                                           Tag::new("more-bad".to_string(), "vader".to_string()),
                                                           Tag::new("more-good".to_string(), "grandma".to_string()),
                                                           Tag::new("badest".to_string(), "voldemort".to_string()),
                                                       ]));
        tag_filter.process_node(&mut node);
        assert_eq!(node.tags().len(), 2);
        assert_eq!(node.tags()[0].k(), &"good");
        assert_eq!(node.tags()[0].v(), &"kasper");
        assert_eq!(node.tags()[1].k(), &"more-good");
        assert_eq!(node.tags()[1].v(), &"grandma");
    }
    #[test]
    fn test_tag_filter_by_key__node_not_handled() {
        let mut tag_filter = TagFilterByKey::new(
            PbfTypeSwitch{node:false, way:true, relation:true},
            Regex::new(".*").unwrap(),
            FilterType::RemoveMatching,
            FinalHandler::new());

        let mut node = MutableNode::new(&mut Node::new(1, 1, Coordinate::new(1.0f64, 1.1f64), 1, 1, 1, "a".to_string(), true,
                                                       vec![
                                                           Tag::new("a".to_string(), "1".to_string()),
                                                           Tag::new("b".to_string(), "2".to_string()),
                                                           Tag::new("c".to_string(), "3".to_string()),
                                                       ]));
        tag_filter.process_node(&mut node);
        assert_eq!(node.tags().len(), 3);
    }
    #[test]
    fn test_tag_filter_by_key__node_handled() {
        let mut tag_filter = TagFilterByKey::new(
            PbfTypeSwitch{node:true, way:true, relation:true},
            Regex::new(".*").unwrap(),
            FilterType::RemoveMatching,
            FinalHandler::new());

        let mut node = MutableNode::new(&mut Node::new(1, 1, Coordinate::new(1.0f64, 1.1f64), 1, 1, 1, "a".to_string(), true,
                                                       vec![
                                                           Tag::new("a".to_string(), "1".to_string()),
                                                           Tag::new("b".to_string(), "2".to_string()),
                                                           Tag::new("c".to_string(), "3".to_string()),
                                                       ]));
        tag_filter.process_node(&mut node);
        assert_eq!(node.tags().len(), 0);
    }
}