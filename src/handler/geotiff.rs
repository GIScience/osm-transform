use std::collections::{BTreeMap, HashMap};
use std::error::Error;
use std::fs::File;
use std::io::BufReader;
use bit_vec::BitVec;
use georaster::geotiff::{GeoTiffReader, RasterValue};
use glob::glob;
use log::error;
use osm_io::osm::model::element::Element;
use osm_io::osm::model::node::Node;
use osm_io::osm::model::tag::Tag;
use proj4rs::Proj;
use rstar::{RTree, RTreeObject, AABB, Point, PointDistance, Envelope};
use crate::processor::{format_element_id, into_node_element, into_vec_node_element, into_vec_relation_element, into_vec_way_element, into_way_element, Processor};
use crate::srs::{DynamicSrsResolver, SrsResolver};

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
        RasterValue::NoData => {return None}
        RasterValue::U8(val) => {return Some(val.to_string())}
        RasterValue::U16(val) => {return Some(val.to_string())}
        RasterValue::U32(val) => {return Some(val.to_string())}
        RasterValue::U64(val) => {return Some(val.to_string())}
        RasterValue::F32(val) => {return Some(round_f32(val, 2.0))}
        RasterValue::F64(val) => {return Some(round_f64(val, 2.0))}
        RasterValue::I8(val) => {return Some(val.to_string())}
        RasterValue::I16(val) => {return Some(val.to_string())}
        RasterValue::I32(val) => {return Some(val.to_string())}
        RasterValue::I64(val) => {return Some(val.to_string())}
        RasterValue::Rgb8(_, _, _) => {return None}
        RasterValue::Rgba8(_, _, _, _) => {return None}
        RasterValue::Rgb16(_, _, _) => {return None}
        RasterValue::Rgba16(_, _, _, _) => {return None}
        _ => {return None}
    }
}

pub struct GeoTiffLoader {
    index: Box<dyn GeoTiffIndex>,
}
impl GeoTiffLoader {//todo rename to GeoTiffManager
    pub fn new() -> Self {
        Self {
            index: Box::new(RSGeoTiffIndex::new())
        }
    }
    pub fn load_geotiffs(&mut self, files_pattern: &str, srs_resolver: &DynamicSrsResolver) {//todo rename to load_and_index o.ä.
        match glob(files_pattern) {
            Ok(paths) => {
                for entry in paths {
                    match entry {
                        Ok(path) => {
                            if path.is_file() {
                                let geotiff = self.load_geotiff(path.to_str().unwrap(), srs_resolver);
                                match geotiff {
                                    Ok(geotiff) => {
                                        self.index.add_geotiff(geotiff, path.to_str().unwrap());
                                        log::debug!("Successfully indexed geotiff file '{:?}'", path);
                                    }
                                    Err(error) => {
                                        log::error!("{:?}", error)
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

    pub fn load_geotiff(&mut self, file_path: &str, srs_resolver: &DynamicSrsResolver) -> Result<GeoTiff, Box<dyn Error>> {
        let img_file = BufReader::new(File::open(file_path).expect("Could not open input file"));
        let mut geotiffreader = GeoTiffReader::open(img_file).expect("Could not read input file as tiff");

        let origin = geotiffreader.origin().unwrap();
        let pixel_size = geotiffreader.pixel_size().unwrap();
        let geo_params = geotiffreader.geo_params.clone().unwrap();
        let dimensions = geotiffreader.images().get(0).expect("no image in tiff").dimensions.unwrap();

        let geo_params : Vec<&str> = geo_params.split("|").collect();
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
    rtree: RTree<RSBoundingBox>
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
        let bbox = RSBoundingBox::new(geotiff_id.to_string(),
                                      [top_left_wgs84.0, bottom_right_wgs84.1],
                                      [bottom_right_wgs84.0, top_left_wgs84.1]
        );
        self.rtree.insert(bbox);
    }

    fn find_geotiff_id_for_wgs84_coord(&mut self, lon: f64, lat: f64) -> Vec<String> {
        let point_to_find = [lon, lat];
        // for b in self.rtree.iter(){
        //     dbg!(b);
        // }
        // dbg!(&point_to_find);
        self.rtree.locate_all_at_point(&point_to_find)
            .map(|bbox| bbox.id.clone())
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
}
impl RSBoundingBox {
    fn new(id: String, min: [f64; 2], max: [f64; 2]) -> Self {
        Self { id, min, max }
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
    geotiff_loader: GeoTiffLoader,
    nodes_for_geotiffs: HashMap<String,Vec<Node>>,//TODO change to references &Node ?
    node_counts_for_geotiffs: HashMap<String, usize>,
    srs_resolver: DynamicSrsResolver,
    max_buffer_len: usize,
    max_buffered_nodes: usize,
}
impl BufferingElevationEnricher {
    fn default() -> Self {
        Self::new(100000, 5_000_000)
    }
    fn new(max_buffer_len: usize, max_buffered_nodes: usize) -> Self {
        Self {
            geotiff_loader: GeoTiffLoader::new(),
            nodes_for_geotiffs: HashMap::new(),
            node_counts_for_geotiffs: HashMap::new(),
            srs_resolver: DynamicSrsResolver::new(),
            max_buffer_len,
            max_buffered_nodes,
        }
    }
    fn init(mut self, file_pattern: &str) -> BufferingElevationEnricher {
        self.geotiff_loader.load_geotiffs(file_pattern, &self.srs_resolver);
        self
    }

    /// only add node to (new) buffer, nothing else.
    /// handling and flushing is triggered by the trait methods
    fn buffer_node(&mut self, node: Node) -> (Option<String>, Option<Node>) {
        let geotiffs = self.geotiff_loader.find_geotiff_id_for_wgs84_coord(node.coordinate().lon(), node.coordinate().lat());
        if geotiffs.is_empty() {
            return (None, Some(node))
        }

        let geotiff_name = geotiffs.first().unwrap().to_string();
        // let mut buffer_option: Option<&Vec<Node>> = self.nodes_for_geotiffs.get(&geotiff_name);
        // let mut buffer_vec;
        self.nodes_for_geotiffs.entry(geotiff_name.clone()).or_insert_with(Vec::new).push(node);//todo remove clone
        // match buffer_option {
        //     None => {
        //         buffer_vec = vec![node];
        //         self.nodes_for_geotiffs.insert(geotiff_name.clone(), buffer_vec);
        //         self.node_counts_for_geotiffs.insert(geotiff_name.clone(), 1);
        //     }
        //     Some(buffer) => {
        //         buffer.push(node);
        //
        //         let current_count = self.node_counts_for_geotiffs.get(&geotiff_name).unwrap_or(&0usize);
        //         self.node_counts_for_geotiffs.insert(geotiff_name, current_count + 1);
        //     }
        // }
        //
        (Some(geotiff_name), None)
    }

    // load geotiff for this buffer and add elevation to all nodes in buffer,
    // return enriched nodes for downstream prociessing and empty the buffer.
    fn handle_and_flush_buffer(&mut self, buffer_name: String) -> Vec<Element> {
        let mut result_elements = vec![];
        // let geotiff = self.geotiff_loader.get_geotiff_by_id(buffer_name.as_str()).expect("could not load geotiff");
        let mut geotiff = self.geotiff_loader.load_geotiff(buffer_name.as_str(), &self.srs_resolver).expect("could not load geotiff");
        let buffer_vec = self.nodes_for_geotiffs.remove(&buffer_name).expect("buffer not found");
        for mut node in buffer_vec {
            let result = &geotiff.get_string_value_for_wgs_84(node.coordinate().lon(), node.coordinate().lat());
            match result {
                None => {
                    log::warn!("no elevation value for node#{}", node.id());
                }
                Some(value) => {
                    node.tags_mut().push(Tag::new("ele".to_string(), value.clone()));//todo avoid clone
                }
            }
            result_elements.push(into_node_element(node.clone()));//todo avoid clone
        }
        self.nodes_for_geotiffs.insert(buffer_name, vec![]);
        result_elements
    }

}
impl Processor for BufferingElevationEnricher {
    fn name(&self) -> String { "BufferingElevationEnricher".to_string() }
    fn handle_element(&mut self, element: Element) -> Vec<Element> {
        match element {
            Element::Node { node } => {
                let node_id = node.id();
                let (buffer_option, node_option) = self.buffer_node(node); //nur puffern, nichts tun
                match buffer_option {
                    None => {
                        log::warn!("node#{} was not buffered - no geotiff found for it?", &node_id);
                        match node_option {
                            None => {
                                log::error!("buffer_node returned no buffer name and also no node");
                                vec![]
                            },
                            Some(node) => {vec![into_node_element(node)]}
                        }
                    }
                    Some(buffer_name) => {
                        match self.nodes_for_geotiffs.get(&buffer_name) {
                            None => {
                                log::error!("the map nodes_for_geotiffs contained key {} but no value!", &node_id);
                                match node_option {
                                    None => vec![],
                                    Some(node) => {vec![into_node_element(node)]}
                                }
                            }
                            Some(buffer_vec) => {
                                if buffer_vec.len() > self.max_buffer_len {
                                    self.handle_and_flush_buffer(buffer_name) //elevation setzen, zrückgeben
                                } else {
                                    vec![]
                                }
                            }
                        }
                    }
                }
            }
            Element::Way { .. } => { vec![element] }
            Element::Relation { .. } => { vec![element] }
            Element::Sentinel => {vec![]}
        }
    }
    fn handle_and_flush_elements(&mut self, elements: Vec<Element>) -> Vec<Element> {
        let mut handeled = vec![];
        for element in elements {
            match element {
                Element::Node { node } => {
                    let node_id = node.id();
                    let (buffer_option, node_option) = self.buffer_node(node); //nur puffern, nichts tun
                    match buffer_option {
                        None => {
                            match node_option {
                                None => {
                                    log::error!("buffer_node returned no buffer name and also no node");
                                },
                                Some(node) => {
                                    log::warn!("node was not buffered for some reason, but will be sent to downstream processing");
                                    let _ = handeled.push(into_node_element(node.clone()));}//todo avoid clone?
                            }
                        }
                        Some(_) => {
                            //buffering is ok for now, no action needed her, node will be flushed a few lines below
                        }
                    }
                }
                Element::Way { .. } => {
                    //element sent to downstream processors
                    handeled.push(element)
                }
                Element::Relation { .. } => { handeled.push(element) }
                Element::Sentinel => {}
            }
        }
        let mut buffers: Vec<String> = self.nodes_for_geotiffs.iter()
            .map(|(k, v)| k.to_string())
            .collect();
        for buffer_name in buffers {
            handeled.append(&mut self.handle_and_flush_buffer(buffer_name));
        }
        handeled
    }
}







#[cfg(test)]
mod tests {
    use std::fs::File;
    use std::io::BufReader;
    use epsg::CRS;
    use georaster::geotiff::{GeoTiffReader, RasterValue};
    use proj4rs::Proj;
    use simple_logger::SimpleLogger;
    use crate::handler::geotiff::{GeoTiff, GeoTiffLoader, SrsResolver, transform, round_f32, round_f64, format_as_elevation_string, RSBoundingBox};
    use crate::srs::DynamicSrsResolver;

    fn create_geotiff_limburg() -> GeoTiff {
        let mut tiff_loader = GeoTiffLoader::new();
        let mut geotiff = tiff_loader.load_geotiff("test/limburg_an_der_lahn.tif", &DynamicSrsResolver::new()).expect("got error");
        geotiff
    }
    fn create_geotiff_srtm_38_03() -> GeoTiff {
        let mut tiff_loader = GeoTiffLoader::new();
        let mut geotiff = tiff_loader.load_geotiff("test/srtm_38_03.tif", &DynamicSrsResolver::new()).expect("got error");
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
        let mut geotiff_loader = GeoTiffLoader::new();
        geotiff_loader.load_geotiffs("test/*.tif", &srs_resolver);
        assert_eq!(1, geotiff_loader.index.get_geotiff_count());
        assert!(geotiff_loader.index.get_geotiff_by_id("test/limburg_an_der_lahn.tif").is_some());
        // assert!(geotiff_loader.index.get_geotiff_by_id("test/srtm_38_03.tif").is_some());
    }
    #[test]
    fn test_find_geotiff_id_for_wgs84_coord() {
        SimpleLogger::new().init();
        let srs_resolver = DynamicSrsResolver::new();
        let mut geotiff_loader = GeoTiffLoader::new();
        geotiff_loader.load_geotiffs("test/*.tif", &srs_resolver);
        assert_eq!(1, geotiff_loader.index.get_geotiff_count());
        let geotiffs = geotiff_loader.index.find_geotiff_id_for_wgs84_coord(8.06185930f64, 50.38536322f64);
        assert_eq!(1, geotiffs.len());
        assert_eq!("test/limburg_an_der_lahn.tif", geotiffs[0]);
    }
    #[test]
    #[ignore]
    fn test_find_geotiff_id_for_wgs84_coord_srtm_38_03() {
        SimpleLogger::new().init();
        let srs_resolver = DynamicSrsResolver::new();
        let mut geotiff_loader = GeoTiffLoader::new();
        geotiff_loader.load_geotiffs("/home/jh/data/elevation/srtm/*.tif", &srs_resolver);
        assert_eq!(7, geotiff_loader.index.get_geotiff_count());
        let geotiffs = geotiff_loader.index.find_geotiff_id_for_wgs84_coord(6.8633450, 45.8333145);
        assert_eq!(1, geotiffs.len());
        assert_eq!("/home/jh/data/elevation/srtm/srtm_38_03.tif", geotiffs[0]);
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
        let value = geotiff.get_value_for_wgs_84(8.06185930f64, 50.38536322f64 );
        dbg!(&value);
        assert_eq!(&value, &RasterValue::F32(121.21507));
    }

    #[test]
    #[ignore]
    fn geotiff_srtm_38_03_get_value_for_wgs_84() {
        let mut geotiff = create_geotiff_srtm_38_03();
        let value = geotiff.get_value_for_wgs_84(8.06185930f64, 50.38536322f64 );
        dbg!(&value);
        assert_eq!(&value, &RasterValue::I16(86));//TODO check why the value is so different for the same wgs coord as in the limburg test
    }

    #[test]
    fn experiment_from_user_string() {
        let mut srs_resolver = SrsResolver::new();
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
    fn proj_methods(value: &str, source: &str, srs_resolver: &SrsResolver) {
        println!("\n{value} ({source}):");
        dbg!(Proj::from_proj_string(value));
        dbg!(Proj::from_user_string(value));
        dbg!(CRS::try_from(value.to_string()));
        dbg!(epsg::references::get_name(value));
        dbg!(srs_resolver.get_epsg(value));
    }

    #[test]
    fn proj_from_epsg_code_from_user_string(){
        dbg!(Proj::from_epsg_code(4326).expect("not found"));
        dbg!(Proj::from_user_string("+proj=longlat +datum=WGS84 +no_defs +type=crs").expect("not found"));

        dbg!(Proj::from_epsg_code(25832).expect("not found"));
        dbg!(Proj::from_user_string("+proj=utm +zone=32 +ellps=GRS80 +towgs84=0,0,0,0,0,0,0 +units=m +no_defs +type=crs").expect("not found"));
    }

    #[test]
    fn transform_4326_to_4326(){
        let mut point_3d = transform(
            &Proj::from_epsg_code(4326).expect("not found"),
            &Proj::from_epsg_code(4326).expect("not found"),
            8.06185930f64, 50.38536322f64).expect("transformation error");
        assert_eq!(point_3d.0, 8.06185930f64);
        assert_eq!(point_3d.1, 50.38536322f64);
    }
    #[test]
    fn transform_25832_to_4326(){
        let mut point_3d = transform(
            &Proj::from_epsg_code(25832).expect("not found"),
            &Proj::from_epsg_code(4326).expect("not found"),
            433305.7043197789f64, 5581899.216447188f64).expect("transformation error");
        assert!(are_floats_close_7(point_3d.0, 8.06185930f64));
        assert!(are_floats_close_7(point_3d.1, 50.38536322f64));
    }
    #[test]
    fn transform_4326_to_25832(){
        let mut point_3d = transform(
            &Proj::from_epsg_code(4326).expect("not found"),
            &Proj::from_epsg_code(25832).expect("not found"),
            8.06185930f64, 50.38536322f64).expect("transformation error");
        dbg!(&point_3d);
        assert!(are_floats_close(point_3d.0, 433305.7043197789f64, 1e-2)); //todo is this still ok?
        assert!(are_floats_close(point_3d.1, 5581899.216447188f64, 1e-2)); //todo is this still ok?
    }

    #[test]
    fn transform_4326_to_25832_2(){
        let mut point_3d = transform(
            &Proj::from_epsg_code(4326).expect("not found"),
            &Proj::from_epsg_code(25832).expect("not found"),
            8.06f64, 50.28f64).expect("transformation error");
        dbg!(&point_3d);
        assert!(are_floats_close(point_3d.0, 433025.5633903637f64, 1e-4)); //todo is this still ok?
        assert!(are_floats_close(point_3d.1, 5570185.7364423815f64, 1e-3)); //todo is this still ok?
    }

    #[test]
    fn proj4rs_transform_5174_to_4326(){
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
    fn wgs_84_to_tiff_coord_4326(){
        let mut geotiff = create_fake_geotiff(Proj::from_epsg_code(4326).unwrap(), "test/limburg_an_der_lahn.tif");
        let tiff_coord = geotiff.wgs_84_to_tiff_coord(8.06185930f64, 50.38536322f64);
        assert_eq!(tiff_coord.0, 8.06185930f64);
        assert_eq!(tiff_coord.1, 50.38536322f64);
    }
    #[test]
    fn wgs_84_to_tiff_coord_25832(){
        let mut geotiff = create_fake_geotiff(Proj::from_epsg_code(25832).unwrap(), "test/limburg_an_der_lahn.tif");
        let tiff_coord = geotiff.wgs_84_to_tiff_coord(8.06185930f64, 50.38536322f64);
        assert!(are_floats_close(tiff_coord.0, 433305.7043197789f64, 1e-2));
        assert!(are_floats_close(tiff_coord.1, 5581899.216447188f64, 1e-2));
    }

    #[test]
    fn geotiff_limburg_to_pixel_coord_and_get_value_for_pixel_coord(){
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
    #[ignore]
    fn geotiff_srtm_38_03_to_pixel_coord_and_get_value_for_pixel_coord(){
        //Values and expected results picket from QGIS
        let mut geotiff = create_geotiff_srtm_38_03();
        check_tiff_to_pixel_coord_and_get_value_for_pixel_coord(&mut geotiff, (5.0, 50.0), (0u32, 0u32), RasterValue::I16(422));
        check_tiff_to_pixel_coord_and_get_value_for_pixel_coord(&mut geotiff, (9.9992467, 45.0007625), (5999u32, 5999u32), RasterValue::I16(36));
        check_tiff_to_pixel_coord_and_get_value_for_pixel_coord(&mut geotiff, (6.8633450, 45.8333145), (2236u32, 5000u32), RasterValue::I16(4771));
    }
    fn check_tiff_to_pixel_coord_and_get_value_for_pixel_coord(geotiff: &mut GeoTiff, tiff_coord: (f64, f64), expected_pixel_coord: (u32, u32), expected_value: RasterValue ){
        let pixel_coord = geotiff.tiff_to_pixel_coord(tiff_coord.0, tiff_coord.1);
        dbg!(&pixel_coord);
        let value = geotiff.get_value_for_pixel_coord(pixel_coord.0, pixel_coord.1);
        dbg!(&value);
        assert_eq!(pixel_coord, expected_pixel_coord);
        assert_eq!(value, expected_value);
    }

    #[test]
    fn elevation_lookup(){//passing test from osm-transform
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
}