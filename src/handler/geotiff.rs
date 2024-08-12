use std::collections::{BTreeMap, HashMap};
use std::error::Error;
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;
use itertools::Itertools;
use bit_vec::BitVec;
use georaster::geotiff::{GeoTiffReader, RasterValue};
use glob::glob;
use log::error;
use osm_io::osm::model::element::Element;
use osm_io::osm::model::node::Node;
use osm_io::osm::model::tag::Tag;
use proj4rs::Proj;
use rstar::{AABB, Envelope, Point, PointDistance, RTree, RTreeObject};
use serde::__private::de::Content::F64;
use crate::handler::{format_element_id, into_node_element, into_vec_node_element, into_vec_relation_element, into_vec_way_element, into_way_element, Handler};
use crate::srs::DynamicSrsResolver;

pub struct GeoTiff {
    file_path: String,
    proj_wgs_84: Proj,
    proj_tiff: Proj,
    top_left_x: f64,
    top_left_y: f64,
    pixel_width: f64,
    pixel_height: f64,
    pixels_horizontal: u32,
    pixels_vertical: u32,
    // data_type: ,//todo
    // no_data_value: ,//todo
    geotiffreader: GeoTiffReader<BufReader<File>>,
}

impl GeoTiff {
    pub(crate) fn get_string_value_for_wgs_84(&mut self, lon: f64, lat: f64) -> Option<String> {
        let raster_value = self.get_value_for_wgs_84(lon, lat);
        format_as_elevation_string(raster_value)
    }

    pub(crate) fn get_value_for_wgs_84(&mut self, lon: f64, lat: f64) -> RasterValue {
        let tiff_coord = &self.wgs_84_to_tiff_coord(lon, lat);
        let pixel_coord = &self.tiff_to_pixel_coord(tiff_coord.0, tiff_coord.1);
        self.get_value_for_pixel_coord(pixel_coord.0, pixel_coord.1)
    }

    fn get_value_for_pixel_coord(&mut self, x: u32, y: u32) -> RasterValue {
        self.geotiffreader.read_pixel(x, y)
    }

    fn wgs_84_to_tiff_coord(&self, lon: f64, lat: f64) -> (f64, f64) {
        transform(&self.proj_wgs_84, &self.proj_tiff, lon, lat).expect("transformation error")
    }

    pub(crate) fn tiff_to_pixel_coord(&self, lon: f64, lat: f64) -> (u32, u32) {
        let pixel_x = ((lon - self.top_left_x) / self.pixel_width) as u32;
        let pixel_y = ((lat - self.top_left_y) / self.pixel_height) as u32;

        (pixel_x, pixel_y)
    }
    pub(crate) fn get_top_left_as_wgs84(&self) -> Result<(f64, f64), proj4rs::errors::Error> {
        transform(&self.proj_tiff, &self.proj_wgs_84, self.top_left_x, self.top_left_y)
    }
    pub(crate) fn get_bottom_right_as_wgs84(&self) -> Result<(f64, f64), proj4rs::errors::Error> {
        let lon_tiff = self.top_left_x + (self.pixel_width * self.pixels_horizontal as f64);
        let lat_tiff = self.top_left_y + (self.pixel_height * self.pixels_vertical as f64);
        transform(&self.proj_tiff, &self.proj_wgs_84, lon_tiff, lat_tiff)
    }
    pub(crate) fn get_pixel_degrees_horizontal_vertical(&self) -> (f64, f64){
        let top_left_wgs84 = self.get_top_left_as_wgs84().expect("Could not transform top left corner to WGS 84");
        let bottom_right_wgs84 = self.get_bottom_right_as_wgs84().expect("Could not transform bottom right corner to WGS 84");
        let pixel_size_horizontal = (bottom_right_wgs84.0 - top_left_wgs84.0) / self.pixels_horizontal as f64;
        let pixel_size_vertical = (top_left_wgs84.1 - bottom_right_wgs84.1) / self.pixels_vertical as f64;
        (pixel_size_horizontal, pixel_size_vertical)
    }
}
fn transform(src: &Proj, dst: &Proj, lon: f64, lat: f64) -> Result<(f64, f64), proj4rs::errors::Error> {
    let mut point = (lon, lat, 0.);

    if src.is_latlong() {
        point.0 = point.0.to_radians();
        point.1 = point.1.to_radians();
    }

    proj4rs::transform::transform(&src, &dst, &mut point)?;

    if dst.is_latlong() {
        point.0 = point.0.to_degrees();
        point.1 = point.1.to_degrees();
    }

    Ok((point.0, point.1))
}
fn round_f32(num: f32, dec_places: f32) -> String {
    let factor = 10.0_f32.powf(dec_places);
    let rounded_num: f32 = (num * factor).round() / factor;
    let dec_places = dec_places as usize;
    format!("{:.dec_places$}", rounded_num)
}
fn round_f64(num: f64, dec_places: f64) -> String {
    let factor = 10.0_f64.powf(dec_places);
    let rounded_num: f64 = (num * factor).round() / factor;
    let dec_places = dec_places as usize;
    format!("{:.dec_places$}", rounded_num)
}
fn format_as_elevation_string(raster_value: RasterValue) -> Option<String> {
    match raster_value {
        RasterValue::NoData => { return None }
        RasterValue::U8(val) => { return Some(val.to_string()) }
        RasterValue::U16(val) => { return Some(val.to_string()) }
        RasterValue::U32(val) => { return Some(val.to_string()) }
        RasterValue::U64(val) => { return Some(val.to_string()) }
        RasterValue::F32(val) => { return Some(round_f32(val, 2.0)) }
        RasterValue::F64(val) => { return Some(round_f64(val, 2.0)) }
        RasterValue::I8(val) => { return Some(val.to_string()) }
        RasterValue::I16(val) => { return Some(val.to_string()) }
        RasterValue::I32(val) => { return Some(val.to_string()) }
        RasterValue::I64(val) => { return Some(val.to_string()) }
        RasterValue::Rgb8(_, _, _) => { return None }
        RasterValue::Rgba8(_, _, _, _) => { return None }
        RasterValue::Rgb16(_, _, _) => { return None }
        RasterValue::Rgba16(_, _, _, _) => { return None }
        _ => { return None }
    }
}

pub struct GeoTiffManager {
    index: Box<dyn GeoTiffIndex>,
}
impl GeoTiffManager {
    pub fn new() -> Self {
        Self {
            index: Box::new(RSGeoTiffIndex::new())
        }
    }
    pub fn load_and_index_geotiffs(&mut self, files_pattern: &str, srs_resolver: &DynamicSrsResolver) {
        match glob(files_pattern) {
            Ok(paths) => {
                for entry in paths {
                    match entry {
                        Ok(path) => {
                            if path.is_file() {
                                let geotiff = self.load_geotiff(path.to_str().unwrap(), srs_resolver);
                                match geotiff {
                                    Ok(geotiff) => {
                                        self.index_geotiff(path, geotiff);
                                    }
                                    Err(error) => {
                                        log::error!("{:?}", error);
                                    }
                                }
                            }
                        }
                        Err(e) => error!("Error reading path: {:?}", e),
                    }
                }
            }
            Err(e) => error!("Failed to read glob pattern: {:?}", e),
        }
    }

    fn index_geotiff(&mut self, path: PathBuf, geotiff: GeoTiff) {
        self.index.add_geotiff(geotiff, path.to_str().unwrap());
        log::debug!("Successfully indexed geotiff file {:?}", path);
    }

    pub fn load_geotiff(&mut self, file_path: &str, srs_resolver: &DynamicSrsResolver) -> Result<GeoTiff, Box<dyn Error>> {
        log::debug!("Loading geotiff {}", file_path);
        let img_file = BufReader::new(File::open(file_path).expect("Could not open input file"));
        let mut geotiffreader = GeoTiffReader::open(img_file).expect("Could not read input file as tiff");

        let origin = geotiffreader.origin().unwrap();
        let pixel_size = geotiffreader.pixel_size().unwrap();
        let geo_params = geotiffreader.geo_params.clone().unwrap();
        let dimensions = geotiffreader.images().get(0).expect("no image in tiff").dimensions.unwrap();

        let geo_params: Vec<&str> = geo_params.split("|").collect();
        let proj_tiff = srs_resolver.get_epsg(geo_params[0].to_string()).expect("not found");

        let proj_wgs84 = Proj::from_epsg_code(4326).unwrap();
        let proj_tiff = Proj::from_epsg_code(proj_tiff as u16).unwrap(); //as u16 should not be necessary, SrsResolver should return u16

        let geo_tiff = GeoTiff {
            file_path: file_path.to_string(),
            proj_wgs_84: proj_wgs84,
            proj_tiff: proj_tiff,
            top_left_x: origin[0],
            top_left_y: origin[1],
            pixel_width: pixel_size[0],
            pixel_height: pixel_size[1],
            pixels_horizontal: dimensions.0,
            pixels_vertical: dimensions.1,
            geotiffreader: geotiffreader,
        };
        Ok(geo_tiff)
    }
    fn find_geotiff_id_for_wgs84_coord(&mut self, lon: f64, lat: f64) -> Vec<String> {
        self.index.find_geotiff_id_for_wgs84_coord(lon, lat)
    }
    fn get_geotiff_by_id(&mut self, geotiff_id: &str) -> Option<String> {
        self.index.get_geotiff_by_id(geotiff_id)
    }
    fn get_geotiff_count(&mut self) -> usize {
        self.index.get_geotiff_count()
    }
}


// Save wgs84 Bounding Boxes with geotiff_id that can be found efficiently by wgs84 coords.
// Clients of the index can identify which geotiff contains the coordinate of interest.
// It's their own responsibility to get the GeoTiffLoader by the id and get the geotiff value for the coordinate.
// The geotiff's file path is a candidate for a geotiff_id.
pub trait GeoTiffIndex {
    fn add_geotiff(&mut self, geotiff: GeoTiff, geotiff_id: &str);
    fn find_geotiff_id_for_wgs84_coord(&mut self, lon: f64, lat: f64) -> Vec<String>;
    fn get_geotiff_by_id(&mut self, geotiff_id: &str) -> Option<String>;
    fn get_geotiff_count(&mut self) -> usize;
}


pub struct RSGeoTiffIndex {
    pub(crate) rtree: RTree<RSBoundingBox>,
}
impl RSGeoTiffIndex {
    pub fn new() -> Self {
        Self {
            rtree: RTree::new()
        }
    }
}
impl GeoTiffIndex for RSGeoTiffIndex {
    fn add_geotiff(&mut self, geotiff: GeoTiff, geotiff_id: &str) {
        let top_left_wgs84 = geotiff.get_top_left_as_wgs84().expect("Could not transform top left corner to WGS 84");
        let bottom_right_wgs84 = geotiff.get_bottom_right_as_wgs84().expect("Could not transform bottom right corner to WGS 84");
        let pixel_size = geotiff.get_pixel_degrees_horizontal_vertical();
        let bbox = RSBoundingBox::new(geotiff_id.to_string(),
                                      [top_left_wgs84.0, bottom_right_wgs84.1],
                                      [bottom_right_wgs84.0, top_left_wgs84.1],
                                      f64::max(pixel_size.0, pixel_size.1) as f64
        );
        self.rtree.insert(bbox);
    }

    fn find_geotiff_id_for_wgs84_coord(&mut self, lon: f64, lat: f64) -> Vec<String> {
        let point_to_find = [lon, lat];
        // for b in self.rtree.iter(){
        //     dbg!(b);
        // }
        // dbg!(&point_to_find);
        // self.rtree.iter().for_each(|b| {
        //     println!("{:?}: {}", b.pixel_size, b.id);
        // });
        self.rtree.locate_all_at_point(&point_to_find)
            .map(|bbox| (bbox.pixel_size, bbox.id.clone()))
            .sorted_by(|a, b| a.0.partial_cmp(&b.0).unwrap())
            .map(|(_, id)| id)
            .collect()
    }

    fn get_geotiff_by_id(&mut self, geotiff_id: &str) -> Option<String> {
        let option = self.rtree.iter().find(|bbox| bbox.id.as_str() == geotiff_id);
        match option {
            None => None,
            Some(rsboundingbox) => Some(rsboundingbox.id.clone()),
        }
    }

    fn get_geotiff_count(&mut self) -> usize {
        self.rtree.size()
    }
}


#[derive(Clone, Debug)]
struct RSBoundingBox {
    id: String,
    min: [f64; 2],
    max: [f64; 2],
    pixel_size: f64,
}
impl RSBoundingBox {
    fn new(id: String, min: [f64; 2], max: [f64; 2], pixel_size: f64) -> Self {
        Self { id, min, max, pixel_size }
    }
}
impl RTreeObject for RSBoundingBox
{
    type Envelope = AABB<[f64; 2]>;

    fn envelope(&self) -> Self::Envelope {
        AABB::from_corners([self.min[0], self.min[1]], [self.max[0], self.max[1]])
    }
}
impl PointDistance for RSBoundingBox {
    fn distance_2(&self, point: &<Self::Envelope as Envelope>::Point) -> <<Self::Envelope as Envelope>::Point as Point>::Scalar {
        log::warn!("distance_2 was called - but is not implemented");
        todo!()
    }

    fn contains_point(&self, point: &<Self::Envelope as Envelope>::Point) -> bool {
        point[0] >= self.min[0] && point[0] <= self.max[0]
            && point[1] >= self.min[1] && point[1] <= self.max[1]
    }
}


pub(crate) struct BufferingElevationEnricher {
    geotiff_manager: GeoTiffManager,
    nodes_for_geotiffs: HashMap<String, Vec<Node>>,
    node_counts_for_geotiffs: HashMap<String, usize>,
    srs_resolver: DynamicSrsResolver,
    max_buffer_len: usize,
    max_buffered_nodes: usize,
    all_buffers_flushed: bool,
}
impl BufferingElevationEnricher {
    pub fn default() -> Self {
        Self::new(1_000_000, 5_000_000)
    }
    pub fn new(max_buffer_len: usize, max_buffered_nodes: usize) -> Self {
        Self {
            geotiff_manager: GeoTiffManager::new(),
            nodes_for_geotiffs: HashMap::new(),
            node_counts_for_geotiffs: HashMap::new(),
            srs_resolver: DynamicSrsResolver::new(),
            max_buffer_len,
            max_buffered_nodes,
            all_buffers_flushed: false,
        }
    }
    pub fn init(&mut self, file_pattern: &str) -> Result<(), Box<dyn Error>> {
        self.geotiff_manager.load_and_index_geotiffs(file_pattern, &self.srs_resolver);
        Ok(())
    }
    fn use_loader(mut self, geo_tiff_loader: GeoTiffManager) -> Self {
        self.geotiff_manager = geo_tiff_loader;
        self
    }

    /// Only add node to (new) buffer, nothing else.
    /// Handling and flushing is triggered by the trait methods.
    fn buffer_node(&mut self, node: Node) -> (Option<String>, Option<Node>) {
        let geotiffs = self.geotiff_manager.find_geotiff_id_for_wgs84_coord(node.coordinate().lon(), node.coordinate().lat());
        if geotiffs.is_empty() {
            return (None, Some(node));
        }

        let geotiff_name = geotiffs.first().unwrap().to_string();
        self.nodes_for_geotiffs.entry(geotiff_name.clone()).or_insert_with(Vec::new).push(node); //todo avoid clone String

        (Some(geotiff_name), None)
    }

    /// Load geotiff for this buffer and add elevation to all nodes in buffer,
    /// return nodes for downstream processing and empty the buffer.
    fn handle_and_flush_buffer(&mut self, buffer_name: String) -> Vec<Node> {
        let mut result = vec![];
        let buffer_vec = &mut self.nodes_for_geotiffs.remove(&buffer_name).expect("buffer not found");

        log::debug!("Handling and flushing buffer with {} buffered nodes for geotiff '{}'", buffer_vec.len(), buffer_name);
        let geotiff = &mut self.geotiff_manager.load_geotiff(buffer_name.as_str(), &self.srs_resolver).expect("could not load geotiff");
        buffer_vec.iter_mut().for_each(|node| self.add_elevation_tag(geotiff, node));

        result.append(buffer_vec);
        result
    }

    fn add_elevation_tag(&mut self, mut geotiff: &mut GeoTiff, mut node: &mut Node) {
        let result = geotiff.get_string_value_for_wgs_84(node.coordinate().lon(), node.coordinate().lat());
        match result {
            None => {
                if log::log_enabled!(log::Level::Trace) {
                    log::warn!("no elevation value for node#{}", node.id());
                }
            }
            Some(value) => {
                node.tags_mut().push(Tag::new("ele".to_string(), value));
            }
        }
    }

    fn handle_node(&mut self, node: Node) -> Vec<Node> {
        let node_id = node.id();
        let (buffer_option, node_option) = self.buffer_node(node); //nur puffern, nichts tun
        match buffer_option {
            None => {
                if log::log_enabled!(log::Level::Trace) {
                    log::warn!("node#{} was not buffered - no geotiff found for it?", &node_id);
                }
                match node_option {
                    None => {
                        log::error!("buffer_node returned no buffer name and also no node");
                        vec![]
                    }
                    Some(node) => { vec![node] }
                }
            }
            Some(buffer_name) => {
                match self.nodes_for_geotiffs.get(&buffer_name) {
                    None => {
                        log::error!("the map nodes_for_geotiffs contained key {} but no value!", &node_id);
                        match node_option {
                            None => vec![],
                            Some(node) => { vec![node] }
                        }
                    }
                    Some(buffer_vec) => {
                        if buffer_vec.len() >= self.max_buffer_len {
                            self.handle_and_flush_buffer(buffer_name) //elevation setzen, zrückgeben
                        } else {
                            vec![]
                        }
                    }
                }
            }
        }
    }
}

impl Handler for BufferingElevationEnricher {
    fn name(&self) -> String { "BufferingElevationEnricher".to_string() }

    fn handle_nodes(&mut self, nodes: Vec<Node>) -> Vec<Node> {
        let mut result = Vec::new();
        for node in nodes {
            result.extend(self.handle_node(node));
        }
        result
    }

    fn handle_and_flush_nodes(&mut self, mut elements: Vec<Node> ) -> Vec<Node> {
        log::debug!("{}: handle_and_flush_nodes called", self.name());
        let mut result = self.handle_nodes(elements);

        let mut buffers: Vec<String> = self.nodes_for_geotiffs.iter()
            .map(|(k, v)| k.to_string())
            .collect();
        for buffer_name in buffers {
            result.extend(self.handle_and_flush_buffer(buffer_name));
        }
        result
    }
}


#[cfg(test)]
mod tests {
    use std::fs::File;
    use std::io::BufReader;

    use epsg::CRS;
    use georaster::geotiff::{GeoTiffReader, RasterValue};
    use osm_io::osm::model::coordinate::Coordinate;
    use osm_io::osm::model::element::Element;
    use osm_io::osm::model::node::Node;
    use osm_io::osm::model::tag::Tag;
    use proj4rs::Proj;
    use simple_logger::SimpleLogger;

    use crate::handler::geotiff::{BufferingElevationEnricher, format_as_elevation_string, GeoTiff, GeoTiffManager, round_f32, round_f64, transform};
    use crate::handler::Handler;
    use crate::srs::DynamicSrsResolver;
    struct TestPoint {
        lon: f64,
        lat: f64,
    }
    impl TestPoint {
        fn new(lon: f64, lat: f64) -> Self {
            Self { lon, lat }
        }
        fn from_coordinate(coordinate: Coordinate) -> Self {
            Self { lon: coordinate.lon(), lat: coordinate.lat() }
        }
        fn as_coordinate(&self) -> Coordinate {
            Coordinate::new(self.lat, self.lon)
        }
        fn as_tuple_lon_lat(&self) -> (f64, f64) {
            (self.lon, self.lat)
        }
        fn lon(&self) -> f64 { self.lon }
        fn lat(&self) -> f64 { self.lat }
    }
    fn as_coord(tup: (f64, f64)) -> Coordinate {Coordinate::new(tup.1, tup.0)}
    fn as_tuple_lon_lat(coordinate: Coordinate) -> (f64, f64) { (coordinate.lon(), coordinate.lat()) }
    #[test]
    #[ignore]
    fn test_find_geotiff_id_for_wgs84_coord_srtm_ma_hd() {
        SimpleLogger::new().init();
        let srs_resolver = DynamicSrsResolver::new();
        let mut geotiff_loader = GeoTiffManager::new();
        geotiff_loader.load_and_index_geotiffs("test/srtm*.tif", &srs_resolver);
        assert_eq!(2, geotiff_loader.index.get_geotiff_count());
        geotiff_loader.load_and_index_geotiffs("test/region*.tif", &srs_resolver);
        assert_eq!(4, geotiff_loader.index.get_geotiff_count());
        geotiff_loader.load_and_index_geotiffs("test/*gmted*.tif", &srs_resolver);
        assert_eq!(6, geotiff_loader.index.get_geotiff_count());

        let test_point = wgs84_coordinate_hd_river();
        let geotiffs = geotiff_loader.index.find_geotiff_id_for_wgs84_coord(test_point.lon(), test_point.lat());
        dbg!(&geotiffs);
        assert_eq!(3, geotiffs.len());
        assert_eq!(geotiffs[0], "test/srtm_38_03.tif");
        assert_eq!(geotiffs[1], "test/region_heidelberg_mannheim.tif");
        assert_eq!(geotiffs[2], "test/30N000E_20101117_gmted_mea075.tif");

        let test_point = wgs84_coordinate_limburg_traffic_circle();
        let geotiffs = geotiff_loader.index.find_geotiff_id_for_wgs84_coord(test_point.lon(), test_point.lat());
        dbg!(&geotiffs);
        assert_eq!(3, geotiffs.len());
        assert_eq!(geotiffs[0], "test/region_limburg_an_der_lahn.tif");
        assert_eq!(geotiffs[1], "test/srtm_38_02.tif");
        assert_eq!(geotiffs[2], "test/50N000E_20101117_gmted_mea075.tif");

        let test_point = wgs84_coord_hd_mountain();
        let geotiffs = geotiff_loader.index.find_geotiff_id_for_wgs84_coord(test_point.lon(), test_point.lat());
        dbg!(&geotiffs);
        assert_eq!(3, geotiffs.len());
        assert_eq!(geotiffs[0], "test/srtm_38_03.tif");
        assert_eq!(geotiffs[1], "test/region_heidelberg_mannheim.tif");
        assert_eq!(geotiffs[2], "test/30N000E_20101117_gmted_mea075.tif");

        let test_point = wgs84_coordinate_limburg_vienna_house();
        let geotiffs = geotiff_loader.index.find_geotiff_id_for_wgs84_coord(test_point.lon(), test_point.lat());
        dbg!(&geotiffs);
        assert_eq!(3, geotiffs.len());
        assert_eq!(geotiffs[0], "test/region_limburg_an_der_lahn.tif");
        assert_eq!(geotiffs[1], "test/srtm_38_02.tif");
        assert_eq!(geotiffs[2], "test/50N000E_20101117_gmted_mea075.tif");

        let test_point = wgs84_coordinate_hamburg_elbphilharmonie();
        let geotiffs = geotiff_loader.index.find_geotiff_id_for_wgs84_coord(test_point.lon(), test_point.lat());
        dbg!(&geotiffs);
        assert_eq!(2, geotiffs.len());
        assert_eq!(geotiffs[0], "test/srtm_38_02.tif");
        assert_eq!(geotiffs[1], "test/50N000E_20101117_gmted_mea075.tif");
    }
    fn wgs84_coord_hd_mountain() -> TestPoint {TestPoint::new(8.726878, 49.397500)}
    fn wgs84_coordinate_hd_river() -> TestPoint {TestPoint::new(8.682461, 49.411029)}
    fn wgs84_coordinate_limburg_vienna_house() -> TestPoint {TestPoint::new(8.06, 50.39)}
    fn wgs84_coordinate_limburg_traffic_circle() -> TestPoint {TestPoint::new(8.06185930, 50.38536322)}

    fn wgs84_coordinate_hamburg_elbphilharmonie() -> TestPoint {TestPoint::new(9.984270930290224, 53.54137211789218)}
    fn create_geotiff_limburg() -> GeoTiff {
        let mut tiff_loader = GeoTiffManager::new();
        let mut geotiff = tiff_loader.load_geotiff("test/region_limburg_an_der_lahn.tif", &DynamicSrsResolver::new()).expect("got error");
        geotiff
    }
    fn create_geotiff_ma_hd() -> GeoTiff {
        let mut tiff_loader = GeoTiffManager::new();
        let mut geotiff = tiff_loader.load_geotiff("test/region_heidelberg_mannheim.tif", &DynamicSrsResolver::new()).expect("got error");
        geotiff
    }

    fn create_fake_geotiff(proj_tiff: Proj, file_path: &str) -> GeoTiff {
        let img_file = BufReader::new(File::open(file_path).expect("Could not open input file"));
        let mut geotiffreader = GeoTiffReader::open(img_file).expect("Could not read input file as tiff");
        GeoTiff {
            file_path: file_path.to_string(),
            proj_wgs_84: Proj::from_epsg_code(4326).unwrap(),
            proj_tiff: proj_tiff,
            top_left_x: 0.0,
            top_left_y: 0.0,
            pixel_width: 1.0,
            pixel_height: 1.0,
            pixels_horizontal: 10,
            pixels_vertical: 10,
            geotiffreader: geotiffreader,
        }
    }
    fn are_floats_close_7(a: f64, b: f64) -> bool {
        are_floats_close(a, b, 1e-7)
    }

    fn are_floats_close(a: f64, b: f64, epsilon: f64) -> bool {
        (a - b).abs() < epsilon
    }
    pub fn simple_node_element_limburg(id: i64, tags: Vec<(&str, &str)>) -> Node {
        let tags_obj = tags.iter().map(|(k, v)| Tag::new(k.to_string(), v.to_string())).collect();
        Node::new(id, 1, wgs84_coordinate_limburg_vienna_house().as_coordinate(), 1, 1, 1, "a".to_string(), true, tags_obj)
    }
    pub fn simple_node_element_hd_ma(id: i64, tags: Vec<(&str, &str)>) -> Node {
        let tags_obj = tags.iter().map(|(k, v)| Tag::new(k.to_string(), v.to_string())).collect();
        Node::new(id, 1, wgs84_coordinate_hd_river().as_coordinate(), 1, 1, 1, "a".to_string(), true, tags_obj)
    }
    fn validate_node_id(id: i64, element_option: &Option<&Element>) {
        if element_option.is_none() {
            panic!("expected some element")
        }
        match element_option.unwrap() {
            Element::Node { node } => {
                assert_eq!(id, node.id())
            }
            _ => panic!("expected a node element")
        }
    }
    fn validate_has_ids(elements: &Vec<Node>, ids: Vec<i64>) {
        let element_ids = elements.into_iter().map(|node| node.id()).collect::<Vec<_>>();
        assert!(element_ids.iter().all(|id| ids.contains(id)));
    }
    fn validate_all_have_elevation_tag(elements: &Vec<Node>) {
        elements.iter().for_each(|element| validate_has_elevation_tag(&element));
    }
    fn validate_has_elevation_tag(node: &Node) {
        assert!(node.tags().iter().any(|tag| tag.k().eq("ele") && !tag.v().is_empty()));
    }
    #[test]
    fn geotiff_limburg_load() {
        let geotiff = create_geotiff_limburg();
        assert_eq!(geotiff.pixels_vertical, 991);
        assert_eq!(geotiff.pixels_horizontal, 1016);
    }
    #[test]
    fn test_load_geotiffs() {
        SimpleLogger::new().init();
        let srs_resolver = DynamicSrsResolver::new();
        let mut geotiff_loader = GeoTiffManager::new();
        geotiff_loader.load_and_index_geotiffs("test/region*.tif", &srs_resolver);
        assert_eq!(2, geotiff_loader.index.get_geotiff_count());
        assert!(geotiff_loader.index.get_geotiff_by_id("test/region_limburg_an_der_lahn.tif").is_some());
    }
    #[test]
    fn test_find_geotiff_id_for_wgs84_coord() {
        SimpleLogger::new().init();
        let srs_resolver = DynamicSrsResolver::new();
        let mut geotiff_loader = GeoTiffManager::new();
        geotiff_loader.load_and_index_geotiffs("test/region*.tif", &srs_resolver);
        assert_eq!(2, geotiff_loader.index.get_geotiff_count());
        let test_point = wgs84_coordinate_limburg_vienna_house();
        let geotiffs = geotiff_loader.index.find_geotiff_id_for_wgs84_coord(test_point.lon(), test_point.lat());
        assert_eq!(1, geotiffs.len());
        assert_eq!("test/region_limburg_an_der_lahn.tif", geotiffs[0]);
    }

    #[test]
    fn test_find_geotiff_id_for_wgs84_coord_ma_hd() {
        SimpleLogger::new().init();
        let srs_resolver = DynamicSrsResolver::new();
        let mut geotiff_loader = GeoTiffManager::new();
        geotiff_loader.load_and_index_geotiffs("test/region*.tif", &srs_resolver);
        assert_eq!(2, geotiff_loader.index.get_geotiff_count());
        let test_point = wgs84_coordinate_hd_river();
        let geotiffs = geotiff_loader.index.find_geotiff_id_for_wgs84_coord(test_point.lon(), test_point.lat());
        assert_eq!(1, geotiffs.len());
        assert_eq!("test/region_heidelberg_mannheim.tif", geotiffs[0]);
    }
    #[test]
    #[ignore]
    fn test_find_geotiff_id_for_wgs84_coord_ma_hd_srtm() {
        SimpleLogger::new().init();
        let srs_resolver = DynamicSrsResolver::new();
        let mut geotiff_loader = GeoTiffManager::new();
        geotiff_loader.load_and_index_geotiffs("test/region*.tif", &srs_resolver);
        assert_eq!(2, geotiff_loader.index.get_geotiff_count());
        geotiff_loader.load_and_index_geotiffs("test/*gmted*.tif", &srs_resolver);
        assert_eq!(4, geotiff_loader.index.get_geotiff_count());
        geotiff_loader.load_and_index_geotiffs("test/srtm*.tif", &srs_resolver);
        assert_eq!(6, geotiff_loader.index.get_geotiff_count());

        let test_point = wgs84_coordinate_hd_river();
        let geotiffs = geotiff_loader.index.find_geotiff_id_for_wgs84_coord(test_point.lon(), test_point.lat());
        dbg!(&geotiffs);
        assert_eq!(3, geotiffs.len());
        assert_eq!(geotiffs[0], "test/srtm_38_03.tif");
        assert_eq!(geotiffs[1], "test/region_heidelberg_mannheim.tif");
        assert_eq!(geotiffs[2], "test/30N000E_20101117_gmted_mea075.tif");

        let test_point = wgs84_coordinate_limburg_traffic_circle();
        let geotiffs = geotiff_loader.index.find_geotiff_id_for_wgs84_coord(test_point.lon(), test_point.lat());
        dbg!(&geotiffs);
        assert_eq!(3, geotiffs.len());
        assert_eq!(geotiffs[0], "test/region_limburg_an_der_lahn.tif");
        assert_eq!(geotiffs[1], "test/srtm_38_02.tif");
        assert_eq!(geotiffs[2], "test/50N000E_20101117_gmted_mea075.tif");

        let test_point = wgs84_coord_hd_mountain();
        let geotiffs = geotiff_loader.index.find_geotiff_id_for_wgs84_coord(test_point.lon(), test_point.lat());
        dbg!(&geotiffs);
        assert_eq!(3, geotiffs.len());
        assert_eq!(geotiffs[0], "test/srtm_38_03.tif");
        assert_eq!(geotiffs[1], "test/region_heidelberg_mannheim.tif");
        assert_eq!(geotiffs[2], "test/30N000E_20101117_gmted_mea075.tif");

        let test_point = wgs84_coordinate_limburg_vienna_house();
        let geotiffs = geotiff_loader.index.find_geotiff_id_for_wgs84_coord(test_point.lon(), test_point.lat());
        dbg!(&geotiffs);
        assert_eq!(3, geotiffs.len());
        assert_eq!(geotiffs[0], "test/region_limburg_an_der_lahn.tif");
        assert_eq!(geotiffs[1], "test/srtm_38_02.tif");
        assert_eq!(geotiffs[2], "test/50N000E_20101117_gmted_mea075.tif");

        let test_point = wgs84_coordinate_hamburg_elbphilharmonie();
        let geotiffs = geotiff_loader.index.find_geotiff_id_for_wgs84_coord(test_point.lon(), test_point.lat());
        dbg!(&geotiffs);
        assert_eq!(2, geotiffs.len());
        assert_eq!(geotiffs[0], "test/srtm_38_02.tif");
        assert_eq!(geotiffs[1], "test/50N000E_20101117_gmted_mea075.tif");
    }

    #[test]
    fn geotiff_limburg_get_value_for_pixel_coord() {
        let mut geotiff = create_geotiff_limburg();

        let value = geotiff.get_value_for_pixel_coord(540u32, 978u32);
        dbg!(&value);
        assert_eq!(&value, &RasterValue::F32(190.338));

        let value = geotiff.get_value_for_pixel_coord(461u32, 731u32);
        dbg!(&value);
        assert_eq!(&value, &RasterValue::F32(163.98439));
    }

    #[test]
    fn geotiff_limburg_get_value_for_wgs_84() {
        let mut geotiff = create_geotiff_limburg();
        let test_point = wgs84_coordinate_limburg_traffic_circle();
        let value = geotiff.get_value_for_wgs_84(test_point.lon(), test_point.lat());
        dbg!(&value);
        assert_eq!(&value, &RasterValue::F32(121.21507));
    }

    #[test]
    fn geotiff_ma_hd_get_value_for_wgs_84() {
        let mut geotiff = create_geotiff_ma_hd();
        let test_point = wgs84_coordinate_hd_river();
        let value = geotiff.get_value_for_wgs_84(test_point.lon(), test_point.lat());
        dbg!(&value);
        assert_eq!(&value, &RasterValue::I16(107));
    }

    #[test]
    fn experiment_from_user_string() {
        let mut srs_resolver = DynamicSrsResolver::new();
        proj_methods("ETRS89 / UTM zone 32N|ETRS89|",
                     "geotiffreader.geo_params", &srs_resolver);
        proj_methods("ETRS89 / UTM zone 32N",
                     "geotiffreader.geo_params vereinfacht", &srs_resolver);
        proj_methods("ETRS89/UTM zone 32N",
                     "geotiffreader.geo_params vereinfacht", &srs_resolver);
        proj_methods("ETRS89 UTM zone 32N",
                     "geotiffreader.geo_params vereinfacht", &srs_resolver);
        proj_methods("ETRS89UTMzone32N",
                     "geotiffreader.geo_params vereinfacht", &srs_resolver);
        proj_methods("ETRS89",
                     "geotiffreader.geo_params vereinfacht", &srs_resolver);
        proj_methods("+proj=utm +zone=32 +ellps=GRS80 +towgs84=0,0,0,0,0,0,0 +units=m +no_defs +type=crs",
                     "proj4 von https://epsg.io/25832", &srs_resolver);
        proj_methods("proj4.defs(\"EPSG:25832\",\"+proj=utm +zone=32 +ellps=GRS80 +towgs84=0,0,0,0,0,0,0 +units=m +no_defs +type=crs\");",
                     "proj4js von https://epsg.io/25832", &srs_resolver);
    }
    fn proj_methods(value: &str, source: &str, srs_resolver: &DynamicSrsResolver) {
        println!("\n{value} ({source}):");
        dbg!(Proj::from_proj_string(value));
        dbg!(Proj::from_user_string(value));
        dbg!(CRS::try_from(value.to_string()));
        dbg!(epsg::references::get_name(value));
        dbg!(srs_resolver.get_epsg(value.to_string()));
    }

    #[test]
    fn proj_from_epsg_code_from_user_string() {
        dbg!(Proj::from_epsg_code(4326).expect("not found"));
        dbg!(Proj::from_user_string("+proj=longlat +datum=WGS84 +no_defs +type=crs").expect("not found"));

        dbg!(Proj::from_epsg_code(25832).expect("not found"));
        dbg!(Proj::from_user_string("+proj=utm +zone=32 +ellps=GRS80 +towgs84=0,0,0,0,0,0,0 +units=m +no_defs +type=crs").expect("not found"));
    }

    #[test]
    fn transform_4326_to_4326() {
        let mut point_3d = transform(
            &Proj::from_epsg_code(4326).expect("not found"),
            &Proj::from_epsg_code(4326).expect("not found"),
            wgs84_coordinate_limburg_traffic_circle().lon(), wgs84_coordinate_limburg_traffic_circle().lat()).expect("transformation error");
        assert_eq!(point_3d.0, 8.06185930f64);
        assert_eq!(point_3d.1, 50.38536322f64);
    }
    #[test]
    fn transform_25832_to_4326() {
        let mut point_3d = transform(
            &Proj::from_epsg_code(25832).expect("not found"),
            &Proj::from_epsg_code(4326).expect("not found"),
            433305.7043197789f64, 5581899.216447188f64).expect("transformation error");
        assert!(are_floats_close_7(point_3d.0, 8.06185930f64));
        assert!(are_floats_close_7(point_3d.1, 50.38536322f64));
    }
    #[test]
    fn transform_4326_to_25832() {
        let mut point_3d = transform(
            &Proj::from_epsg_code(4326).expect("not found"),
            &Proj::from_epsg_code(25832).expect("not found"),
            wgs84_coordinate_limburg_traffic_circle().lon(), wgs84_coordinate_limburg_traffic_circle().lat()).expect("transformation error");
        dbg!(&point_3d);
        assert!(are_floats_close(point_3d.0, 433305.7043197789f64, 1e-2)); // Is this precision still ok?
        assert!(are_floats_close(point_3d.1, 5581899.216447188f64, 1e-2)); // Is this precision still ok?
    }

    #[test]
    fn transform_4326_to_25832_2() {
        let mut point_3d = transform(
            &Proj::from_epsg_code(4326).expect("not found"),
            &Proj::from_epsg_code(25832).expect("not found"),
            8.06f64, 50.28f64).expect("transformation error");
        dbg!(&point_3d);
        assert!(are_floats_close(point_3d.0, 433025.5633903637f64, 1e-4)); // Is this precision still ok?
        assert!(are_floats_close(point_3d.1, 5570185.7364423815f64, 1e-3)); // Is this precision still ok?
    }

    #[test]
    fn proj4rs_transform_5174_to_4326() {
        //values taken from https://github.com/3liz/proj4rs/
        let mut point_3d = (198236.3200000003, 453407.8560000006, 0.0);
        dbg!(&point_3d);
        proj4rs::transform::transform(
            &Proj::from_epsg_code(5174).expect("not found"),
            &Proj::from_epsg_code(4326).expect("not found"),
            &mut point_3d);
        dbg!(&point_3d);
        point_3d.0 = point_3d.0.to_degrees();
        point_3d.1 = point_3d.1.to_degrees();
        dbg!(&point_3d);
        assert!(are_floats_close(point_3d.0, 126.98069676435814, 1e-2));
        assert!(are_floats_close(point_3d.1, 37.58308534678718, 1e-2));
    }
    #[test]
    fn wgs_84_to_tiff_coord_4326() {
        let mut geotiff = create_fake_geotiff(Proj::from_epsg_code(4326).unwrap(), "test/region_limburg_an_der_lahn.tif");
        let tiff_coord = geotiff.wgs_84_to_tiff_coord(wgs84_coordinate_limburg_traffic_circle().lon(), wgs84_coordinate_limburg_traffic_circle().lat());
        assert_eq!(tiff_coord.0, 8.06185930f64);
        assert_eq!(tiff_coord.1, 50.38536322f64);
    }
    #[test]
    fn wgs_84_to_tiff_coord_25832() {
        let mut geotiff = create_fake_geotiff(Proj::from_epsg_code(25832).unwrap(), "test/region_limburg_an_der_lahn.tif");
        let tiff_coord = geotiff.wgs_84_to_tiff_coord(wgs84_coordinate_limburg_traffic_circle().lon(), wgs84_coordinate_limburg_traffic_circle().lat());
        assert!(are_floats_close(tiff_coord.0, 433305.7043197789f64, 1e-2));
        assert!(are_floats_close(tiff_coord.1, 5581899.216447188f64, 1e-2));
    }

    #[test]
    fn geotiff_limburg_to_pixel_coord_and_get_value_for_pixel_coord() {
        //Values and expected results picket from QGIS
        let mut geotiff = create_geotiff_limburg();
        check_tiff_to_pixel_coord_and_get_value_for_pixel_coord(&mut geotiff, (435123.07f64, 5587878.78f64), (520u32, 73u32), RasterValue::F32(238.54259));
        check_tiff_to_pixel_coord_and_get_value_for_pixel_coord(&mut geotiff, (434919.63f64, 5588157.30f64), (500u32, 45u32), RasterValue::F32(210.00824));
        check_tiff_to_pixel_coord_and_get_value_for_pixel_coord(&mut geotiff, (430173.346f64, 5581806.030f64), (25u32, 680u32), RasterValue::F32(108.33898));
        check_tiff_to_pixel_coord_and_get_value_for_pixel_coord(&mut geotiff, (435705.34f64, 5579115.43f64), (578u32, 949u32), RasterValue::F32(186.76392));
        check_tiff_to_pixel_coord_and_get_value_for_pixel_coord(&mut geotiff, (439837f64, 5582052f64), (991u32, 655u32), RasterValue::F32(176.9392));
        check_tiff_to_pixel_coord_and_get_value_for_pixel_coord(&mut geotiff, (434743f64, 5582302f64), (482u32, 630u32), RasterValue::F32(109.42));
    }
    #[test]
    fn geotiff_ma_hd_to_pixel_coord_and_get_value_for_pixel_coord() {
        //Values and expected results picket from QGIS
        let mut geotiff = create_geotiff_ma_hd();
        check_tiff_to_pixel_coord_and_get_value_for_pixel_coord(&mut geotiff, wgs84_coord_hd_mountain().as_tuple_lon_lat(), (425u32, 342u32), RasterValue::I16(573));
        check_tiff_to_pixel_coord_and_get_value_for_pixel_coord(&mut geotiff, wgs84_coordinate_hd_river().as_tuple_lon_lat(), (372u32, 326u32), RasterValue::I16(107));
    }
    fn check_tiff_to_pixel_coord_and_get_value_for_pixel_coord(geotiff: &mut GeoTiff, tiff_coord: (f64, f64), expected_pixel_coord: (u32, u32), expected_value: RasterValue) {
        let pixel_coord = geotiff.tiff_to_pixel_coord(tiff_coord.0, tiff_coord.1);
        dbg!(&pixel_coord);
        let value = geotiff.get_value_for_pixel_coord(pixel_coord.0, pixel_coord.1);
        dbg!(&value);
        assert_eq!(pixel_coord, expected_pixel_coord);
        assert_eq!(value, expected_value);
    }

    #[test]
    fn elevation_lookup() { //passing test from osm-transform
        let mut geotiff = create_geotiff_limburg();
        let tiff_coord = geotiff.wgs_84_to_tiff_coord(8.0513629, 50.3876977);
        dbg!(&tiff_coord);
        let pixel_coord = geotiff.tiff_to_pixel_coord(tiff_coord.0, tiff_coord.1);
        dbg!(&pixel_coord);
        let value = geotiff.get_value_for_pixel_coord(pixel_coord.0, pixel_coord.1);
        dbg!(&value);
        assert_eq!(value, RasterValue::F32(163.81633));
    }

    #[test]
    fn test_round_f32() {
        assert_eq!(round_f32(123.555555f32, 0.0), "124");
        assert_eq!(round_f32(123.555555f32, 1.0), "123.6");
        assert_eq!(round_f32(123.555555f32, 2.0), "123.56");
        assert_eq!(round_f32(123.555555f32, 3.0), "123.556");
    }
    #[test]
    fn test_round_f64() {
        assert_eq!(round_f64(123.555555f64, 0.0), "124");
        assert_eq!(round_f64(123.555555f64, 1.0), "123.6");
        assert_eq!(round_f64(123.555555f64, 2.0), "123.56");
        assert_eq!(round_f64(123.555555f64, 3.0), "123.556");
    }
    #[test]
    fn test_format_as_elevation_string() {
        assert_eq!(format_as_elevation_string(RasterValue::NoData), None);
        assert_eq!(format_as_elevation_string(RasterValue::U8(123)), Some("123".to_string()));
        assert_eq!(format_as_elevation_string(RasterValue::U16(123)), Some("123".to_string()));
        assert_eq!(format_as_elevation_string(RasterValue::U32(123)), Some("123".to_string()));
        assert_eq!(format_as_elevation_string(RasterValue::U64(123)), Some("123".to_string()));
        assert_eq!(format_as_elevation_string(RasterValue::F32(1234.56789)), Some("1234.57".to_string()));
        assert_eq!(format_as_elevation_string(RasterValue::F32(-1234.56789)), Some("-1234.57".to_string()));
        assert_eq!(format_as_elevation_string(RasterValue::F64(1234.56789)), Some("1234.57".to_string()));
        assert_eq!(format_as_elevation_string(RasterValue::F64(-1234.56789)), Some("-1234.57".to_string()));
        assert_eq!(format_as_elevation_string(RasterValue::I8(123)), Some("123".to_string()));
        assert_eq!(format_as_elevation_string(RasterValue::I8(-123)), Some("-123".to_string()));
        assert_eq!(format_as_elevation_string(RasterValue::I16(123)), Some("123".to_string()));
        assert_eq!(format_as_elevation_string(RasterValue::I16(-123)), Some("-123".to_string()));
        assert_eq!(format_as_elevation_string(RasterValue::I32(123)), Some("123".to_string()));
        assert_eq!(format_as_elevation_string(RasterValue::I32(-123)), Some("-123".to_string()));
        assert_eq!(format_as_elevation_string(RasterValue::I64(123)), Some("123".to_string()));
        assert_eq!(format_as_elevation_string(RasterValue::I64(-123)), Some("-123".to_string()));
        assert_eq!(format_as_elevation_string(RasterValue::Rgb8(123, 255, 0)), None);
        assert_eq!(format_as_elevation_string(RasterValue::Rgba8(123, 255, 0, 3)), None);
        assert_eq!(format_as_elevation_string(RasterValue::Rgb16(1234, 1234, 1234)), None);
        assert_eq!(format_as_elevation_string(RasterValue::Rgba16(1234, 1234, 1234, 1234)), None);
    }

    #[test]
    fn buffering_elevation_enricher_test() {
        SimpleLogger::new().init();
        let mut handler = BufferingElevationEnricher::new(4, 5);
        handler.init("test/region*.tif");

        // The first elements should be buffered in the buffer for their tiff
        assert_eq!(0usize, handler.handle_node(simple_node_element_limburg(1, vec![])).len());
        assert_eq!(0usize, handler.handle_node(simple_node_element_hd_ma(20, vec![])).len());
        assert_eq!(0usize, handler.handle_node(simple_node_element_hd_ma(21, vec![])).len());
        assert_eq!(0usize, handler.handle_node(simple_node_element_limburg(2, vec![])).len());
        assert_eq!(0usize, handler.handle_node(simple_node_element_limburg(3, vec![])).len());

        // When receiving the max_buffer_len st element for tiff limburg, this buffer should be flushed
        // and all 4 elements should be returned.
        // But the 2 elements for tiff hd_ma should remain in their buffer.
        let probably_4_limburg_nodes = handler.handle_node(simple_node_element_limburg(4, vec![]));
        assert_eq!(4usize, probably_4_limburg_nodes.len());
        validate_has_ids(&probably_4_limburg_nodes, vec![1, 2, 3, 4]);
        validate_all_have_elevation_tag(&probably_4_limburg_nodes);

        // After the flush call all 2 elements from the hd buffer should be released
        let probably_2_ma_hd_nodes = handler.handle_and_flush_nodes(vec![]);
        assert_eq!(2usize, probably_2_ma_hd_nodes.len());
        validate_has_ids(&probably_2_ma_hd_nodes, vec![20, 21]);
        validate_all_have_elevation_tag(&probably_2_ma_hd_nodes);

        // Now one more element is add, it should be buffered
        assert_eq!(0usize, handler.handle_node(simple_node_element_limburg(10, vec![])).len());
        // and now the flush fn is called with two additional node elements.
        // The two buffered and the two transferred arguments are expected in the return vec:
        let probably_3_elements = handler.handle_and_flush_nodes(vec![
            simple_node_element_hd_ma(22, vec![]),
            simple_node_element_limburg(5, vec![]),
        ]);
        validate_has_ids(&probably_3_elements, vec![5, 10, 22]);
        validate_all_have_elevation_tag(&probably_3_elements);
    }
}