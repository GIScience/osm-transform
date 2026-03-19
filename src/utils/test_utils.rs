use osm_io::osm::model::coordinate::Coordinate;
use osm_io::osm::model::node::Node;
use osm_io::osm::model::tag::Tag;
use osm_io::osm::model::way::Way;
use crate::handler::geotiff::GeoTiffManager;
use crate::handler::geotiff::GeoTiff;
use crate::handler::geotiff::LocationWithElevation;

impl LocationWithElevation {
    pub(crate) fn from_lon_lat_ele(lon: f64, lat: f64, ele: f64) -> Self {
        Self::new(lon, lat, ele)
    }

    pub(crate) fn from_lat_lon(lat: f64, lon: f64) -> Self {
        Self::new(lon, lat, 0.0)
    }

    pub(crate) fn from_lat_lon_ele(lat: f64, lon: f64, ele: f64) -> Self {
        Self::new(lon, lat, ele)
    }

    pub fn to_node(self: Self, id: i64) -> Node {
        self.to_node_with_tags(id, vec![])
    }

    pub fn to_node_with_tags(self: Self, id: i64, tags: Vec<(&str, &str)>) -> Node {
        let tags_obj = tags.iter().map(|(k, v)| Tag::new(k.to_string(), v.to_string())).collect();
        Node::new(id, 1, self.get_coordinate(), 1, 1, 1, "a".to_string(), true, tags_obj)
    }
}

pub(crate) fn wgs84_coord_hd_philosophers_way_start() -> LocationWithElevation  { LocationWithElevation::new(8.693313002586367, 49.41470412961422, 125.0)}

pub(crate) fn wgs84_coord_hd_philosophers_way_end() -> LocationWithElevation  { LocationWithElevation::new(8.707872033119203, 49.41732503427102, 200.0)}

pub(crate) fn wgs84_coord_hd_mountain() -> LocationWithElevation { LocationWithElevation::from_lon_lat(8.726878, 49.397500)}

pub(crate) fn wgs84_coordinate_hd_river() -> LocationWithElevation { LocationWithElevation::from_lon_lat(8.682461, 49.411029)}

pub(crate) fn wgs84_coordinate_limburg_vienna_house() -> LocationWithElevation { LocationWithElevation::from_lon_lat(8.06, 50.39)}

pub(crate) fn wgs84_coordinate_limburg_traffic_circle() -> LocationWithElevation { LocationWithElevation::from_lon_lat(8.06185930, 50.38536322)}

pub(crate) fn wgs84_coordinate_hamburg_elbphilharmonie() -> LocationWithElevation { LocationWithElevation::from_lon_lat(9.984270930290224, 53.54137211789218)}

pub(crate) fn loc_hd_gaulskopfbrunnen() -> LocationWithElevation { LocationWithElevation::from_lat_lon(49.394664, 8.716664)}

pub(crate) fn loc_hd_gaisberg_peak() -> LocationWithElevation { LocationWithElevation::from_lat_lon_ele(49.4035191, 8.7047078, 371.125 )}

pub(crate) fn loc_osm_example() -> LocationWithElevation { LocationWithElevation::from_lon_lat(35.6590699, 139.7006793)}

pub(crate) fn create_geotiff_limburg() -> GeoTiff {
    let mut tiff_loader = GeoTiffManager::new();
    let geotiff = tiff_loader.load_geotiff("test/region_limburg_an_der_lahn.tif").expect("got error");
    geotiff
}

pub(crate) fn create_geotiff_ma_hd() -> GeoTiff {
    let mut tiff_loader = GeoTiffManager::new();
    let geotiff = tiff_loader.load_geotiff("test/region_heidelberg_mannheim.tif").expect("got error");
    geotiff
}


pub(crate) fn are_floats_close_7(a: f64, b: f64) -> bool {
    are_floats_close(a, b, 1e-7)
}

pub(crate) fn are_floats_close(a: f64, b: f64, epsilon: f64) -> bool {
    (a - b).abs() < epsilon
}

pub fn simple_way_element(id: i64, node_refs: Vec<i64>, tags: Vec<(&str, &str)>) -> Way {
    let tags_obj = tags.iter().map(|(k, v)| Tag::new(k.to_string(), v.to_string())).collect();
    Way::new(id, 1, 1, 1, 1, "a".to_string(), true, node_refs, tags_obj)
}

pub fn simple_node_element_hd_ma(id: i64, tags: Vec<(&str, &str)>) -> Node {
    let tags_obj = tags.iter().map(|(k, v)| Tag::new(k.to_string(), v.to_string())).collect();
    Node::new(id, 1, wgs84_coordinate_hd_river().get_coordinate(), 1, 1, 1, "a".to_string(), true, tags_obj)
}

pub(crate) fn validate_has_ids(elements: &Vec<Node>, ids: Vec<i64>) {
    let element_ids = elements.into_iter().map(|node| node.id()).collect::<Vec<_>>();
    assert!(element_ids.iter().all(|id| ids.contains(id)));
}

pub(crate) fn validate_all_have_elevation_tag(elements: &Vec<Node>) {
    elements.iter().for_each(|element| validate_has_elevation_tag(&element));
}

pub(crate) fn validate_has_elevation_tag(node: &Node) {
    assert!(node.tags().iter().any(|tag| tag.k().eq("ele") && !tag.v().is_empty()));
}

pub fn simple_node_element_limburg(id: i64, tags: Vec<(&str, &str)>) -> Node {
        let tags_obj = tags.iter().map(|(k, v)| Tag::new(k.to_string(), v.to_string())).collect();
        Node::new(id, 1, wgs84_coordinate_limburg_vienna_house().get_coordinate(), 1, 1, 1, "a".to_string(), true, tags_obj)
    }

pub fn simple_node_element_heidelberg_gaulskopfbrunnen(id: i64, tags: Vec<(&str, &str)>) -> Node {
    let tags_obj = tags.iter().map(|(k, v)| Tag::new(k.to_string(), v.to_string())).collect();
    Node::new(id, 1, Coordinate::new(49.394664, 8.716664), 1, 1, 1, "a".to_string(), true, tags_obj)
}

pub fn simple_node_element_heidelberg_gaisberg_peak(id: i64, tags: Vec<(&str, &str)>) -> Node {
    let tags_obj = tags.iter().map(|(k, v)| Tag::new(k.to_string(), v.to_string())).collect();
    Node::new(id, 1, Coordinate::new(49.4035191, 8.7047078), 1, 1, 1, "a".to_string(), true, tags_obj)
}

pub fn simple_node_element_osm_example(id: i64, tags: Vec<(&str, &str)>) -> Node {
    let tags_obj = tags.iter().map(|(k, v)| Tag::new(k.to_string(), v.to_string())).collect();
    Node::new(id, 1, Coordinate::new(35.6590699, 139.7006793), 1, 1, 1, "a".to_string(), true, tags_obj)
}

pub fn simple_node_element(id: i64, coordinate: Coordinate, tags: Vec<(&str, &str)>) -> Node {
    let tags_obj = tags.iter().map(|(k, v)| Tag::new(k.to_string(), v.to_string())).collect();
    Node::new(id, 1, coordinate, 1, 1, 1, "a".to_string(), true, tags_obj)
}
