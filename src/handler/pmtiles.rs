use crate::handler::Handler;
use bytes::Bytes;
use libwebp::boxed::WebpBox;
use libwebp::WebPDecodeRGBA;
use osm_io::osm::model::node::Node;
use std::collections::HashMap;
use std::f64::consts::PI;
use pmtiles::s3::creds::Credentials;
use pmtiles::s3::{Bucket, Region};
use pmtiles::{AsyncPmTilesReader, HashMapCache, TileCoord, TileId};
use osm_io::osm::model::coordinate::Coordinate;

pub(crate) struct PMTilesElevationEnricher {
    buckets: HashMap<TileId, Vec<Node>>
}

impl PMTilesElevationEnricher {
    fn new() -> Self {
        PMTilesElevationEnricher { buckets: HashMap::new() }
    }

    fn handle_single_node(&mut self, node: Node) {
        let zoom = 6;
        let (tile_x, tile_y, x_in_tile, y_in_tile) = match_node_to_tile(&node, &zoom); // TODO make configurable later
        let tile_coordinate = TileCoord::new(zoom, tile_x, tile_y).unwrap();

        let tile_id = TileId::from(tile_coordinate);

        self.buckets.entry(tile_id).or_insert_with(Vec::new).push(node);
    }
}


impl Handler for PMTilesElevationEnricher {
    fn name(&self) -> String {
        String::from("PMTilesElevationEnricher")
    }

    fn handle(&mut self, data: &mut super::HandlerData) { 
        for node in &data.nodes {
            self.handle_single_node(node.clone());
        }
    }

    fn flush(&mut self, data: &mut super::HandlerData) {
        todo!()
    }
}

async fn get_tile_from_s3(url: String, bucket_name: String, key: String, secret_access_key: &str, access_key_id: &str, z: u8, x: u32, y: u32) -> Option<Bytes> {
    let credentials = Credentials::new(
        Some(access_key_id),
        Some(secret_access_key),
        None, None, None)
        .expect("failed to get credentials");

    let region = Region::Custom {
        region: "eu-central-1".to_owned(),
        endpoint: url,
    };

    let cache = HashMapCache::default();
    let bucket = Bucket::new(&bucket_name, region, credentials).expect("failed to create bucket").with_path_style();
    let reader =
        AsyncPmTilesReader::new_with_cached_bucket_path(cache, *bucket, key)
            .await
            .unwrap();
    let coord = TileCoord::new(z, x, y).unwrap();
    reader.get_tile(coord).await.unwrap()
}

async fn get_tile_from_file(path: &str, z: u8, x: u32, y: u32) -> Option<Bytes> {
    let reader = AsyncPmTilesReader::new_with_path(path).await.unwrap();
    let coord = TileCoord::new(z, x, y).unwrap();
    reader.get_tile(coord).await.unwrap()
}

fn bytes_to_rgba(data: Bytes) -> (u32, u32, WebpBox<[u8]>) {
    WebPDecodeRGBA(data.iter().as_slice()).unwrap()
}

fn match_node_to_tile(node: &Node, zoom: &u8) -> (u32, u32, f64, f64) {
    // see https://wiki.openstreetmap.org/wiki/Slippy_map_tilenames
    let n = (2 as u32).pow(zoom.clone() as u32) as f64;
    let x = n * (0.5 + node.coordinate().lon() / 360.0);
    let y = n * (0.5 - node.coordinate().lat().to_radians().tan().asinh() / (2.0 * PI));
    let tile_x = x as u32;
    let tile_y = y as u32;
    let x_in_tile = x.fract();
    let y_in_tile = y.fract();
    (tile_x, tile_y, x_in_tile, y_in_tile)
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
    use osm_io::osm::model::coordinate::Coordinate;
    use crate::handler::pmtiles::{bytes_to_rgba, get_elevation_for_pixel, get_tile_from_file, get_tile_from_s3, match_node_to_tile};
    use crate::handler::Handler;
    use crate::handler::{pmtiles::PMTilesElevationEnricher, HandlerData};
    use crate::utils::test_utils;
    use crate::utils::test_utils::{loc_hd_gaulskopfbrunnen, loc_osm_example, simple_node_element, wgs84_coordinate_hamburg_elbphilharmonie, wgs84_coordinate_limburg_traffic_circle, wgs84_coordinate_limburg_vienna_house};


    // TODO write test for new handle node
    #[test]
    fn test_pmtiles_elevation_enricher_handle_node() {
        let mut enricher = PMTilesElevationEnricher::new();
        enricher.handle_single_node(wgs84_coordinate_hamburg_elbphilharmonie().to_node(1));
        enricher.handle_single_node(wgs84_coordinate_hamburg_elbphilharmonie().to_node(1));
        assert!(!enricher.buckets.is_empty());
    }


    #[test]
    fn test_match_node_to_tile() {
        let node = test_utils::simple_node_element_heidelberg_gaulskopfbrunnen(1, vec![]);

        let (tile_x, tile_y, x_in_tile, y_in_tile) = match_node_to_tile(&node, &16);

        assert_eq!(tile_x, 34354);
        assert_eq!(tile_y, 22396);
        assert!(test_utils::are_floats_close_7(x_in_tile, 0.8202552888906212));
        assert!(test_utils::are_floats_close_7(y_in_tile, 0.5625188843041542));
    }

    // TODO this is just explorative for now - develop a useful test
    #[test]
    fn test_match_node_to_tile__() {
        let mut enricher = PMTilesElevationEnricher::new();
        let mut coords = vec![
            simple_node_element(1, Coordinate::new(8.697355076272407, 49.4095351845969), vec![]),
            simple_node_element(1, Coordinate::new(8.701197202042264, 49.4110429951535), vec![]),
            simple_node_element(1, Coordinate::new(8.705683259505369, 49.40871145371345), vec![]),
            simple_node_element(1, Coordinate::new(8.693577343671906, 49.405039400462755), vec![]),
            simple_node_element(1, Coordinate::new(8.704674433185888, 49.40240037890541), vec![]),
            simple_node_element(1, Coordinate::new(8.695438792261415, 49.39645148761798), vec![]),
            simple_node_element(1, wgs84_coordinate_limburg_vienna_house().get_coordinate(), vec![]),
            simple_node_element(1, wgs84_coordinate_limburg_traffic_circle().get_coordinate(), vec![]),
            wgs84_coordinate_hamburg_elbphilharmonie().to_node(122),
            loc_hd_gaulskopfbrunnen().to_node(123),
            loc_osm_example().to_node(2),
        ];
        let zoom = 18;
        for node in &coords {
            let (tile_x, tile_y, x_in_tile, y_in_tile) = match_node_to_tile(&node, &zoom);
            println!("zoom={zoom} tile_x={tile_x} tile_y={tile_y} x_in_tile={x_in_tile} y_in_tile={y_in_tile}");
        }
    }

    #[test]
    fn test_match_node_osm_example() {
        let node = test_utils::simple_node_element_osm_example(1, vec![]);

        let zoom = 18;
        let (tile_x, tile_y, x_in_tile, y_in_tile) = match_node_to_tile(&node, &zoom);
        println!("zoom={zoom} tile_x={tile_x} tile_y={tile_y} x_in_tile={x_in_tile} y_in_tile={y_in_tile}");
        assert_eq!(tile_x, 232798, "x should be correct");
        assert_eq!(tile_y, 103246, "y should be correct");
        assert!(test_utils::are_floats_close_7(x_in_tile, 0.93020672));
        assert!(test_utils::are_floats_close_7(y_in_tile, 0.41043781971));
    }

    #[tokio::test]
    async fn test_match_node_gaisberg() {
        let node = test_utils::loc_hd_gaisberg_peak().to_node(1);

        let zoom = 16;
        let (tile_x, tile_y, x_in_tile, y_in_tile) = match_node_to_tile(&node, &zoom);
        println!("zoom={zoom} tile_x={tile_x} tile_y={tile_y} x_in_tile={x_in_tile} y_in_tile={y_in_tile}");
        assert_eq!(tile_x, 34352, "x should be correct");
        assert_eq!(tile_y, 22394, "y should be correct");
        assert!(test_utils::are_floats_close_7(x_in_tile, 0.64369550222));
        assert!(test_utils::are_floats_close_7(y_in_tile, 0.08548168248));
        let path = "test/pmtiles/hd-6-33-21.pmtiles";
        let bytes = get_tile_from_file(path, zoom, tile_x, tile_y ).await.unwrap();
        let (width, height, tile) = bytes_to_rgba(bytes);
        let elevation = get_elevation_for_pixel(tile, width, height, x_in_tile, y_in_tile);
        assert_eq!(elevation, test_utils::loc_hd_gaisberg_peak().ele());
    }

    #[ignore]
    #[test]
    fn test_pmtiles_elevation_enricher() {
        let mut handler = PMTilesElevationEnricher::new();

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
        let bytes = get_tile_from_file(path, 16, 34354, 22396 ).await.unwrap();
        assert!(!bytes.is_empty());
    }

    #[tokio::test]
    async fn test_convert_bytes() {
        let path = "test/pmtiles/hd-6-33-21.pmtiles";
        let bytes = get_tile_from_file(path, 16, 34354, 22396 ).await.unwrap();
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
    #[ignore]
    #[tokio::test]
    async fn test_get_tile_from_s3() {
        // TODO this needs a better test setup
        // because environment variables are probably not the best choice of secrets management here
        let aws_access_key_id=std::env::var("AWS_ACCESS_KEY_ID").unwrap();
        let aws_secret_access_key= std::env::var("AWS_SECRET_ACCESS_KEY").unwrap();
        let url = std::env::var("S3_URL").unwrap();
        let bucket = std::env::var("S3_BUCKET").unwrap();
        let path = "mapterhorn/0.0.8/";
        let pmtiles = "6-33-21.pmtiles";
        let key = format!("{path}{pmtiles}");
        let bytes = get_tile_from_s3(url, bucket, key, &aws_secret_access_key, &aws_access_key_id, 16, 34352, 22394).await.unwrap();
        let (width, height, tile) = bytes_to_rgba(bytes);
        let elevation = get_elevation_for_pixel(tile, width, height, 0.64369550222 , 0.08548168248);
        assert_eq!(371.125, elevation);
    }

}