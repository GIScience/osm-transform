use std::f64::consts::PI;

use crate::handler::Handler;
use bytes::Bytes;
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

fn match_node_to_tile(node: Node, zoom: u8) -> TileCoord {
    let lon = node.coordinate().lon();
    let lat = node.coordinate().lat();

    let lat_radian = lat.to_radians();
    let n = (2 as u8).pow(zoom as u32) as f64;

    let tile_x = ((lon + 180.0) / (360.0 * n)) as u32;
    let tile_y = ((1.0 - lat_radian.tan().asinh() / PI) / 2.0 * n) as u32;

    TileCoord::new(zoom, tile_x, tile_y).unwrap()
}

#[cfg(test)]
mod test {
    use crate::handler::Handler;
    use crate::handler::{pmtiles::PMTilesElevationEnricher, HandlerData};
    use crate::utils::test_utils;

    #[test]
    fn test_pmtiles_elevation_enricher_handle_node() {
        let enricher = PMTilesElevationEnricher {};
        let result = enricher.handle_node();
        assert_eq!(0, result.unwrap())
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
}
