
pub mod io;
pub mod conf;

use osm_io::osm::model::node::Node;
use osm_io::osm::model::relation::Relation;
use osm_io::osm::model::way::Way;
use io::process_file;
use conf::Config;

pub fn run(config: Config) {
    dbg!(config);
    // process_file().expect("did not work");



    // read pbf, filter node ids belonging to ways -> node_ids, extract bbox, maxId (gefilterte)
    // reader(config, filter, bbox_extracotr, max_id_extractor);

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

trait Handler {
    fn handle_node(&self, node: Node) -> Node {
        self.get_downstream_handler().handle_node(node)
    }
    fn handle_way(&self, way: Way) -> Way {
        self.get_downstream_handler().handle_way(way)
    }
    fn handle_relation(&self, relation: Relation) -> Relation {
        self.get_downstream_handler().handle_relation(relation)
    }

    fn get_downstream_handler(&self) -> &Box<dyn Handler>;
}

struct Filter<'a> {
    downstream_handler: &'a Box<dyn Handler>,
    node_ids: Vec<i32>,
    way_ids: Vec<i32>,
}
impl Handler for Filter<'_> {
    fn handle_node(&self, node: Node) -> Node {
        todo!()
    }

    fn handle_way(&self, way: Way) -> Way {
        todo!()
    }

    fn handle_relation(&self, relation: Relation) -> Relation {
        todo!()
    }

    fn get_downstream_handler(&self) -> &Box<dyn Handler> {
        return self.downstream_handler;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hello() {

    }
}