use std::collections::HashMap;
use std::error::Error;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use csv::ReaderBuilder;
use epsg::CRS;
use georaster::geotiff::{GeoTiffReader, RasterValue};
use osm_io::osm::model::node::Node;
use osm_io::osm::model::relation::Relation;
use osm_io::osm::model::way::Way;
use phf::phf_map;
use proj4rs::Proj;
use crate::handler::Handler;
use crate::srs::SrsResolver;

pub struct GeoTiff {
    srs: String,
    epsg: u16,
    top_left_x: f64,
    top_left_y: f64,
    pixel_width: f64,
    pixel_height: f64,
    pixels_horizontal: u32,
    pixels_vertical: u32,
    geotiffreader: GeoTiffReader<BufReader<File>>,
}

impl GeoTiff {
    pub(crate) fn get_value(&mut self, lat: f64, lon: f64) -> RasterValue {
        let xy = &self.to_image_xy(lat, lon);
        self.geotiffreader.read_pixel(xy.0, xy.1)
    }
    pub(crate) fn get_value_for_pixel_coord(&mut self, x: u32, y: u32) -> RasterValue {
        self.geotiffreader.read_pixel(x, y)
    }

    pub(crate) fn check(&self) {
        if (!self.srs.starts_with("WGS 84|")) {
            log::warn!("UNSUPPORTED SRS {}", self.srs)
        }
    }

    pub(crate) fn to_image_xy(&self, lat: f64, lon: f64) -> (u32, u32) {
        // (lat as u32, lon as u32) //todo implement transformation

        // Get the GeoTIFF's CRS (this example assumes it's EPSG:4326 for simplicity)
        // You will need to get the actual CRS from the metadata if it's different
        let tiff_crs = "EPSG:4326"; // Replace with actual CRS if different
        // let tiff_crs = self.reader.geo_params.unwrap(); // Replace with actual CRS if different

        // Create a Proj instance to transform coordinates
        // let wgs84_to_tiff_crs = Proj ::new_known_crs("EPSG:4326", tiff_crs, None)?;

        // Transform the WGS84 coordinates to the GeoTIFF's CRS
        // let (x, y) = wgs84_to_tiff_crs.convert((lon, lat))?;
        // println!("Transformed coordinates in GeoTIFF CRS: ({}, {})", x, y);

        // temporary workaround without Proj (proj dependency)
        // let (x, y) = (lat as u32, lon as u32);
        let (x, y) = (lon as u32, lat as u32);

        // attempt with proj4rs
        // let from = Proj::from_proj_string(concat!(
        let from_wgs_84 = Proj::from_epsg_code(4326).unwrap();
        // "+proj=longlat +ellps=WGS84",
        // " +datum=WGS84 +no_defs"
        // ))
        //     .unwrap();
        // let to = Proj::from_proj_string(concat!(
        let to = Proj::from_epsg_code(25832).unwrap();
        // "+proj=utm +zone=32 +ellps=GRS80 +towgs84=0,0,0,0,0,0,0 +units=m +no_defs +type=crs"
        // ))
        //     .unwrap();
        let mut point_3d = (lon, lat, 0.0);
        proj4rs::transform::transform(&from_wgs_84, &to, &mut point_3d).unwrap();

        // XXX Note that angular unit is radians, not degrees !
        point_3d.0 = point_3d.0.to_degrees();
        point_3d.1 = point_3d.1.to_degrees();

        let (x, y) = (point_3d.0, point_3d.1);


        let binding = self.geotiffreader.origin().unwrap();
        let top_left_x = binding.get(0).unwrap();
        let top_left_y = binding.get(1).unwrap();
        let binding = self.geotiffreader.pixel_size().unwrap();
        let pixel_width = binding.get(0).unwrap();
        let pixel_height = binding.get(1).unwrap();

        let pixel_x = ((lon - top_left_x) / pixel_width).round() as u32;
        let pixel_y = ((lat - top_left_y) / pixel_height).round() as u32;

        println!("Pixel coordinates: ({}, {})", pixel_x, pixel_y);

        (pixel_x, pixel_y)
    }
}

pub struct GeoTiffLoader{
}
impl GeoTiffLoader {
    pub fn load_geotiff(mut self, file_path: &str, srs_resolver: &SrsResolver) -> Result<GeoTiff, Box<dyn std::error::Error>> {
        let img_file = BufReader::new(File::open(file_path).expect("Could not open input file"));
        let mut geotiffreader = GeoTiffReader::open(img_file).expect("Could not read input file as tiff");

        let origin = geotiffreader.origin().unwrap();
        let pixel_size = geotiffreader.pixel_size().unwrap();
        let geo_params = geotiffreader.geo_params.clone().unwrap(); //todo use this to derive epsg
        let dimensions = geotiffreader.images().get(0).expect("no image in tiff").dimensions.unwrap();

        println!("Origin: {:?}", origin);
        println!("Pixel size: {:?}", pixel_size);
        println!("SRS: {:?}", geo_params);
        println!("Dimensions: {:?}", dimensions);

        let geo_tiff = GeoTiff {
            srs: geo_params,
            epsg: 4326, // this should be checked
            top_left_x: origin[0],
            top_left_y: origin[1],
            pixel_width: pixel_size[0],
            pixel_height: pixel_size[1],
            pixels_horizontal: dimensions.0,
            pixels_vertical: dimensions.1,
            geotiffreader: geotiffreader,
        };
        geo_tiff.check();
        Ok(geo_tiff)
    }
}

pub struct ElevationResolver {
    geotiffs: Vec<GeoTiff>,
}
impl ElevationResolver {
    pub fn add_geotiff(&mut self, geotiff: GeoTiff) {
        //todo update index (e.g. rtree)
        self.geotiffs.push(geotiff)
    }
    pub fn find_geotiff(&mut self, lat: f64, lon: f64) -> &mut GeoTiff {
        //todo find the correct tiff e.g. with the help of an rtree
        &mut self.geotiffs[0] //this is a first workaround
    }
    pub fn get_elevation(&mut self, lat: f64, lon: f64) -> RasterValue {
        let tiff: &mut GeoTiff = self.find_geotiff(lat, lon);
        tiff.get_value(lat, lon)
    }
}


#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::fs::File;
    use std::path::Path;
    use epsg::CRS;
    use georaster::geotiff::{GeoTiffReader, RasterValue};
    use proj4rs::Proj;
    // use proj::Proj;
    use crate::handler::geotiff::{ElevationResolver, GeoTiffLoader, SrsResolver};

    #[test]
    fn srs_resolver() {
        let srs_resolver = SrsResolver::new();
        assert_eq!(4326, srs_resolver.get_epsg("WGS 84").expect("not found"))
    }
    #[test]
    fn load() {
        let srs_resolver = SrsResolver::new();
        let mut tiff_loader = GeoTiffLoader {};
        let geotiff = tiff_loader.load_geotiff("test/limburg_an_der_lahn.tif", &srs_resolver).expect("got error");
        let mut resolver = ElevationResolver { geotiffs: vec![] };
        resolver.add_geotiff(geotiff);
        assert_eq!(resolver.find_geotiff(50f64, 50f64).pixels_vertical, 991);
        assert_eq!(resolver.find_geotiff(50f64, 50f64).pixels_horizontal, 1016);
    }

    #[test]
    fn geotiff_get_value_for_pixel_coord() {
        let srs_resolver = SrsResolver::new();
        let mut tiff_loader = GeoTiffLoader {};
        let mut geotiff = tiff_loader.load_geotiff("test/limburg_an_der_lahn.tif", &srs_resolver).expect("got error");

        let value = geotiff.get_value_for_pixel_coord(540u32, 978u32);
        dbg!(&value);
        assert_eq!(&value, &RasterValue::F32(163.98439));

        let value = geotiff.get_value_for_pixel_coord(461u32, 731u32);
        dbg!(&value);
        assert_eq!(&value, &RasterValue::F32(190.338));
    }

    #[test]
    fn geotiff_get_value() {
        let mut tiff_loader = GeoTiffLoader {};
        let mut geotiff = tiff_loader.load_geotiff("test/limburg_an_der_lahn.tif", &SrsResolver::new()).expect("got error");

        let value = geotiff.get_value(50.39f64, 8.06f64);
        dbg!(&value);
        assert_eq!(&value, &RasterValue::F32(163.98439));
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
        // dbg!(get_crs(value));
        dbg!(CRS::try_from(value.to_string()));
        dbg!(epsg::references::get_name(value));
        dbg!(srs_resolver.get_epsg(value));
    }

}