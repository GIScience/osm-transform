use osm_io::osm::model::node::Node;
use regex::Regex;

struct HandlerResult {
    pub count_all_nodes: i32,
    pub count_accepted_nodes: i32,
    pub bbox_max_lon: f64,
    pub bbox_min_lon: f64,
    pub bbox_max_lat: f64,
    pub bbox_min_lat: f64,


}

impl HandlerResult {
    pub fn new() -> Self {
        Self {
            count_all_nodes: 0,
            count_accepted_nodes: 0,
            bbox_min_lat: f64::MAX,
            bbox_min_lon: f64::MAX,
            bbox_max_lat: f64::MIN,
            bbox_max_lon: f64::MIN,
        }
    }
}


trait Handler {
    fn handle_node(&mut self, node: &Node);
    fn get_result(&mut self, result: HandlerResult) -> HandlerResult;
}

struct Terminator;

impl Terminator {
    pub fn new() -> Self {
        Self {}
    }
}

impl Handler for Terminator {
    fn handle_node(&mut self, node: &Node) {
        println!("terminator received {:?}", node)
    }
    fn get_result(&mut self, result: HandlerResult) -> HandlerResult {
        result
    }
}

#[derive(Debug)]
enum CountType {
    ALL,
    ACCEPTED,
}

struct NodesCounter {
    pub count: i32,
    pub count_type: CountType,
    pub next: Box<dyn Handler + 'static>,
}

impl Handler for NodesCounter {
    fn handle_node(&mut self, node: &Node) {
        self.count += 1;
        self.next.handle_node(node)
    }
    fn get_result(&mut self, mut result: HandlerResult) -> HandlerResult {
        match self.count_type {
            CountType::ALL => { result.count_all_nodes = self.count }
            CountType::ACCEPTED => { result.count_accepted_nodes = self.count }
        }
        return self.next.get_result(result);
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
    pub next: Box<dyn Handler + 'static>,
}

#[derive(Debug)]
enum HandlerDef {
    NodesFilterDef(NodesFilterDef),
    NodesCounterDef(NodesCounterDef),
    BBoxCollectorDef
}
#[derive(Debug)]
struct NodesFilterDef{tag: String, regex: Regex, filter_type: FilterType }
#[derive(Debug)]
struct NodesCounterDef{ count_type: CountType }

fn as_chain(mut defs: Vec<HandlerDef>) -> Box<dyn Handler> {
    defs.reverse();
    let mut previous: Box<dyn Handler> = Box::new(Terminator::new());
    for hander_def in defs {
        match hander_def {
            HandlerDef::NodesFilterDef(def) => {
                // println!("regex: {:?}, filter_type: {:?}", &def.regex, &def.filter_type);
                previous = Box::new(NodesFilterForTagValueMatch {tag: def.tag, filter_type: def.filter_type, regex: def.regex, next: previous });
            }
            HandlerDef::NodesCounterDef(def) => {
                // println!("count_type: {:?}", &def.count_type);
                previous = Box::new(NodesCounter{count: 0, count_type: def.count_type, next: previous });
            }
            HandlerDef::BBoxCollectorDef => {
                previous = Box::new(BboxCollector::new(previous));
            }
        }
    }
    previous
}
impl Handler for NodesFilterForTagValueMatch {
    fn handle_node(&mut self, node: &Node) {
        match self.filter_type {
            FilterType::AcceptMatching => {
                for tag in node.tags() {
                    if self.tag.eq(tag.k()) && self.regex.is_match(tag.v()) {
                        self.next.handle_node(node)
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
                    self.next.handle_node(node)
                }
            }
        }
    }
    fn get_result(&mut self, result: HandlerResult) -> HandlerResult {
        return self.next.get_result(result);
    }
}


struct BboxCollector {
    pub next: Box<dyn Handler>,
    pub min_lat: f64,
    pub min_lon: f64,
    pub max_lat: f64,
    pub max_lon: f64,
}
impl BboxCollector {
    pub fn new(next: Box<dyn Handler>) -> Self {
        Self {
            min_lat: f64::MAX,
            min_lon: f64::MAX,
            max_lat: f64::MIN,
            max_lon: f64::MIN,
            next: next,
        }
    }
}
impl Handler for BboxCollector {
    fn handle_node(&mut self, node: &Node) {
        if &self.min_lat == &0.0 {
            self.min_lat = node.coordinate().lat();
        }
        if &self.min_lon == &0.0 {
            self.min_lon = node.coordinate().lon()
        }

        if node.coordinate().lat() < self.min_lat {
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
        self.next.handle_node(node);
    }

    fn get_result(&mut self, mut result: HandlerResult) -> HandlerResult {
        result.bbox_min_lat = self.min_lat;
        result.bbox_min_lon = self.min_lon;
        result.bbox_max_lat = self.max_lat;
        result.bbox_max_lon = self.max_lon;
        return self.next.get_result(result);
    }
}



#[cfg(test)]
mod tests {
    use osm_io::osm::model::coordinate::Coordinate;
    use osm_io::osm::model::node::Node;
    use osm_io::osm::model::tag::Tag;
    use regex::Regex;
    use crate::handler::{as_chain, CountType, FilterType, Handler, HandlerDef, HandlerResult, NodesCounter, NodesCounterDef, NodesFilterForTagValueMatch, NodesFilterDef, Terminator, BboxCollector};

    const EXISTING_TAG: &str = "EXISTING_TAG";
    const MISSING_TAG: &str = "MISSING_TAG";
    fn existing_tag() -> String {"EXISTING_TAG".to_string()}
    fn missing_tag() -> String {"MISSING_TAG".to_string()}
    #[test]
    fn test_handle_nodes_with_as_chain() {
        let handlers = vec![
            HandlerDef::NodesCounterDef(NodesCounterDef{count_type: CountType::ALL}),
            HandlerDef::NodesFilterDef(NodesFilterDef{tag: existing_tag(), regex: Regex::new(".*p.*").unwrap(), filter_type: FilterType::AcceptMatching }),
            HandlerDef::NodesFilterDef(NodesFilterDef{tag: existing_tag(), regex: Regex::new(".*z.*").unwrap(), filter_type: FilterType::RemoveMatching }),
            HandlerDef::BBoxCollectorDef,
            HandlerDef::NodesCounterDef(NodesCounterDef{count_type: CountType::ACCEPTED}),
        ];
        let mut handler = as_chain(handlers);
        handle_test_nodes_and_verify_result(&mut *handler);
    }
    #[test]
    fn test_handle_nodes_with_manually_chanied_handlesr() {
        let mut handler = NodesCounter {
            count: 0,
            count_type: CountType::ALL,
            next: Box::new(NodesFilterForTagValueMatch {
                tag: existing_tag(),
                filter_type: FilterType::AcceptMatching,
                regex: Regex::new(".*p.*").unwrap(),
                next: Box::new(NodesFilterForTagValueMatch {
                    tag: existing_tag(),
                    filter_type: FilterType::RemoveMatching,
                    regex: Regex::new(".*z.*").unwrap(),
                    next: Box::new(NodesCounter {
                        count: 0,
                        count_type: CountType::ACCEPTED,
                        next: Box::new(BboxCollector::new(Box::new(Terminator::new())))
                    }),
                }),
            }),
        };
        handle_test_nodes_and_verify_result(&mut handler);
    }

    fn handle_test_nodes_and_verify_result(handler: &mut dyn Handler) {
        handler.handle_node(&Node::new(1, 1, Coordinate::new(1.0f64, 1.1f64), 1, 1, 1, "a".to_string(), true, vec![Tag::new(existing_tag(), "kasper".to_string())]));
        handler.handle_node(&Node::new(2, 1, Coordinate::new(2.0f64, 1.2f64), 1, 1, 1, "a".to_string(), true, vec![Tag::new(existing_tag(), "seppl".to_string())]));
        handler.handle_node(&Node::new(3, 1, Coordinate::new(3.0f64, 1.3f64), 1, 1, 1, "a".to_string(), true, vec![Tag::new(existing_tag(), "hotzenplotz".to_string())]));
        handler.handle_node(&Node::new(4, 1, Coordinate::new(4.0f64, 1.4f64), 1, 1, 1, "a".to_string(), true, vec![Tag::new(existing_tag(), "großmutter".to_string())]));
        let result = handler.get_result(HandlerResult::new());
        assert_eq!(result.count_all_nodes, 4);
        assert_eq!(result.count_accepted_nodes, 2);
        //BBox based on only filtered (accepted) nodes!
        assert_eq!(result.bbox_min_lat, 1.0f64);
        assert_eq!(result.bbox_min_lon, 1.1f64);
        assert_eq!(result.bbox_max_lat, 2.0f64);
        assert_eq!(result.bbox_max_lon, 1.2f64);
    }
}