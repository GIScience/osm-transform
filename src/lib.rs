pub mod conf;
pub mod io;
pub mod handler;

use crate::io::process_with_handler;
use conf::Config;
use io::process_file;
use osm_io::osm::model::node::Node;
use osm_io::osm::model::relation::Relation;
use osm_io::osm::model::way::Way;

pub fn run(config: Config) {
    dbg!(config);
    // process_file().expect("did not work");

    // read pbf, filter node ids belonging to ways -> node_ids, extract bbox, maxId (gefilterte)
    // reader(config, filter, bbox_extracotr, max_id_extractor);

    // let mut bbox_collector = BboxCollector{next: None, min_lat: 0f64, min_lon: 0f64, max_lat: 0f64, max_lon: 0f64};
    // let mut filter = Filter{next: &bbox_collector, node_ids: Vec::new(), way_ids: Vec::new()};
    // process_with_handler(config, filter);

    // download geotiffs for bbox
    // geo_tiff_downloader(config, bbox_extractor);

    // read pbf, nodes: handle notes to keep
    //                      remove tags
    //                      if elevation: add ele tag
    //                      if interpolation & elevation: add node_id:coordinates
    //                      if country: add country tag
    //                      write node to node pbf file nodes1
    // reader(config, filter, remove_tags, elevation_handler, interpolation_handler, country_handler, output_handler)

    //            ways:
    //                    remove tags
    //                    if interpolation: interpolate: create new nodes an add to nodes1
    //                    write way to ways
    //             relations:
    //                  remove tags
    //                  write
    //  if interpolated : merge files
}

#[derive(Default)]
pub struct HandlerResult {
    pub bbox_min_lat: f64,
    pub bbox_min_lon: f64,
    pub bbox_max_lat: f64,
    pub bbox_max_lon: f64,
}

trait Handler {
    fn handle_node(&mut self, node: &Node) {
        if let Some(next) = &mut self.get_next() {
            next.handle_node(node);
        }
    }

    fn handle_node_next(&mut self, node: &Node) {
        if let Some(next) = &mut self.get_next() {
            next.handle_node(node);
        }
    }

    fn handle_way(&mut self, way: &Way) {
        if let Some(next) = &mut self.get_next() {
            next.handle_way(way);
        }
    }

    fn handle_way_next(&mut self, way: &Way) {
        if let Some(next) = &mut self.get_next() {
            next.handle_way(way);
        }
    }

    fn handle_relation(&mut self, relation: &Relation) {
        self.handle_relation_next(relation)
    }

    fn handle_relation_next(&mut self, relation: &Relation) {
        if let Some(next) = &mut self.get_next() {
            next.handle_relation(relation);
        }
    }

    fn get_next(&mut self) -> &mut Option<Box<dyn Handler>>;

    fn get_results(&mut self, res: &mut HandlerResult) {
        self.get_results_next(res);
    }

    fn get_results_next(&mut self, res: &mut HandlerResult) {
        if let Some(next) = &mut self.get_next() {
            next.get_results(res);
        }
    }
}

pub fn into_next(handler: impl Handler + Sized + 'static) -> Option<Box<dyn Handler>> {
    Some(Box::new(handler))
}

#[derive(Default)]
struct Filter {
    pub next: Option<Box<dyn Handler>>,
    pub node_ids: Vec<i64>,
    pub way_ids: Vec<i64>,
}

impl Filter {
    pub fn new(next: impl Handler + 'static) -> Self {
        Self {
            next: into_next(next),
            node_ids: vec![],
            way_ids: vec![],
        }
    }
}
impl Handler for Filter {
    fn handle_node(&mut self, node: &Node) {
        self.node_ids.push(node.id());
        if let Some(next) = &mut self.get_next() {
            next.handle_node(node);
        }
    }

    fn get_next(&mut self) -> &mut Option<Box<dyn Handler>> {
        return &mut self.next;
    }
}

struct BboxCollector {
    pub next: Option<Box<dyn Handler>>,
    pub min_lat: f64,
    pub min_lon: f64,
    pub max_lat: f64,
    pub max_lon: f64,
}
impl Default for BboxCollector {
    fn default() -> Self {
        Self {
            next: None,
            min_lat: f64::MAX,
            min_lon: f64::MAX,
            max_lat: f64::MIN,
            max_lon: f64::MIN,
        }
    }
}

impl BboxCollector {
    pub fn new(next: impl Handler + 'static) -> Self {
        Self {
            next: into_next(next),
            min_lat: 0.0,
            min_lon: 0.0,
            max_lat: 0.0,
            max_lon: 0.0,
        }
    }

    fn set_min_lat(&mut self, val: f64) {
        self.min_lat = val;
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
        self.handle_node_next(node);
    }

    fn get_next(&mut self) -> &mut Option<Box<dyn Handler>> {
        return &mut self.next;
    }

    fn get_results(&mut self, res: &mut HandlerResult) {
        res.bbox_max_lon = self.max_lon;
        res.bbox_min_lon = self.min_lon;
        res.bbox_max_lat = self.max_lat;
        res.bbox_min_lat = self.min_lat;
        self.get_results_next(res);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hello() {}
}
