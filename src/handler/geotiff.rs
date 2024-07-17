use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use georaster::geotiff::{GeoTiffReader, RasterValue};

pub struct GeoTiff{
    dimensions: (u32, u32),
    reader: GeoTiffReader<File>
}
impl GeoTiff {
    pub fn read(&mut self) {
        // self.reader.seek_to_image(0);
        // println!("tile_count: {:?}", self.reader.tile_count());
        // match self.reader.read_image() {
        //
        //     Ok(DecodingResult::U8(data)) => {
        //         println ! ("Data length (U8): {}", data.len());
        //         // Process data...
        //     }
        //     Ok(DecodingResult::U16(data)) => {
        //         println ! ("Data length (U16): {}", data.len());
        //         // Process data...
        //     }
        //     Ok(DecodingResult::F32(data)) => {
        //         println ! ("Data length (F32): {}", data.len());
        //         // Process data...
        //     }
        //     _ => {
        //         println ! ("Unsupported data format");
        //     }
        // }
    }

    pub(crate) fn get_value(&mut self, lat: f64, lon: f64) -> RasterValue {
        let xy = self.to_image_xy(lat, lon);
        self.reader.read_pixel(xy.0, xy.1)
    }

    pub(crate) fn to_image_xy(&mut self, lat: f64, lon: f64) -> (u32, u32) {
        (50, 50) //todo implement
    }
}


pub struct TiffLoader;
impl TiffLoader {
    pub fn load(mut self, file_path: &str) -> Result<GeoTiff, Box<dyn std::error::Error>> {

        let img_file = BufReader::new(File::open(file_path).expect("Could not open input file"));
        let mut tiff = GeoTiffReader::open(img_file).expect("Could not read input file as tiff");

        println!("Origin: {:?}", tiff.origin());
        println!("Pixel size: {:?}", tiff.pixel_size());
        println!("SRS: {:?}", tiff.geo_params);


        // Print image dimensions
        let dimensions = tiff.images()[0].dimensions() ?;
        println !("Dimensions: {:?}", dimensions);

        let geo_tiff = GeoTiff{dimensions: dimensions, reader: tiff};
        Ok(geo_tiff)
    }
}

pub struct ElevationResolver {
    geotiffs: Vec<GeoTiff>
}
impl ElevationResolver {
    pub fn get_elevation(&mut self, lat: f64, lon: f64) -> RasterValue {
        let mut tiff = self.find_tiff(lat, lon);
        tiff.get_value(lat, lon)
    }

    fn find_tiff(&mut self, lat: f64, lon: f64) -> &GeoTiff {
        &self.geotiffs[0]
    }
}

    #[cfg(test)]
mod tests {
    use crate::handler::geotiff::{ElevationResolver, TiffLoader};

    #[test]
    fn load() {
        let mut tiff_loader = TiffLoader{};

        let mut geotiff = tiff_loader.load("test/limburg_an_der_lahn.tif").expect("got error");
        let mut resolver = ElevationResolver{ geotiffs: vec![geotiff]};
        assert_eq!(geotiff.dimensions, (1016, 991))
    }


}