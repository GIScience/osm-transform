#[derive(Debug, Default)]
pub struct Config {
    pub param: i32,
    pub country_path: String,
}

impl Config {
    pub fn default() -> Self {
        Self {
            param: 0,
            country_path: "./test/mapping_test.csv".to_string(),
        }
    }
}
