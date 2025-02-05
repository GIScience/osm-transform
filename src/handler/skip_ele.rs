use bit_vec::BitVec;
use osm_io::osm::model::way::Way;

use crate::handler::{HandlerResult, HIGHEST_NODE_ID, Handler};

pub(crate) struct SkipElevationNodeCollector {
    referenced_node_ids: BitVec,
    no_elevation_keys: Vec<String>,
}
impl SkipElevationNodeCollector {
    const DEFAULT_KEYS: [&'static str; 4] = ["bridge", "tunnel", "cutting", "indoor"];

    pub(crate) fn new(nbits: usize, no_elevation_keys: Vec<&str>) -> Self {
        Self {
            referenced_node_ids: BitVec::from_elem(nbits, false),
            no_elevation_keys: no_elevation_keys.iter().map(|&str| String::from(str)).collect()
        }
    }

    pub(crate) fn default() -> Self {Self::new(HIGHEST_NODE_ID as usize, Self::DEFAULT_KEYS.to_vec())
    }

    fn skip_elevation(&mut self, way: &Way) -> bool {
        way.tags().iter().any(|tag| self.no_elevation_keys.contains(&tag.k()) && tag.v() != "no")
    }

    fn handle_way(&mut self, way: &Way) {
        if self.skip_elevation(way) {
            log::trace!("skipping elevation for way {}", way.id());
            for &id in way.refs() {
                log::trace!("skipping elevation for node {}", id);
                self.referenced_node_ids.set(id as usize, true);
            }
        }
    }
}
impl Handler for SkipElevationNodeCollector {
    fn name(&self) -> String {
        String::from("SkipElevationNodeCollector")
    }

    fn handle_ways(&mut self, elements: Vec<Way>) -> Vec<Way> {
        elements.iter().for_each(|way| self.handle_way(way));
        elements
    }

    fn add_result(&mut self, mut result: HandlerResult) -> HandlerResult {
        log::debug!("cloning node_ids of NoElevationNodeIdCollector with len={} into HandlerResult ", self.referenced_node_ids.len());
        result.skip_ele = self.referenced_node_ids.clone();
        result
    }
}


#[cfg(test)]
mod test {
    use crate::handler::skip_ele::SkipElevationNodeCollector;
    use crate::handler::tests::{simple_way};
    const TUNNEL: (&str, &str) = ("tunnel", "avalanche_protector");
    const BRIDGE: (&str, &str) = ("bridge", "yes");
    const INDOOR: (&str, &str) = ("bridge", "yes");
    const CUTTING: (&str, &str) = ("bridge", "yes");
    const NO_BRIDGE: (&str, &str) = ("bridge", "no");
    const HIGHWAY: (&str, &str) = ("highway", "primary");

    #[test]
    fn test_skip_elevation() {
        let mut collector = SkipElevationNodeCollector::new(0, SkipElevationNodeCollector::DEFAULT_KEYS.to_vec());

        assert!( collector.skip_elevation(&simple_way(0, vec![1, 2, 3], vec![TUNNEL])) );
        assert!( collector.skip_elevation(&simple_way(0, vec![1, 2, 3], vec![BRIDGE])) );
        assert!( collector.skip_elevation(&simple_way(0, vec![1, 2, 3], vec![INDOOR])) );
        assert!( collector.skip_elevation(&simple_way(0, vec![1, 2, 3], vec![CUTTING])) );
        assert!( collector.skip_elevation(&simple_way(0, vec![1, 2, 3], vec![HIGHWAY, CUTTING])) );
        assert!(!collector.skip_elevation(&simple_way(0, vec![1, 2, 3], vec![NO_BRIDGE])) );
    }

    #[test]
    fn test_handle_way() {
        let mut collector = SkipElevationNodeCollector::new(10, SkipElevationNodeCollector::DEFAULT_KEYS.to_vec());
        let ways = vec![
            simple_way(1, vec![1, 2, 3], vec![HIGHWAY]),
            simple_way(2, vec![3, 4], vec![HIGHWAY, BRIDGE]),
            simple_way(3, vec![4, 5, 6], vec![HIGHWAY, NO_BRIDGE]),
            simple_way(4, vec![6, 7], vec![HIGHWAY, NO_BRIDGE, CUTTING]),
            simple_way(5, vec![7, 8, 9], vec![HIGHWAY, TUNNEL])
        ];
        for way in ways {
            collector.handle_way(&way);
        }

        assert!(!collector.referenced_node_ids.get(0).unwrap_or(false) );
        assert!(!collector.referenced_node_ids.get(1).unwrap_or(false) );
        assert!(!collector.referenced_node_ids.get(2).unwrap_or(false) );
        assert!( collector.referenced_node_ids.get(3).unwrap_or(false) );
        assert!( collector.referenced_node_ids.get(4).unwrap_or(false) );
        assert!(!collector.referenced_node_ids.get(5).unwrap_or(false) );
        assert!( collector.referenced_node_ids.get(6).unwrap_or(false) );
        assert!( collector.referenced_node_ids.get(7).unwrap_or(false) );
        assert!( collector.referenced_node_ids.get(8).unwrap_or(false) );
        assert!( collector.referenced_node_ids.get(9).unwrap_or(false) );
    }

    #[test]
    #[should_panic(expected = "index out of bounds")]
    fn node_id_collector_out_of_bounds(){
        let mut collector = SkipElevationNodeCollector::new(10, SkipElevationNodeCollector::DEFAULT_KEYS.to_vec());
        collector.handle_way( &simple_way(1, vec![9, 10], vec![TUNNEL]) );
    }
}
