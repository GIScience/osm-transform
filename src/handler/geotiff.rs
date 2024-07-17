use std::fs::File;
use std::path::Path;
use tiff::decoder::{Decoder, DecodingResult};

pub struct GeoTiff{
    dimensions: (u32, u32),
    decoder: Decoder<File>
}
impl GeoTiff {
    pub fn read(&mut self) {
        match self.decoder.read_image() {

            Ok(DecodingResult::U8(data)) => {
                println ! ("Data length (U8): {}", data.len());
                // Process data...
            }
            Ok(DecodingResult::U16(data)) => {
                println ! ("Data length (U16): {}", data.len());
                // Process data...
            }
            Ok(DecodingResult::F32(data)) => {
                println ! ("Data length (F32): {}", data.len());
                // Process data...
            }
            _ => {
                println ! ("Unsupported data format");
            }
        }
    }

    pub(crate) fn get_value(&self, lat: f64, lon: f64) -> f64 {
        todo!()
    }
}

pub struct TiffLoader {}
impl TiffLoader {
    pub fn load(mut self, file_path: &str) -> Result<GeoTiff, Box<dyn std::error::Error>> {
        // Open the file
        let file = File::open(Path::new(file_path)) ?;

        // Create a decoder
        let mut decoder = Decoder::new(file) ?;

        // Print image dimensions
        let dimensions = decoder.dimensions() ?;
        println !("Dimensions: {:?}", dimensions);

        let geo_tiff = GeoTiff{dimensions: dimensions, decoder: decoder};
        Ok(geo_tiff)
    }
}

pub struct ElevationResolver {}
impl ElevationResolver {
    pub fn get_elevation(lat: f64, lon: f64) -> f64 {
        let mut tiff = find_tiff(lat, lon);
        return tiff.get_value(lat, lon);
    }
}

fn find_tiff(lat: f64, lon: f64) -> GeoTiff {
    todo!()
}



#[cfg(test)]
mod tests {
    use crate::handler::geotiff::TiffLoader;

    #[test]
    fn load() {
        let mut tiff_loader = TiffLoader{};
        let mut geotiff = tiff_loader.load("test/limburg_an_der_lahn.tif").expect("got error");
        geotiff.read();
        assert_eq!(geotiff.dimensions, (1016, 991))
    }


}