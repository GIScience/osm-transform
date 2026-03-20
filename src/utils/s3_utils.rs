//use aws_sdk_s3::Config;
//use aws_sdk_s3::config::SharedCredentialsProvider;
use pmtiles::aws_sdk_s3::config::{BehaviorVersion, Credentials, Region};
use pmtiles::aws_sdk_s3::{Client, Config};

async fn get_tiles_json_from_s3(url: String, bucket_name: String, key: String, secret_access_key: &str, access_key_id: &str) -> String {

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

    match rustfs_client
        .get_object()
        .bucket(bucket_name)
        .key(key)
        .send()
        .await
    {
        Ok(res) => {
            println!("Object downloaded successfully, res: {:?}", res);
            res.body.collect().await
                .map(|bytes| String::from_utf8(bytes.into_bytes().to_vec()).unwrap())
                .unwrap_or_else(|e| {
                    println!("Error collecting object body: {:?}", e);
                    String::new()
                })
        }
        Err(e) => {
            println!("Error downloading object: {:?}", e);
            String::new()
        }
    }
}

#[cfg(test)]
mod test {
    use crate::utils::s3_utils::get_tiles_json_from_s3;

    #[ignore]
    #[tokio::test]
    async fn test_list_pmtiles_from_s3() {
        // TODO this needs a better test setup
        // because environment variables are probably not the best choice of secrets management here
        let aws_access_key_id= std::env::var("AWS_ACCESS_KEY_ID").unwrap().as_str();
        let aws_secret_access_key= std::env::var("AWS_SECRET_ACCESS_KEY").unwrap().as_str();
        let url = std::env::var("S3_URL").unwrap();
        let bucket = std::env::var("S3_BUCKET").unwrap();
        let key = "mapterhorn/0.0.8/download_urls.json".to_string();
        let res = get_tiles_json_from_s3(url, bucket, key, &aws_secret_access_key, &aws_access_key_id).await;
        assert!(res != "");
    }
}