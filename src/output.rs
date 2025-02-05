use std::path::PathBuf;
use osm_io::osm::model::element::Element;
use osm_io::osm::model::node::Node;
use osm_io::osm::model::relation::Relation;
use osm_io::osm::model::way::Way;
use osm_io::osm::pbf;
use osm_io::osm::pbf::compression_type::CompressionType;
use osm_io::osm::pbf::file_info::FileInfo;
use crate::handler::{Handler, HandlerResult, into_node_element, into_relation_element, into_way_element};

pub struct SimpleOutputHandler {
    pub writer: pbf::writer::Writer,
}


impl SimpleOutputHandler {
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

impl Handler for SimpleOutputHandler {
    fn name(&self) -> String { "OutputHandler".to_string() }

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


pub struct SplittingOutputHandler {
    pub node_writer: pbf::writer::Writer,
    pub way_relation_writer: pbf::writer::Writer,
}

impl SplittingOutputHandler {
    pub fn new(output_path: PathBuf) -> Self {
        let mut file_info = FileInfo::default();
        file_info.with_writingprogram_str("rusty-routes");

        let base_name = output_path.file_stem().expect("Failed to get file stem").to_str().expect("Failed to convert file stem to string");
        let ways_relations_path = PathBuf::from(format!("{}_ways_relations.pbf", base_name));

        Self {
            node_writer: pbf::writer::Writer::from_file_info(output_path, file_info.clone(), CompressionType::Zlib).expect("Failed to create node output writer"),
            way_relation_writer: pbf::writer::Writer::from_file_info(ways_relations_path, file_info.clone(), CompressionType::Zlib).expect("Failed to create way_relation output writer"),
        }
    }

    pub fn init(&mut self) {
        self.node_writer.write_header().expect("node writer failed to write header");
        self.way_relation_writer.write_header().expect("way_relation writer failed to write header");
    }

    pub fn close(&mut self) {
        self.node_writer.close().expect("Failed to close writer");
        self.way_relation_writer.close().expect("Failed to close writer");
    }
}

impl Handler for SplittingOutputHandler {
    fn name(&self) -> String {
        todo!()
    }


    fn handle_nodes(&mut self, nodes: Vec<Node>) -> Vec<Node> {
        for node in nodes {
            self.node_writer.write_element( Element::Node { node } ).expect("Failed to write node");
        }
        vec![]
    }

    fn handle_ways(&mut self, ways: Vec<Way>) -> Vec<Way> {
        for way in ways {
            self.way_relation_writer.write_element(Element::Way { way }).expect("Failed to write way");
        }
        vec![]
    }


    fn handle_relations(&mut self, relations: Vec<Relation>) -> Vec<Relation> {
        for relation in relations {
            self.way_relation_writer.write_element(Element::Relation { relation }).expect("Failed to write relation");
        }
        vec![]
    }
    fn add_result(&mut self, result: HandlerResult) -> HandlerResult {
        self.way_relation_writer.close().expect("Failed to close writer");
        let fresh_way_relation_reader = pbf::reader::Reader::new(&self.way_relation_writer.path());
        match fresh_way_relation_reader {
            Ok(reader) => {
                for element in reader.elements().unwrap() {
                    self.node_writer.write_element(element).expect("Failed to write element");
                }
            }
            Err(_) => {}
        }
        self.close();
        result
    }
}
