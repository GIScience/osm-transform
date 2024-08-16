use osm_io::osm::model::coordinate::Coordinate;
use osm_io::osm::model::node::Node;
use rustc_hash::FxHashMap;

#[allow(dead_code)]
pub(crate) struct WaySplitter {
    pub next_node_id: i64,
    pub location_index: FxHashMap<i64, Coordinate>,
}

#[allow(dead_code)]
impl WaySplitter {
    pub(crate) fn new() -> Self {
        Self {
            next_node_id: -1,
            location_index: FxHashMap::default(),
        }
    }

    fn set_coordinates(&mut self, node: Node) {
        let coordinate = node.coordinate().clone();
        self.location_index.insert(node.id(), coordinate);
    }

    fn get_coordinates(&mut self, id: &i64) -> Option<&Coordinate> {
        self.location_index.get(id)
    }

    fn compute_intermediate_points(&mut self, a: Coordinate, b: Coordinate, resolution: (f64, f64)) -> Vec<Coordinate> {
        let dx = b.lat() - a.lat();
        let dy = b.lon() - a.lon();
        let n = f64::max(dx.abs() / resolution.0, dy.abs() / resolution.1).max(1.0).ceil();

        let sx = dx / n;
        let sy = dy / n;

        let mut x = a.lat();
        let mut y = a.lon();
        let n = (n - 1.0) as usize;
        let mut v = Vec::with_capacity(n);

        for i in 0..n {
            x += sx;
            y += sy;
            v.push(Coordinate::new(x, y));
        }

        v
    }
}

#[test]
fn test_node_indexing() {
    let mut way_splitter = WaySplitter::new();
    let coordinate1 = Coordinate::new(-1.0, 1.0);
    let coordinate2 = Coordinate::new(1.23, 0.0);

    way_splitter.set_coordinates(Node::new(1, 1, coordinate1.clone(), 1, 1, 1, 'a'.to_string(), true, Vec::new()));
    way_splitter.set_coordinates(Node::new(2, 1, coordinate2.clone(), 1, 1, 1, 'a'.to_string(), true, Vec::new()));

    assert_eq!(&coordinate1, way_splitter.get_coordinates(&1).unwrap());
    assert_eq!(&coordinate2, way_splitter.get_coordinates(&2).unwrap());
    assert!(way_splitter.get_coordinates(&3).is_none());
}

#[test]
fn test_intermediate_points() {
    let mut way_splitter = WaySplitter::new();
    let point_a = Coordinate::new(-1., 0.0);
    let point_b = Coordinate::new(0.0, 1.0);
    let point_c = Coordinate::new(1.0, 0.0);
    let point_d = Coordinate::new(0.0, -1.);

    // test no intermediate points necessary
    let points = way_splitter.compute_intermediate_points(point_a.clone(), point_b.clone(), (1.0, 1.0));
    assert_eq!(points.len(), 0);

    // test one intermediate point necessary
    let points = way_splitter.compute_intermediate_points(point_a.clone(), point_b.clone(), (0.5, 0.5));
    assert_eq!(points.len(), 1);
    assert_eq!(points[0], Coordinate::new(-0.5, 0.5));

    // test multiple intermediate points necessary
    let points = way_splitter.compute_intermediate_points(point_a.clone(), point_b.clone(), (0.3, 0.5));
    assert_eq!(points.len(), 3);
    assert_eq!(points[0], Coordinate::new(-0.75, 0.25));
    assert_eq!(points[1], Coordinate::new(-0.50, 0.50));
    assert_eq!(points[2], Coordinate::new(-0.25, 0.75));

    // test longitude lines perpendicular to the Equator
    let points = way_splitter.compute_intermediate_points(point_a.clone(), point_c.clone(), (1.0, 1.0));
    assert_eq!(points[0], Coordinate::new(0.0, 0.0));

    // test latitude lines parallel to the Equator
    let points = way_splitter.compute_intermediate_points(point_b.clone(), point_d.clone(), (2.0, 1.0));
    assert_eq!(points[0], Coordinate::new(0.0, 0.0));

    // test attempt to split zero length way
    let points = way_splitter.compute_intermediate_points(point_a.clone(), point_a.clone(), (1.0, 1.0));
    assert_eq!(points.len(), 0);
}
