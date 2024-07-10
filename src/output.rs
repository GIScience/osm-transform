use std::path::PathBuf;
use std::string::ToString;
use osm_io::osm::model::element::Element;
use osm_io::osm::model::relation::Relation;
use osm_io::osm::model::way::Way;
use osm_io::osm::model::node::Node;
use osm_io::osm::pbf;
use osm_io::osm::pbf::compression_type::CompressionType;
use osm_io::osm::pbf::file_info::FileInfo;
use crate::handler::{Handler, HandlerResult};
use crate::conf::Config;

pub struct OutputHandler {
    pub writer: pbf::writer::Writer,
    pub next: Option<Box<dyn Handler>>
}


impl OutputHandler {
    pub fn new(config: &Config) -> Self {
        let mut file_info = FileInfo::default();
        file_info.with_writingprogram_str("rusty-routes");
        Self {
            writer: pbf::writer::Writer::from_file_info(PathBuf::from(config.output_path.to_string()), file_info, CompressionType::Zlib).expect("Failed to create output writer"),
            next: None
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
    fn process_node_owned(&mut self, node: Node) -> Option<Node> {
        self.writer.write_element(Element::Node { node }).expect("Failed to write node");
        None
    }

    fn process_node(&mut self, node: &mut Node) -> bool {
        log::debug!("Writing node: {:?}", node);
        self.writer.write_element(Element::Node { node: node.to_owned() }).expect("Failed to write node");
        false
    }

    fn process_way_owned(&mut self, way: Way) -> Option<Way> {
        self.writer.write_element(Element::Way { way }).expect("Failed to write way");
        None
    }

    fn process_way(&mut self, way: &mut Way) -> bool {
        self.writer.write_element(Element::Way { way: way.clone() }).expect("Failed to write way");
        false
    }

    fn process_relation_owned(&mut self, relation: Relation) -> Option<Relation> {
        self.writer.write_element(Element::Relation { relation }).expect("Failed to write relation");
        None
    }

    fn process_relation(&mut self, relation: &mut Relation) -> bool {
        self.writer.write_element(Element::Relation { relation: relation.clone() }).expect("Failed to write relation");
        false
    }

    fn get_next(&mut self) -> &mut Option<Box<dyn Handler>> {
        return &mut self.next;
    }

    fn process_results(&mut self, res: &mut HandlerResult) {
        self.close()
    }
}
