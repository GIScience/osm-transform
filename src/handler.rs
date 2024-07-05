use osm_io::osm::model::node::Node;
use osm_io::osm::model::relation::Relation;
use osm_io::osm::model::way::Way;
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
    fn handle_node(&mut self, node: &Node) {//TODO why not node: &mut Node?

    }

    fn handle_node_next(&mut self, node: &Node) {
        self.handle_node(node);
        if let Some(next) = &mut self.get_next() {
            next.handle_node_next(node);
        }
    }
    fn handle_way(&mut self, way: &Way) {
        return;
    }
    fn handle_way_next(&mut self, way: &Way) {
        self.handle_way(way);
        if let Some(next) = &mut self.get_next() {
            next.handle_way_next(way);
        }
    }
    fn handle_relation(&mut self, relation: &Relation) {}
    fn handle_relation_next(&mut self, relation: &Relation) {
        self.handle_relation(relation);
        if let Some(next) = &mut self.get_next() {
            next.handle_relation_next(relation);
        }
    }

    fn get_next(&mut self) -> &mut Option<Box<dyn Handler>>;

    fn get_results(&mut self, res: &mut HandlerResult) {}

    fn get_results_next(&mut self, res: &mut HandlerResult) {
        self.get_results(res);
        if let Some(next) = &mut self.get_next() {
            next.get_results_next(res);
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
    fn handle_node(&mut self, node: &Node) {
        // dbg!(node);
    }

    fn get_next(&mut self) -> &mut Option<Box<dyn Handler>> {
        &mut self.next
    }

    fn get_results(&mut self, result: &mut HandlerResult) {
        // dbg!("FinalHandler get_results was called");
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
    fn handle_node(&mut self, node: &Node) {
        self.node_ids.push(node.id());
    }

    fn get_next(&mut self) -> &mut Option<Box<dyn Handler>> {
        return &mut self.next;
    }

    fn get_results(&mut self, res: &mut HandlerResult) {
        res.node_ids = self.node_ids.clone();//TODO remove, this is just an experiment
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
    fn handle_node(&mut self, node: &Node) {
        self.count += 1
    }

    fn get_next(&mut self) -> &mut Option<Box<dyn Handler>> {
        return &mut self.next;
    }
    fn get_results(&mut self, mut result: &mut HandlerResult) {
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

struct NodesFilterForTagValueMatch {
    pub tag: String,
    pub regex: Regex,
    pub filter_type: FilterType,
    pub next: Option<Box<dyn Handler>>,
}
impl NodesFilterForTagValueMatch {
    fn new(tag: String, regex: Regex, filter_type: FilterType, next: impl Handler + 'static) -> Self {
        Self {
            next: into_next(next),
            tag: tag,
            regex: regex,
            filter_type: filter_type
        }
    }
}
impl Handler for NodesFilterForTagValueMatch {
    fn handle_node_next(&mut self, node: &Node) {
        match self.filter_type {
            FilterType::AcceptMatching => {
                for tag in node.tags() {
                    if self.tag.eq(tag.k()) && self.regex.is_match(tag.v()) {
                        if let Some(next_handler) = self.get_next() {
                            next_handler.handle_node_next(node)
                        }
                    }
                }
            }
            FilterType::RemoveMatching => {
                let mut found_match = false;
                for tag in node.tags() {
                    if self.tag.eq(tag.k()) && self.regex.is_match(tag.v()) {
                        found_match = true;
                        break;
                    }
                }
                if !found_match {
                    if let Some(next_handler) = self.get_next() {
                        next_handler.handle_node_next(node)
                    }
                }
            }
        }
    }


    fn get_next(&mut self) -> &mut Option<Box<dyn Handler>> {
        return &mut self.next;
    }

    fn get_results(&mut self, result: &mut HandlerResult) {

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
    fn handle_node(&mut self, node: &Node) {
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
    }
    fn get_results(&mut self, res: &mut HandlerResult) {
        res.bbox_max_lon = self.max_lon;
        res.bbox_min_lon = self.min_lon;
        res.bbox_max_lat = self.max_lat;
        res.bbox_min_lat = self.min_lat;
    }
    fn get_next(&mut self) -> &mut Option<Box<dyn Handler>> {
        return &mut self.next;
    }
}

#[cfg(test)]
mod tests {
    use osm_io::osm::model::coordinate::Coordinate;
    use osm_io::osm::model::node::Node;
    use osm_io::osm::model::tag::Tag;
    use regex::Regex;
    use simple_logger::SimpleLogger;
    use crate::handler::{BboxCollector, CountType, FilterType, Handler, HandlerResult, NodesCounter, NodesFilterForTagValueMatch, FinalHandler, NodeIdCollector};

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
                NodesFilterForTagValueMatch::new(
                    existing_tag(),
                    Regex::new(".*p.*").unwrap(),
                    FilterType::AcceptMatching,
                    NodesFilterForTagValueMatch::new(
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
        handler.handle_node_next(&Node::new(1, 1, Coordinate::new(1.0f64, 1.1f64), 1, 1, 1, "a".to_string(), true, vec![Tag::new(existing_tag(), "kasper".to_string())]));
        handler.handle_node_next(&Node::new(2, 1, Coordinate::new(2.0f64, 1.2f64), 1, 1, 1, "a".to_string(), true, vec![Tag::new(existing_tag(), "seppl".to_string())]));
        handler.handle_node_next(&Node::new(3, 1, Coordinate::new(3.0f64, 1.3f64), 1, 1, 1, "a".to_string(), true, vec![Tag::new(existing_tag(), "hotzenplotz".to_string())]));
        handler.handle_node_next(&Node::new(4, 1, Coordinate::new(4.0f64, 1.4f64), 1, 1, 1, "a".to_string(), true, vec![Tag::new(existing_tag(), "großmutter".to_string())]));

        let mut result = HandlerResult::default();
        handler.get_results_next(&mut result);
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
}