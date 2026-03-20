use std::env::VarError;
//use aws_sdk_s3::Config;
//use aws_sdk_s3::config::SharedCredentialsProvider;
use pmtiles::aws_sdk_s3::config::{BehaviorVersion, Credentials, Region};
use pmtiles::aws_sdk_s3::{Client, Config};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub(crate) struct PMTilesDownloadUrls {
    pub(crate) version: String,
    pub(crate) items: Vec<TileInfo>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct TileInfo {
    pub(crate) name: String,
    pub(crate) url: String,
    pub(crate) md5sum: String,
    pub(crate) size: u64,
    pub(crate) min_lon: f64,
    pub(crate) min_lat: f64,
    pub(crate) max_lon: f64,
    pub(crate) max_lat: f64,
    pub(crate) min_zoom: u8,
    pub(crate) max_zoom: u8,
}

pub(crate) async fn get_tiles_json_from_s3(url: String, bucket_name: String, key: String, access_key_id: String, secret_access_key: String) -> Result<PMTilesDownloadUrls,Box<dyn std::error::Error>> {

    let credentials = Credentials::new(
        access_key_id,
        secret_access_key,
        None,
        None,
        "rustfs",
    );

    let region = Region::new("should-be-ignored-by-rustfs");

    let config = Config::builder()
        .credentials_provider(credentials)
        .region(region)
        .behavior_version(BehaviorVersion::latest())
        .endpoint_url(url)
        .force_path_style(true);

    let rustfs_client = Client::from_conf(config.build());

    let result = rustfs_client
        .get_object()
        .bucket(bucket_name)
        .key(key)
        .send()
        .await;

    let result_string = result?.body.collect().await.map(|bytes| String::from_utf8(bytes.into_bytes().to_vec()).unwrap()).unwrap();
    let download_urls: PMTilesDownloadUrls = serde_json::from_str(&result_string).unwrap();
    Ok(download_urls)
}

#[cfg(test)]
mod test {
    use crate::utils::s3_utils::{get_tiles_json_from_s3};

    #[ignore]
    #[tokio::test]
    async fn test_list_pmtiles_from_s3() {
        // TODO this needs a better test setup
        // because environment variables are probably not the best choice of secrets management here
        let aws_access_key_id= std::env::var("AWS_ACCESS_KEY_ID").expect("AWS_ACCESS_KEY_ID not set");
        let aws_secret_access_key= std::env::var("AWS_SECRET_ACCESS_KEY").expect("AWS_SECRET_ACCESS_KEY not set");
        let url = std::env::var("S3_URL").unwrap();
        let bucket = std::env::var("S3_BUCKET").unwrap();
        let key = "mapterhorn/0.0.8/download_urls.json".to_string();

        let download_urls = get_tiles_json_from_s3(url, bucket, key, aws_access_key_id, aws_secret_access_key).await.unwrap();

        println!("items count: {}", download_urls.items.len());
        for tile_info in download_urls.items.iter() {
            println!("{:?}", tile_info);
        }

        assert_eq!(456, download_urls.items.len());
    }
}