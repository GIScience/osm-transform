#[derive(Debug, Default)]
pub struct Config {
    pub param: i32,
    pub country_path: String,
    pub input_path: String,
    pub output_path: String,
    pub with_copy: bool,
}

impl Config {
    pub fn default() -> Self {
        Self {
            param: 0,
            country_path: "test/mapping_test.csv".to_string(),
            input_path:  "test/baarle_small.pbf".to_string(),
            output_path:  "output.pbf".to_string(),
            with_copy: false,
        }
    }
}
