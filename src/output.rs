use std::path::PathBuf;
use osm_io::osm::model::element::Element;
use osm_io::osm::model::node::Node;
use osm_io::osm::model::relation::Relation;
use osm_io::osm::model::way::Way;
use osm_io::osm::pbf;
use osm_io::osm::pbf::compression_type::CompressionType;
use osm_io::osm::pbf::file_info::FileInfo;
use crate::handler::{Handler, HandlerResult, into_node_element, into_relation_element, into_way_element, format_element_id};

pub struct SimpleOutputHandler {
    pub writer: pbf::writer::Writer,
}


impl SimpleOutputHandler {
    pub fn new(output_path: PathBuf) -> Self {
        let mut file_info = FileInfo::default();
        file_info.with_writingprogram_str("rusty-routes-transformer");
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

    fn handle_result(&mut self, result: &mut HandlerResult) {
        result.nodes.iter().for_each(|node| {
            self.writer.write_element( Element::Node { node: node.clone() }).expect("Failed to write node");
        });
        result.ways.iter().for_each(|way| {
            self.writer.write_element( Element::Way { way: way.clone() }).expect("Failed to write way");
        });
        result.relations.iter().for_each(|relation| {
            self.writer.write_element( Element::Relation { relation: relation.clone() }).expect("Failed to write relation");
        });
        result.clear_elements();
    }

    fn add_result(&mut self, result: &mut HandlerResult) {
        self.close();
        result.other.insert("output file".to_string(), format!("{:?}", &self.writer.path()));
    }
}


pub struct SplittingOutputHandler {
    pub node_writer: pbf::writer::Writer,
    pub way_relation_writer: pbf::writer::Writer,
}

impl SplittingOutputHandler {
    pub fn new(output_path: PathBuf) -> Self {
        let mut file_info_node = FileInfo::default();
        file_info_node.with_writingprogram_str("rusty-routes-transformer");
        let mut file_info_way_relation = FileInfo::default();
        file_info_way_relation.with_writingprogram_str("rusty-routes-transformer");

        let base_name = output_path.as_os_str().to_str().unwrap();
        let ways_relations_path = PathBuf::from(format!("{}_ways_relations.pbf", base_name));

        Self {
            node_writer: pbf::writer::Writer::from_file_info(output_path, file_info_node, CompressionType::Zlib).expect("Failed to create node output writer"),
            way_relation_writer: pbf::writer::Writer::from_file_info(ways_relations_path, file_info_way_relation, CompressionType::Zlib).expect("Failed to create way_relation output writer"),
        }
    }

    pub fn init(&mut self) {
        self.node_writer.write_header().expect("node writer failed to write header");
        self.way_relation_writer.write_header().expect("way_relation writer failed to write header");
    }

    pub fn close(&mut self) {
        log::info!("Closing both writers");
        self.node_writer.close().expect("Failed to close writer");
        self.way_relation_writer.close().expect("Failed to close writer");
    }
}

impl Handler for SplittingOutputHandler {
    fn name(&self) -> String {
        "SplittingOutputHandler".to_string()
    }

    fn handle_result(&mut self, result: &mut HandlerResult) {
        result.nodes.iter().for_each(|node| {
            self.node_writer.write_element( Element::Node { node: node.clone() }).expect("Failed to write node");
        });
        result.ways.iter().for_each(|way| {
            self.way_relation_writer.write_element( Element::Way { way: way.clone() }).expect("Failed to write way");
        });
        result.relations.iter().for_each(|relation| {
            self.way_relation_writer.write_element( Element::Relation { relation: relation.clone() }).expect("Failed to write relation");
        });
        result.clear_elements();
    }

    fn add_result(&mut self, result: &mut HandlerResult) {
        self.way_relation_writer.close().expect("Failed to close way_relation_writer");
        self.node_writer.close().expect("Failed to close node writer");
        result.other.insert("output files".to_string(), format!("{:?}, {:?}", &self.way_relation_writer.path() , &self.node_writer.path()));
        log::info!("Reading the newly generated file {:?} and appending all elements to {:?}...", &self.way_relation_writer.path(), &self.node_writer.path());
        let fresh_way_relation_reader = pbf::reader::Reader::new(&self.way_relation_writer.path());
        match fresh_way_relation_reader {
            Ok(reader) => {
                for element in reader.elements().unwrap() {
                    log::trace!("fresh_way_relation_reader copies element {} to node_writer", format_element_id(&element));
                    self.node_writer.write_element(element).expect("Failed to write element");
                }
            }
            Err(_) => {}
        }
        self.node_writer.close().expect("Failed to close node writer");
    }
}
