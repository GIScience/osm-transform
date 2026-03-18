use std::cmp::min;
use std::f64::consts::PI;

use crate::handler::Handler;
use bytes::Bytes;
use libwebp::boxed::WebpBox;
use osm_io::osm::model::node::Node;
use pmtiles::aws_sdk_s3::Client; // Re-exported AWS SDK S3 client
use pmtiles::{AsyncPmTilesReader, HashMapCache, TileCoord};
use libwebp::WebPDecodeRGBA;
use proj4rs::Proj;
use proj4rs::transform::transform;

pub(crate) struct PMTilesElevationEnricher {}

impl PMTilesElevationEnricher {
    fn handle_node(&self) -> Result<i32, String> {
        Ok(0)
    }
}

impl Handler for PMTilesElevationEnricher {
    fn name(&self) -> String {
        String::from("PMTilesElevationEnricher")
    }

    fn handle(&mut self, _data: &mut super::HandlerData) {
        todo!()
    }

    fn flush(&mut self, data: &mut super::HandlerData) {
        todo!()
    }
}

async fn get_tile(client: Client, z: u8, x: u32, y: u32) -> Option<Bytes> {
    let cache = HashMapCache::default();
    let bucket = "https://s3.example.com".to_string();
    let key = "example.pmtiles".to_string();
    let reader =
        AsyncPmTilesReader::new_with_cached_client_bucket_and_path(cache, client, bucket, key)
            .await
            .unwrap();
    let coord = TileCoord::new(z, x, y).unwrap();
    reader.get_tile(coord).await.unwrap()
}

async fn get_tile_file(path: &str, z: u8, x: u32, y: u32) -> Option<Bytes> {
    let reader = AsyncPmTilesReader::new_with_path(path).await.unwrap();
    let coord = TileCoord::new(z, x, y).unwrap();
    // reader.get_tile(coord).await.unwrap()
    reader.get_tile(coord).await.unwrap()
}

// fn bytes_to_rgba(data: Bytes) -> (u32, u32, WebpBox<[u8]>) {
//     let (width, height, buf) = WebPDecodeRGBA(data.iter().as_slice()).unwrap();
//     // assert_eq!(buf.len(), width as usize * height as usize * 4);
//     // eprintln!("width = {}, height = {}", width, height);
//     // eprintln!(
//     //     "top-left pixel: rgba({}, {}, {}, {})",
//     //     buf[0],
//     //     buf[1],
//     //     buf[2],
//     //     buf[3] as f64 / 255.0,
//     // );
//     (width, height, buf)
// }

fn match_node_to_tile(node: Node, zoom: &u8) -> (f64, f64) {
    // TODO: Calculation seems incorrect
    let lon = node.coordinate().lon().to_radians();
    let lat = node.coordinate().lat().to_radians();
    let src = Proj::from_epsg_code(4326).unwrap();
    let dst = Proj::from_epsg_code(3857).expect("Projection not Implemented");

    let mut point = (lon, lat, 0.0);

    transform(&src, &dst, &mut point).unwrap();

    let lon_trans = point.0.to_degrees();
    let lat_trans = point.1;

    let x = 0.5 + lon_trans / 360.0;
    let y = 0.5 - lat_trans / (2.0 * PI);
        
    let n = (2 as u32).pow(zoom.clone() as u32) as f64;

    let x_tile = n * x;
    let y_tile = n * y;

    (x_tile, y_tile)
}

#[derive(Debug, Clone, Copy)]
pub struct BoundingBox {
    pub min_lon: f64,
    pub min_lat: f64,
    pub max_lon: f64,
    pub max_lat: f64,
}

pub fn get_pixel_coordinates(
    bbox: &BoundingBox,
    img_w: usize,
    img_h: usize,
    lon: f64,
    lat: f64,
) -> (usize, usize) {
    let pixel_width = (bbox.max_lon - bbox.min_lon) / img_w as f64;
    let pixel_height = (bbox.max_lat - bbox.min_lat) / img_h as f64;

    let mut pixel_x = ((lon - bbox.min_lon) / pixel_width) as usize;
    let mut pixel_y = ((bbox.max_lat - lat) / pixel_height) as usize;

    pixel_x = min(pixel_x, img_w - 1);
    pixel_y = min(pixel_y, img_h - 1);

    (pixel_x, pixel_y)
}

pub fn get_elevation_for_pixel(rgba: WebpBox<[u8]>, width: u32, height: u32, pixel_x: usize, pixel_y: usize) -> f64 {
    //TODO: only RGB channels are needed, use WebPDecodeRGB istead of WebPDecodeRGBA to avoid unnecessary alpha channel
    let index = convert_pixel_coordinate_to_pixel_index(pixel_x, pixel_y, width);

    let elevation = calculate_elevation_for_pixel(rgba, index);

    elevation
}

fn calculate_elevation_for_pixel(rgba: WebpBox<[u8]>, pixel_index: usize) -> f64 {
    let r = rgba[pixel_index];
    let g = rgba[pixel_index + 1];
    let b = rgba[pixel_index + 2];

    (r as f64 * 256.0 + g as f64 + b as f64 / 256.0) - 32768.0
}

pub fn convert_pixel_coordinate_to_pixel_index(pixel_x: usize, pixel_y: usize, width: u32) -> usize {
    (pixel_y * width as usize + pixel_x) * 4
}


#[cfg(test)]
mod test {
    use crate::handler::Handler;
    use crate::handler::{pmtiles::PMTilesElevationEnricher, HandlerData};
    use crate::handler::pmtiles::{convert_pixel_coordinate_to_pixel_index, get_tile_file, match_node_to_tile};
    use crate::utils::test_utils;

    #[test]
    fn test_pmtiles_elevation_enricher_handle_node() {
        let enricher = PMTilesElevationEnricher {};
        let result = enricher.handle_node();
        assert_eq!(0, result.unwrap())
    }

    #[ignore]
    #[test]
    fn test_match_node_to_tile() {
        let node = test_utils::simple_node_element_heidelberg_gaulskopfbrunnen(1, vec![]);

        let (x,y ) = match_node_to_tile(node, &16);

        assert_eq!(x as u64, 34354);
        assert_eq!(y as u64, 22396);
    }

    #[ignore]
    #[test]
    fn test_match_node_osm_example() {
        let node = test_utils::simple_node_element_osm_example(1, vec![]);

        let (x, y) = match_node_to_tile(node, &18);

        assert_eq!(y, 103246.410442);
        assert_eq!(x, 232798.930207);
    }

    #[ignore]
    #[test]
    fn test_pmtiles_elevation_enricher() {
        let mut handler = PMTilesElevationEnricher {};

        let mut data = HandlerData::default();
        data.nodes.push(test_utils::simple_node_element_limburg(
            1,
            vec![("ele", "10000")],
        ));
        data.nodes
            .push(test_utils::simple_node_element_limburg(2, vec![]));
        handler.flush(&mut data);
    }

    #[tokio::test]
    async fn test_access_file() {
        let path = "test/pmtiles/hd-6-33-21.pmtiles";
        let bytes = get_tile_file(path, 16, 34354, 22396 ).await.unwrap();
        assert!(!bytes.is_empty());
    }

    // #[tokio::test]
    // async fn test_convert_bytes() {
    //     let path = "test/pmtiles/hd-6-33-21.pmtiles";
    //     let bytes = get_tile_file(path, 16, 34354, 22396 ).await.unwrap();
    //     let (width, height, buf) = crate::handler::pmtiles::bytes_to_rgba(bytes);
    //     assert_eq!(width, 512);
    //     assert_eq!(height, 512);
    //     assert_eq!(buf.len(), 512 * 512 * 4);
    //     assert_eq!(buf[0], 129);
    //     assert_eq!(buf[1], 99);
    //     assert_eq!(buf[2], 16);
    //     assert_eq!(buf[3], 255);
    // }

    #[ignore]
    #[test]
    fn test_calculate_elevation_for_pixel() {
        todo!("missing test")
    }

    #[test]
    fn test_convert_pixel_coordinate_to_byte_array_index() {
        let mut index = convert_pixel_coordinate_to_pixel_index(0, 0, 10);
        assert_eq!(0, index);

        index = convert_pixel_coordinate_to_pixel_index(0, 1, 10);
        assert_eq!(40, index)
    }
}