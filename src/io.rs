
use anyhow;
use benchmark_rs::stopwatch::StopWatch;
use osm_io::osm::pbf;
use crate::Config;
use crate::handler::HandlerChain;

pub(crate) fn process_with_handler(config: &Config, handler_chain: &mut HandlerChain) -> Result<(), anyhow::Error> {
    log::info!("Starting pbf io pipeline...");
    let mut stopwatch = StopWatch::new();
    stopwatch.start();
    let reader = pbf::reader::Reader::new(&config.input_pbf)?;

    for element in reader.elements()? {
        handler_chain.process(element);
    }
    handler_chain.flush_handlers();
    log::info!("Finished pbf io pipeline, time: {}", stopwatch);
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use osm_io::osm::model::coordinate::Coordinate;
    use osm_io::osm::model::element::Element;
    use osm_io::osm::model::node::Node;
    use osm_io::osm::model::tag::Tag;
    use osm_io::osm::pbf::compression_type::CompressionType;
    use osm_io::osm::pbf::file_info::FileInfo;
    use pbf::reader::Reader;

    use super::*;

    pub fn process_file(output: String) -> Result<(), anyhow::Error> {
        log::info!("Starting pbf io pipeline...");
        let mut stopwatch = StopWatch::new();
        stopwatch.start();
        let input_path = PathBuf::from("test/baarle_small.pbf");
        let output_path = PathBuf::from(output);
        let reader = Reader::new(&input_path)?;
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

    #[test]
    #[ignore] //fixme - test does not work with cargo test
    fn process_files_verify_node_added() {
        let output = "test/baarle_small-mod.pbf".to_string();

        process_file(output.clone()).expect("ARGH");

        let output_path = PathBuf::from(output.clone());
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
