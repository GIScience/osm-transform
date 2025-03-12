use log::{log_enabled, trace};
use log::Level::Trace;
use osm_io::osm::model::way::Way;

use crate::handler::{Handler, HandlerData};

pub(crate) struct SkipElevationNodeCollector {
    no_elevation_keys: Vec<String>,
}
impl SkipElevationNodeCollector {
    const DEFAULT_KEYS: [&'static str; 4] = ["bridge", "tunnel", "cutting", "indoor"];

    pub(crate) fn new(no_elevation_keys: Vec<&str>) -> Self {
        Self {
            no_elevation_keys: no_elevation_keys.iter().map(|&str| String::from(str)).collect()
        }
    }

    #[allow(dead_code)]
    pub(crate) fn default() -> Self {Self::new(Self::DEFAULT_KEYS.to_vec())
    }

    fn skip_elevation(&mut self, way: &Way) -> bool {
        way.tags().iter().any(|tag| self.no_elevation_keys.contains(&tag.k()) && tag.v() != "no")
    }

}
impl Handler for SkipElevationNodeCollector {
    fn name(&self) -> String {
        String::from("SkipElevationNodeCollector")
    }

    fn handle(&mut self, data: &mut HandlerData) {
        for way in & data.ways {
            if self.skip_elevation(way) {
                if log_enabled!(Trace) { trace!("skipping elevation for way {}", way.id()); }
                for &id in way.refs() {
                    if log_enabled!(Trace) { trace!("skipping elevation for node {}", id); }
                    data.no_elevation_node_ids.set(id as usize, true);
                }
            }
        }
    }
}


#[cfg(test)]
mod test {
    use crate::handler::{Handler, HandlerData};
    use crate::handler::skip_ele::SkipElevationNodeCollector;
    use crate::test::simple_way;

    const TUNNEL: (&str, &str) = ("tunnel", "avalanche_protector");
    const BRIDGE: (&str, &str) = ("bridge", "yes");
    const INDOOR: (&str, &str) = ("bridge", "yes");
    const CUTTING: (&str, &str) = ("bridge", "yes");
    const NO_BRIDGE: (&str, &str) = ("bridge", "no");
    const HIGHWAY: (&str, &str) = ("highway", "primary");

    #[test]
    fn test_skip_elevation() {
        let mut collector = SkipElevationNodeCollector::new(SkipElevationNodeCollector::DEFAULT_KEYS.to_vec());

        assert!( collector.skip_elevation(&simple_way(0, vec![1, 2, 3], vec![TUNNEL])) );
        assert!( collector.skip_elevation(&simple_way(0, vec![1, 2, 3], vec![BRIDGE])) );
        assert!( collector.skip_elevation(&simple_way(0, vec![1, 2, 3], vec![INDOOR])) );
        assert!( collector.skip_elevation(&simple_way(0, vec![1, 2, 3], vec![CUTTING])) );
        assert!( collector.skip_elevation(&simple_way(0, vec![1, 2, 3], vec![HIGHWAY, CUTTING])) );
        assert!(!collector.skip_elevation(&simple_way(0, vec![1, 2, 3], vec![NO_BRIDGE])) );
    }

    #[test]
    fn test_skip_elevation_node_collector_handle_result() {
        let mut data = HandlerData::default();
        let mut collector = SkipElevationNodeCollector::default();
        data.ways.push(simple_way(1, vec![1, 2, 3], vec![HIGHWAY]));
        data.ways.push(simple_way(2, vec![3, 4], vec![HIGHWAY, BRIDGE]));
        data.ways.push(simple_way(3, vec![4, 5, 6], vec![HIGHWAY, NO_BRIDGE]));
        data.ways.push(simple_way(4, vec![6, 7], vec![HIGHWAY, NO_BRIDGE, CUTTING]));
        data.ways.push(simple_way(5, vec![7, 8, 9], vec![HIGHWAY, TUNNEL]));

        collector.handle(&mut data);

        assert!(!data.no_elevation_node_ids.get(0).unwrap_or(false) );
        assert!(!data.no_elevation_node_ids.get(1).unwrap_or(false) );
        assert!(!data.no_elevation_node_ids.get(2).unwrap_or(false) );
        assert!( data.no_elevation_node_ids.get(3).unwrap_or(false) );
        assert!( data.no_elevation_node_ids.get(4).unwrap_or(false) );
        assert!(!data.no_elevation_node_ids.get(5).unwrap_or(false) );
        assert!( data.no_elevation_node_ids.get(6).unwrap_or(false) );
        assert!( data.no_elevation_node_ids.get(7).unwrap_or(false) );
        assert!( data.no_elevation_node_ids.get(8).unwrap_or(false) );
        assert!( data.no_elevation_node_ids.get(9).unwrap_or(false) );
    }

    #[ignore]//SkipElevationNodeCollector uses now the bitvec of HandlerData which is initialized with full size
    #[test]
    #[should_panic(expected = "index out of bounds")]
    fn node_id_collector_out_of_bounds(){
        let mut collector = SkipElevationNodeCollector::new(SkipElevationNodeCollector::DEFAULT_KEYS.to_vec());
        let mut data = HandlerData::default();
        data.ways.push(simple_way(1, vec![9, 10], vec![TUNNEL]));
        collector.handle(&mut HandlerData::default());
    }
}
