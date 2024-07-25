use std::path::PathBuf;

use osm_io::osm::model::element::Element;
use osm_io::osm::model::node::Node;
use osm_io::osm::model::relation::Relation;
use osm_io::osm::model::way::Way;
use osm_io::osm::pbf;
use osm_io::osm::pbf::compression_type::CompressionType;
use osm_io::osm::pbf::file_info::FileInfo;

use crate::handler::{Handler, HandlerResult};

pub struct OutputHandler {
    pub writer: pbf::writer::Writer,
}


impl OutputHandler {
    pub fn new(output_path: PathBuf) -> Self {
        let mut file_info = FileInfo::default();
        file_info.with_writingprogram_str("rusty-routes");
        Self {
            writer: pbf::writer::Writer::from_file_info(output_path, file_info, CompressionType::Zlib).expect("Failed to create output writer"),
        }
    }

    pub fn init(&mut self) {
        self.writer.write_header().expect("Failed to write header");
    }

    pub fn close(&mut self) {
        self.writer.close().expect("Failed to close writer");
    }
}

impl Handler for OutputHandler {
    fn handle_node(&mut self, node: Node) -> Vec<Node> {
        self.writer.write_element(Element::Node { node }).expect("Failed to write node");
        vec![]
    }

    fn handle_way(&mut self, way: Way) -> Vec<Way> {
        self.writer.write_element(Element::Way { way }).expect("Failed to write way");
        vec![]
    }


    fn handle_relation(&mut self, relation: Relation) -> Vec<Relation> {
        self.writer.write_element(Element::Relation { relation }).expect("Failed to write relation");
        vec![]
    }

    fn add_result(&mut self, result: HandlerResult) -> HandlerResult {
        self.close();
        result
    }
}
