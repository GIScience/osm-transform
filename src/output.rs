use std::path::PathBuf;

use osm_io::osm::model::element::Element;
use osm_io::osm::model::node::Node;
use osm_io::osm::model::relation::Relation;
use osm_io::osm::model::way::Way;
use osm_io::osm::pbf;
use osm_io::osm::pbf::compression_type::CompressionType;
use osm_io::osm::pbf::file_info::FileInfo;

use crate::handler::{Handler, HandlerResult, into_node_element, into_relation_element, into_way_element};

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
    fn handle_node(&mut self, node: Node) -> Vec<Element> {
        self.writer.write_element(Element::Node { node }).expect("Failed to write node");
        vec![]
    }

    fn handle_way(&mut self, way: Way) -> Vec<Element> {
        self.writer.write_element(Element::Way { way }).expect("Failed to write way");
        vec![]
    }


    fn handle_relation(&mut self, relation: Relation) -> Vec<Element> {
        self.writer.write_element(Element::Relation { relation }).expect("Failed to write relation");
        vec![]
    }
}

impl Handler for OutputHandler {
    fn name(&self) -> String { "OutputHandler".to_string() }
    fn handle_element(&mut self, element: Element) -> Vec<Element> {
        match element {
            Element::Node { node } => self.handle_node(node),
            Element::Way { way } => self.handle_way(way),
            Element::Relation { relation } => self.handle_relation(relation),
            Element::Sentinel => vec![]
        }
    }

    fn handle_nodes(&mut self, elements: Vec<Node>) -> Vec<Node> {
        for element in elements {
            self.writer.write_element(into_node_element(element)).expect("Failed to write node");
        }
        Vec::new()
    }

    fn handle_ways(&mut self, elements: Vec<Way>) -> Vec<Way> {
        for element in elements {
            self.writer.write_element(into_way_element(element)).expect("Failed to write way");
        }
        Vec::new()
    }

    fn handle_relations(&mut self, elements: Vec<Relation>) -> Vec<Relation> {
        for element in elements {
            self.writer.write_element(into_relation_element(element)).expect("Failed to write relation");
        }
        Vec::new()
    }

    fn add_result(&mut self, result: HandlerResult) -> HandlerResult {
        self.close();
        result
    }
}
