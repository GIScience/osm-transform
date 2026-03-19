use std::cmp::min;
use std::f64::consts::PI;

use crate::handler::Handler;
use bytes::Bytes;
use libwebp::boxed::WebpBox;
use libwebp::WebPDecodeRGBA;
use osm_io::osm::model::node::Node;
use pmtiles::aws_sdk_s3::Client; // Re-exported AWS SDK S3 client
use pmtiles::{AsyncPmTilesReader, HashMapCache, TileCoord};

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
    reader.get_tile(coord).await.unwrap()
}

fn bytes_to_rgba(data: Bytes) -> (u32, u32, WebpBox<[u8]>) {
    WebPDecodeRGBA(data.iter().as_slice()).unwrap()
}

fn match_node_to_tile(node: Node, zoom: &u8) -> (f64, f64) {
    // see https://wiki.openstreetmap.org/wiki/Slippy_map_tilenames
    let n = (2 as u32).pow(zoom.clone() as u32) as f64;
    let x_tile = n * (0.5 + node.coordinate().lon() / 360.0);
    let y_tile = n * (0.5 - node.coordinate().lat().to_radians().tan().asinh() / (2.0 * PI));
    (x_tile, y_tile)
}

pub fn get_elevation_for_pixel(rgba: WebpBox<[u8]>, width: u32, height: u32, pixel_x: f64, pixel_y: f64) -> f64 {

    //TODO: only RGB channels are needed, use WebPDecodeRGB istead of WebPDecodeRGBA to avoid unnecessary alpha channel
    let index = convert_pixel_coordinate_to_pixel_index(pixel_x, pixel_y, width, height);

    let elevation = calculate_elevation_for_pixel(rgba, index);

    elevation
}

fn calculate_elevation_for_pixel(rgba: WebpBox<[u8]>, pixel_index: usize) -> f64 {
    let r = rgba[pixel_index];
    let g = rgba[pixel_index + 1];
    let b = rgba[pixel_index + 2];

    (r as f64 * 256.0 + g as f64 + b as f64 / 256.0) - 32768.0
}

pub fn convert_pixel_coordinate_to_pixel_index(pixel_x: f64, pixel_y: f64, width: u32, height: u32) -> usize {
    let pixel_x_u = (pixel_x * width as f64) as usize;
    let pixel_y_u = (pixel_y * height as f64) as usize;
    (pixel_y_u * width as usize + pixel_x_u) * 4
}


#[cfg(test)]
mod test {
    use crate::handler::Handler;
    use crate::handler::{pmtiles::PMTilesElevationEnricher, HandlerData};
    use crate::handler::pmtiles::{bytes_to_rgba, convert_pixel_coordinate_to_pixel_index, get_elevation_for_pixel, get_tile_file, match_node_to_tile};
    use crate::utils::test_utils;

    #[test]
    fn test_pmtiles_elevation_enricher_handle_node() {
        let enricher = PMTilesElevationEnricher {};
        let result = enricher.handle_node();
        assert_eq!(0, result.unwrap())
    }

    #[test]
    fn test_match_node_to_tile() {
        let node = test_utils::simple_node_element_heidelberg_gaulskopfbrunnen(1, vec![]);

        let (x,y ) = match_node_to_tile(node, &16);

        assert_eq!(x as u64, 34354);
        assert_eq!(y as u64, 22396);
    }

    #[test]
    fn test_match_node_osm_example() {
        let node = test_utils::simple_node_element_osm_example(1, vec![]);

        let (x, y) = match_node_to_tile(node, &18);

        assert_eq!(x, 232798.93020672, "x should be correct");
        assert_eq!(y, 103246.41043781971, "y should be correct");
    }

    #[tokio::test]
    async fn test_match_node_gaisberg() {
        let node = test_utils::simple_node_element_heidelberg_gaisberg_peak(1, vec![]);

        let (x, y) = match_node_to_tile(node, &16);
        println!("x: {}, y: {}", x, y);
        assert_eq!(x, 34352.64369550222, "x should be correct");
        assert_eq!(y, 22394.08548168248, "y should be correct");

        let path = "test/pmtiles/hd-6-33-21.pmtiles";
        let bytes = get_tile_file(path, 16, 34352, 22394 ).await.unwrap();
        let (width, height, tile) = bytes_to_rgba(bytes);
        let elevation = get_elevation_for_pixel(tile, width, height, 0.64369550222 , 0.08548168248);
        assert_eq!(371.125, elevation);
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

    #[tokio::test]
    async fn test_convert_bytes() {
        let path = "test/pmtiles/hd-6-33-21.pmtiles";
        let bytes = get_tile_file(path, 16, 34354, 22396 ).await.unwrap();
        let (width, height, buf) = crate::handler::pmtiles::bytes_to_rgba(bytes);
        assert_eq!(width, 512);
        assert_eq!(height, 512);
        assert_eq!(buf.len(), 512 * 512 * 4);
        assert_eq!(buf[0], 129);
        assert_eq!(buf[1], 99);
        assert_eq!(buf[2], 16);
        assert_eq!(buf[3], 255);
    }

    #[ignore]
    #[test]
    fn test_calculate_elevation_for_pixel() {
        todo!("missing test")
    }

    // #[test]
    // fn test_convert_pixel_coordinate_to_byte_array_index() {
    //     let mut index = convert_pixel_coordinate_to_pixel_index(0, 0, 10);
    //     assert_eq!(0, index);
    //
    //     index = convert_pixel_coordinate_to_pixel_index(0, 1, 10);
    //     assert_eq!(40, index)
    // }
}