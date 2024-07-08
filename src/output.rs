use std::path::PathBuf;
use std::string::ToString;
use osm_io::osm::model::element::Element;
use osm_io::osm::model::relation::Relation;
use osm_io::osm::model::way::Way;
use osm_io::osm::pbf;
use osm_io::osm::pbf::compression_type::CompressionType;
use osm_io::osm::pbf::file_info::FileInfo;
use crate::handler::{Handler, HandlerResult};
use crate::conf::Config;
use crate::osm_model::{MutableNode, MutableWay};

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
    fn process_node(&mut self, node: &mut MutableNode) -> bool {
        log::debug!("Writing node: {:?}", node);
        self.writer.write_element(Element::Node { node: node.make_node() }).expect("Failed to write node");
        false
    }

    fn process_way(&mut self, way: &mut MutableWay) -> bool {
        self.writer.write_element(Element::Way { way: <MutableWay<'_> as Clone>::clone(&way).build() }).expect("Failed to write way");
        false
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
