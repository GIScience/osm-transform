use std::path::PathBuf;
use osm_io::osm::pbf::reader::Reader;

pub fn read_osm_timestamp(file_path: &PathBuf) -> i64 {
    let reader = Reader::new(&file_path).expect("file not found");
    let timestamp = reader.info().osmosis_replication_timestamp().expect("no timestamp found");
    timestamp
}