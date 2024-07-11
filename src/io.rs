use std::path::PathBuf;

use anyhow;
use benchmark_rs::stopwatch::StopWatch;
use osm_io::osm::model::coordinate::Coordinate;
use osm_io::osm::model::element::Element;
use osm_io::osm::model::node::Node;
use osm_io::osm::model::tag::Tag;

use crate::conf::Config;
use crate::handler::{Handler, HandlerResult};
use osm_io::osm::pbf;
use osm_io::osm::pbf::compression_type::CompressionType;
use osm_io::osm::pbf::file_info::FileInfo;

pub fn process_with_handler(config: &Config, handler: &mut dyn Handler) -> Result<(), anyhow::Error> {
    log::info!("Started pbf io pipeline");
    let mut stopwatch = StopWatch::new();
    stopwatch.start();
    let input_path = PathBuf::from(config.input_path.to_string());
    let reader = pbf::reader::Reader::new(&input_path)?;
    if config.with_copy {
        log::info!("Running variant where objects are cloned...");
        for element in reader.elements()? {
            match element {
                Element::Node { mut node } => {
                    handler.handle_node_chained(&mut node)
                },
                Element::Way { mut way } => {
                    handler.handle_way_chained(&mut way)
                },
                Element::Relation { mut relation } => {
                    handler.handle_relation_chained(&mut relation)
                },
                _ => (),
            }
        }
    } else {
        log::info!("Running variant where objects are not cloned...");
        for element in reader.elements()? {
            match element {
                Element::Node { mut node } => {
                    handler.handle_node_chained_owned(node)
                },
                Element::Way { mut way } => {
                    handler.handle_way_chained_owned(way)
                },
                Element::Relation { mut relation } => {
                    handler.handle_relation_chained_owned(relation)
                },
                _ => (),
            }
        }
    }
    let mut handler_result = HandlerResult::default();
    handler.get_results_chained(&mut handler_result);
    log::info!("Result: {}, {}, {}, {}", handler_result.bbox_min_lat, handler_result.bbox_max_lat, handler_result.bbox_min_lon, handler_result.bbox_max_lon);
    log::info!("Finished pbf io pipeline, time: {}", stopwatch);
    Ok(())
}

pub fn process_file() -> Result<(), anyhow::Error> {
    log::info!("Started pbf io pipeline");
    let mut stopwatch = StopWatch::new();
    stopwatch.start();
    let input_path = PathBuf::from("test/baarle_small.pbf");
    let output_path = PathBuf::from("test/baarle_small-mod.pbf");
    let reader = pbf::reader::Reader::new(&input_path)?;
    let mut file_info = FileInfo::default();
    file_info.with_writingprogram_str("pbf-io-example");
    let mut writer =
        pbf::writer::Writer::from_file_info(output_path, file_info, CompressionType::Zlib)?;

    writer.write_header()?;
    let mut first = true;
    for element in reader.elements()? {
        let mut filter_out = false;
        match &element {
            Element::Node { node: _ } => {}
            Element::Way { way } => {
                if first {
                    let mut tags = Vec::new();
                    tags.push(Tag::new("key".to_string(), "value".to_string()));
                    let new_node = Node::new(
                        999999,
                        1,
                        Coordinate::new(0.0, 0.0),
                        0,
                        0,
                        0,
                        "argh".to_string(),
                        true,
                        tags,
                    );
                    let element: Element = Element::Node { node: new_node };
                    writer.write_element(element)?;
                    first = false;
                }
                for tag in way.tags() {
                    if tag.k() == "building" && tag.v() == "yes" {
                        filter_out = true;
                        break;
                    }
                }
            }
            Element::Relation { relation: _ } => {}
            Element::Sentinel => {
                filter_out = true;
            }
        }
        if !filter_out {
            writer.write_element(element)?;
        }
    }

    writer.close()?;

    log::info!("Finished pbf io pipeline, time: {}", stopwatch);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::handler::{FinalHandler, HandlerResult, NodeIdCollector};
    use pbf::reader::Reader;

    #[test]
    fn process_files_verify_node_added() {
        let output_path = PathBuf::from("test/baarle_small-mod.pbf");
        let reader = Reader::new(&output_path).expect("output file not found");
        let mut found = false;
        for element in reader.elements().expect("corrupted file") {
            match &element {
                Element::Node { node } => {
                    if node.id() == 999999 {
                        found = true;
                    }
                }
                _ => (),
            }
        }
        assert!(found);
    }
}
