use crate::handler::{Handler, HandlerData};
use crate::utils::s3_utils::{PMTilesDownloadUrls, TileInfo};
use bytes::Bytes;
use futures_util::TryStreamExt;
use libwebp::boxed::WebpBox;
use libwebp::WebPDecodeRGBA;
use log::info;
use osm_io::osm::model::node::Node;
use osm_io::osm::model::tag::Tag;
use pmtiles::s3::creds::Credentials;
use pmtiles::s3::{Bucket, Region};
use pmtiles::{AsyncPmTilesReader, HashMapCache, S3Backend, TileCoord, TileId};
use std::collections::{HashMap, HashSet};
use std::f64::consts::PI;
use std::sync::Arc;

pub struct PMTilesElevationEnricher {
    buffers: HashMap<TileId, Vec<Node>>,
    tiles_map: HashMap<TileId, PMTilesInfo>,
    planet_reader: Option<AsyncPmTilesReader<S3Backend, HashMapCache>>,
    failed_tile_lookups: HashSet<TileId>,
    buffer_threshold: usize,
    total_threshold: usize,

    bucket: String,
    credentials: Option<Credentials>,
    region: Option<Region>,
    planet_key: String,

    ele_lookups_successful: usize,
    ele_lookups_failed: usize,
    ele_lookups_skipped: usize,
}

pub struct PMTilesInfo {
    z_max: u8,
    key: String,
    reader: Option<AsyncPmTilesReader<S3Backend, HashMapCache>>,
}

impl PMTilesElevationEnricher {
    pub async fn new(url: String, bucket: String, path: String, buffer_threshold: usize, total_threshold: usize) -> Self {
        let access_key_id= std::env::var("AWS_ACCESS_KEY_ID").expect("AWS_ACCESS_KEY_ID not set");
        let secret_access_key= std::env::var("AWS_SECRET_ACCESS_KEY").expect("AWS_SECRET_ACCESS_KEY not set");
        let download_urls_key = format!("{path}download_urls.json");
        let download_urls = crate::utils::s3_utils::get_tiles_json_from_s3(url.clone(), bucket.clone(), download_urls_key, &access_key_id, &secret_access_key).await.unwrap();
        info!("JSON downloaded, initializing readers...");
        let credentials = Credentials::new(
            Some(access_key_id.as_str()),
            Some(secret_access_key.as_str()),
            None, None, None)
            .expect("failed to get credentials");

        let region = Region::Custom {
            region: "eu-central-1".to_owned(),
            endpoint: url,
        };
        let planet_key = format!("{}planet.pmtiles", path);
        let tiles_map = Self::extract_tiles_map(download_urls, path).await;
        info!("Readers initialized.");
        PMTilesElevationEnricher {
            buffers: HashMap::new(),
            tiles_map,
            planet_reader: None,
            failed_tile_lookups: HashSet::new(),
            buffer_threshold,
            total_threshold,

            bucket: bucket,
            credentials: Some(credentials),
            region: Some(region),
            planet_key,

            ele_lookups_successful: 0,
            ele_lookups_failed: 0,
            ele_lookups_skipped: 0,
        }
    }

    #[allow(dead_code)]
    async fn default() -> Self {
        info!("Initializing PMTilesElevationEnricher with defaults for testing...");
        let access_key_id= "AWS_ACCESS_KEY_ID".to_string();
        let secret_access_key= "AWS_SECRET_ACCESS_KEY".to_string();
        let bucket = "S3_BUCKET".to_string();
        let url = "S3_URL".to_string();
        let path = "mapterhorn/0.0.8/".to_string();
        let buffer_threshold = 100;
        let total_threshold = 1000;
        let download_urls = crate::utils::s3_utils::get_tiles_json_from_file().unwrap();
        let tiles_map = Self::extract_tiles_map(download_urls, path).await;
        PMTilesElevationEnricher {
            buffers: HashMap::new(),
            tiles_map,
            planet_reader: None,
            failed_tile_lookups: HashSet::new(),
            buffer_threshold,
            total_threshold,

            bucket: "Ignored for default".to_string(),
            credentials: None,
            region: None,
            planet_key: "Ignored for default".to_string(),

            ele_lookups_successful: 0,
            ele_lookups_failed: 0,
            ele_lookups_skipped: 0,
        }
    }

    async fn extract_tiles_map(download_urls: PMTilesDownloadUrls, path: String) -> HashMap<TileId, PMTilesInfo> {
        let mut tiles_map = HashMap::new();
        for tile_info in download_urls.items.iter() {
            if tile_info.name.starts_with("planet") {
                continue; // skip planet file
            }
            let mut tile_name = tile_info.name.as_str().split('.').next().unwrap().split('-');
            tile_name.next();
            let x = tile_name.next().unwrap().parse().unwrap();
            let y = tile_name.next().unwrap().parse().unwrap();
            let tile_id = TileCoord::new(6, x, y).unwrap();
            let z_max = tile_info.max_zoom;
            let key = format!("{}{}", path, tile_info.name);
            tiles_map.insert(TileId::from(tile_id), PMTilesInfo { z_max, key, reader: None });
        }
        tiles_map
    }

    async fn handle_nodes(&mut self, nodes: Vec<Node>) -> Vec<Node> {
        let mut result_vec = Vec::new();
        for node in nodes {
            result_vec.extend(self.handle_single_node(node.clone()).await);
        }
        result_vec
    }
    async fn handle_and_flush_nodes(&mut self, nodes: Vec<Node>) -> Vec<Node> {
        let mut result_vec = Vec::new();
        for node in &nodes {
            result_vec.extend(self.handle_single_node(node.clone()).await);
        }
        // flush all remaining buffers
        for tile_id in self.buffers.keys().cloned().collect::<Vec<TileId>>() {
            let mut buffer = self.buffers.remove(&tile_id).unwrap();
            result_vec.extend(self.handle_buffer(&tile_id, &mut buffer).await);
        }
        result_vec
    }

    async fn handle_single_node(&mut self, node: Node) -> Vec<Node> {
        let zoom = 6;
        let (tile_x, tile_y, _x_in_tile, _y_in_tile) = match_node_to_tile(&node, &zoom); // TODO make configurable later
        let tile_coordinate = TileCoord::new(zoom, tile_x, tile_y).unwrap();

        let tile_id = TileId::from(tile_coordinate);

        self.buffers.entry(tile_id).or_insert_with(Vec::new).push(node);
        self.check_threshold(&tile_id).await
    }

    async fn check_threshold(&mut self, tile_id: &TileId) -> Vec<Node> {
        let mut result_vec = Vec::new();
        if self.buffers[&tile_id].len() >= self.buffer_threshold {
            info!("Buffer for tile_id {tile_id:?} reached threshold {}, processing...", self.buffer_threshold);
            let mut buffer =self.buffers.remove(&tile_id).unwrap();
            result_vec.extend(self.handle_buffer(&tile_id, &mut buffer).await);
        }
        if self.buffers.values().map(|buffer| buffer.len()).sum::<usize>() >= self.total_threshold {
            info!("Total buffer size reached threshold {:?}, flushing largest buffer...", self.total_threshold);
            let largest_buffer_tile_id = self.buffers.iter().max_by_key(|entry| entry.1.len()).map(|(tile_id, _)| tile_id).unwrap().clone();
            let mut buffer = self.buffers.remove(&largest_buffer_tile_id).unwrap();
            result_vec.extend(self.handle_buffer(&largest_buffer_tile_id, &mut buffer).await);
        }
        result_vec
    }

    async fn handle_buffer(&mut self, tile_id: &TileId, buffer: &mut Vec<Node>) -> Vec<Node> {
        let mut result_vec = Vec::new();
        for node in buffer.iter_mut() {
            result_vec.push(self.enrich_node(tile_id, &mut node.clone()).await);
        }
        result_vec
    }

pub     async fn enrich_node(&mut self, tile_id: &TileId, node: &mut Node) -> Node {
        // TODO implement actual enrichment logic here
        // for now just return the node as is and count lookups
        let mut tile_info = self.tiles_map.get_mut(tile_id);
        let reader = if tile_info.is_none() {
            if self.planet_reader.is_none() {
                let new_reader = match self.credentials {
                    Some(_) => Some(get_reader_for_key(self.bucket.as_str(), self.planet_key.to_string(), self.credentials.clone().unwrap(), self.region.clone().unwrap()).await),
                    _ => None
                };
                if new_reader.is_none() {
                    panic!("Reader for tile_id {tile_id:?} could not be initialized, but needed for enrichment");
                }
                tile_info.as_mut().unwrap().reader = new_reader;
            }
            self.planet_reader.as_ref().unwrap()
        } else {
            if tile_info.as_ref().unwrap().reader.is_none() {
                let new_reader = match self.credentials {
                    Some(_) => Some(get_reader_for_key(self.bucket.as_str(), tile_info.as_ref().unwrap().key.to_string(), self.credentials.clone().unwrap(), self.region.clone().unwrap()).await),
                    _ => None
                };
                if new_reader.is_none() {
                    panic!("Reader for tile_id {tile_id:?} could not be initialized, but needed for enrichment");
                }
                tile_info.as_mut().unwrap().reader = new_reader;
            }
            tile_info.as_ref().unwrap().reader.as_ref().unwrap()
        };
        let zoom = if tile_info.is_none() { 12 } else { tile_info.as_ref().unwrap().z_max };
        // println!("Node {:?} matched to tile_id {tile_id:?} with zoom {zoom}", node.id());
        for z in (13..=zoom).rev() {
            let (tile_x, tile_y, x_in_tile, y_in_tile) = match_node_to_tile(&node, &z);
            if self.failed_tile_lookups.contains(&TileId::from(TileCoord::new(z, tile_x, tile_y).unwrap())) {
                continue;
            }
            let tile = get_tile_from_reader(reader, z, tile_x, tile_y).await;
            println!("Fetched: {:?}", tile);

            if !tile.is_none() {
                self.ele_lookups_successful += 1;
                return add_elevation_to_node(node, tile.unwrap(), x_in_tile, y_in_tile)
            } else {
                println!("Tile {z:?} {tile_x:?} {tile_y:?} could not be found");
                self.failed_tile_lookups.insert(TileId::from(TileCoord::new(z, tile_x, tile_y).unwrap()));
            }
        }
        self.ele_lookups_failed += 1;
        node.to_owned()
    }

        fn name(&self) -> String {
            String::from("PMTilesElevationEnricher")
        }

        pub async fn handle(&mut self, data: &mut super::HandlerData) {
            if data.nodes.len() > 0 {
                data.nodes = self.handle_nodes(data.nodes.clone()).await;
            }
        }

        pub async fn flush(&mut self, data: &mut super::HandlerData) {
            data.nodes = self.handle_and_flush_nodes(data.nodes.clone()).await;
        }

        pub fn close(&mut self, data: &mut HandlerData) {
            data.elevation_found_node_count = self.ele_lookups_successful as u64;
            data.elevation_not_found_node_count = self.ele_lookups_failed as u64;
            data.elevation_not_relevant_node_count = self.ele_lookups_skipped as u64;
        }

}

fn add_elevation_to_node(node: &mut Node, tile: Bytes, x: f64, y: f64) -> Node {
    let (width, height, rgba) = bytes_to_rgba(tile);
    let elevation = get_elevation_for_pixel(rgba, width, height, x, y);
    node.tags_mut().push(Tag::new("ele".to_string(), elevation.to_string()));
    println!("{:?}", node);
    node.to_owned()
}

async fn get_tile_from_reader(reader: &AsyncPmTilesReader<S3Backend, HashMapCache>, z: u8, x: u32, y: u32) -> Option<Bytes> {
    println!("Fetch: {z:?}, {x:?}, {y:?}");
    let coord = TileCoord::new(z, x, y).unwrap();
    reader.get_tile(coord).await.unwrap()
}

pub async fn get_reader_for_key(bucket_name: &str, key: String, credentials: Credentials, region: Region) -> AsyncPmTilesReader<S3Backend, HashMapCache> {
    let cache = HashMapCache::default();
    let bucket = Bucket::new(bucket_name, region, credentials).expect("failed to create bucket").with_path_style();
    AsyncPmTilesReader::new_with_cached_bucket_path(cache, *bucket, key)
            .await
            .unwrap()
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

async fn get_tile_ids() -> Result<(), pmtiles::PmtError> {
    let path = "test/pmtiles/hd-6-33-21.pmtiles";
    let reader = Arc::new(AsyncPmTilesReader::new_with_path(path).await?);
    let mut entries = reader.entries();
    while let Some(entry) = entries.try_next().await? {
        entry.iter_coords().for_each(|coord| {
            let xyz = TileCoord::from(coord);
            println!("entry: {xyz:?}  ");
        });
        println!("entry: {entry:?}");
    }
    Ok(())
}

async fn get_tile_ids_from_s3(url: String, bucket_name: String, key: String, access_key_id: &str, secret_access_key: &str) -> Result<Vec<TileId>,Box<dyn std::error::Error>> {
    let credentials = Credentials::new(
        Some(&access_key_id),
        Some(&secret_access_key),
        None, None, None)
        .expect("failed to get credentials");

    let region = Region::Custom {
        region: "eu-central-1".to_owned(),
        endpoint: url,
    };

    let bucket = Bucket::new(&bucket_name, region, credentials).expect("failed to create bucket").with_path_style();
    let reader = Arc::new(AsyncPmTilesReader::new_with_bucket_path(*bucket, key).await?);
    let mut entries = reader.entries();
    let mut tile_ids = vec![];
    while let Some(entry) = entries.try_next().await? {
        entry.iter_coords().for_each(|tile_id| {
            tile_ids.push(tile_id);
        }
        );
    }
    Ok(tile_ids)
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
    use benchmark_rs::stopwatch::StopWatch;
    use log::error;
    use crate::handler::pmtiles::{bytes_to_rgba, calculate_elevation_for_pixel, get_elevation_for_pixel, get_reader_for_key, get_tile_from_file, get_tile_from_reader, get_tile_from_s3, get_tile_ids, get_tile_ids_from_s3, match_node_to_tile};
    use crate::handler::Handler;
    use crate::handler::{pmtiles::PMTilesElevationEnricher, HandlerData};
    use crate::utils::test_utils;
    use crate::utils::test_utils::{loc_hd_gaulskopfbrunnen, loc_osm_example, simple_node_element, wgs84_coordinate_hamburg_elbphilharmonie, wgs84_coordinate_limburg_traffic_circle, wgs84_coordinate_limburg_vienna_house};
    use osm_io::osm::model::coordinate::Coordinate;
    use pmtiles::s3::creds::Credentials;
    use pmtiles::s3::Region;

    // TODO write test for new handle node
    #[tokio::test]
    async fn test_pmtiles_elevation_enricher_handle_node() {
        let mut enricher = PMTilesElevationEnricher::default().await;
        enricher.handle_single_node(wgs84_coordinate_hamburg_elbphilharmonie().to_node(1));
        enricher.handle_single_node(wgs84_coordinate_hamburg_elbphilharmonie().to_node(1));
        assert!(!enricher.buffers.is_empty());
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
        let mut enricher = PMTilesElevationEnricher::default();
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

    #[tokio::test]
    async fn test_match_node_gaisberg_s3() {
        let node = test_utils::loc_hd_gaisberg_peak().to_node(1);

        let zoom = 16;
        let (tile_x, tile_y, x_in_tile, y_in_tile) = match_node_to_tile(&node, &zoom);
        println!("zoom={zoom} tile_x={tile_x} tile_y={tile_y} x_in_tile={x_in_tile} y_in_tile={y_in_tile}");

        let access_key_id= std::env::var("AWS_ACCESS_KEY_ID").expect("AWS_ACCESS_KEY_ID not set");
        let secret_access_key= std::env::var("AWS_SECRET_ACCESS_KEY").expect("AWS_SECRET_ACCESS_KEY not set");
        let url = "https://warm.storage.heigit.org".to_string();
        let bucket = "heigit-highres-elevation-data".to_string();
        let key = "mapterhorn/0.0.8/6-33-21.pmtiles".to_string();
        let credentials = Credentials::new(
            Some(access_key_id.as_str()),
            Some(secret_access_key.as_str()),
            None, None, None)
            .expect("failed to get credentials");

        let region = Region::Custom {
            region: "eu-central-1".to_owned(),
            endpoint: url,
        };
        let reader = get_reader_for_key(&*bucket, key, credentials, region).await;
        let mut stopwatch = StopWatch::new();
        stopwatch.start();
        let bytes = get_tile_from_reader(&reader, zoom, tile_x, tile_y).await.unwrap();
        println!("{} done, time: {}", "get tile", stopwatch);
        let (width, height, tile) = bytes_to_rgba(bytes);
        println!("{} done, time: {}", "read tile", stopwatch);
        let elevation = get_elevation_for_pixel(tile, width, height, x_in_tile, y_in_tile);
        println!("{} done, time: {}", "read ele", stopwatch);
        assert_eq!(elevation, test_utils::loc_hd_gaisberg_peak().ele());
    }

    #[ignore]
    #[tokio::test]
    async fn test_pmtiles_elevation_enricher() {
        let mut handler = PMTilesElevationEnricher::default().await;

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
    async fn test_get_tile_ids() {
        get_tile_ids().await.unwrap();
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


    #[ignore]
    #[tokio::test]
    async fn test_one_list_pmtiles_from_s3() {
        // TODO this needs a better test setup
        // because environment variables are probably not the best choice of secrets management here
        let aws_access_key_id = std::env::var("AWS_ACCESS_KEY_ID").expect("AWS_ACCESS_KEY_ID not set");
        let aws_secret_access_key = std::env::var("AWS_SECRET_ACCESS_KEY").expect("AWS_SECRET_ACCESS_KEY not set");
        let url = std::env::var("S3_URL").unwrap();
        let bucket = std::env::var("S3_BUCKET").unwrap();
        let tiles_path = "mapterhorn/0.0.8/";
        let tile_names = ["6-5-14.pmtiles", "6-50-33.pmtiles", "6-52-36.pmtiles", "6-52-38.pmtiles"];
        for tile_name in tile_names.iter() {
            println!("downloading {tile_name}");
            let key = format!("{tiles_path}{tile_name}");
            let current_tile_ids = get_tile_ids_from_s3(url.clone(), bucket.clone(), key, aws_access_key_id.as_str(), aws_secret_access_key.as_str()).await.unwrap();
            println!("current_tile_ids.len(): {}", current_tile_ids.len());
        }
    }

    #[ignore]
    #[tokio::test]
    async fn test_list_pmtiles_from_s3() {
        // TODO this needs a better test setup
        // because environment variables are probably not the best choice of secrets management here
        let aws_access_key_id= std::env::var("AWS_ACCESS_KEY_ID").expect("AWS_ACCESS_KEY_ID not set");
        let aws_secret_access_key= std::env::var("AWS_SECRET_ACCESS_KEY").expect("AWS_SECRET_ACCESS_KEY not set");
        let url = std::env::var("S3_URL").unwrap();
        let bucket = std::env::var("S3_BUCKET").unwrap();
        let download_urls_key = "mapterhorn/0.0.8/download_urls.json".to_string();

        let download_urls = crate::utils::s3_utils::get_tiles_json_from_s3(url.clone(), bucket.clone(), download_urls_key, &aws_access_key_id, &aws_secret_access_key).await.unwrap();
        let mut tile_ids = vec![];
        let tiles_path = "mapterhorn/0.0.8/";
        for tile_info in download_urls.items.iter() {
            println!("downloading {:?}", tile_info.name);
            let tile_name = tile_info.name.as_str();
            let key = format!("{tiles_path}{tile_name}");
            let current_tile_ids = get_tile_ids_from_s3(url.clone(), bucket.clone(), key, aws_access_key_id.as_str(), aws_secret_access_key.as_str()).await.unwrap();
            tile_ids.extend(current_tile_ids);
        }
        println!("tile_ids.len(): {}", tile_ids.len());
    }
}