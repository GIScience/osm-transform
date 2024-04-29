mod io {

    use std::path::PathBuf;

    use anyhow;
    use benchmark_rs::stopwatch::StopWatch;
    use simple_logger::SimpleLogger;

    use osm_io::osm::model::element::Element;
    use osm_io::osm::model::node::Node;
    use osm_io::osm::model::tag::Tag;
    use osm_io::osm::model::coordinate::Coordinate;

    use osm_io::osm::pbf;
    use osm_io::osm::pbf::compression_type::CompressionType;
    use osm_io::osm::pbf::file_info::FileInfo;

    pub fn process_file() -> Result<(), anyhow::Error> {
        SimpleLogger::new().init()?;
        log::info!("Started pbf io pipeline");
        let mut stopwatch = StopWatch::new();
        stopwatch.start();
        let input_path = PathBuf::from("./chemindelagode.pbf");
        let output_path = PathBuf::from("./chemindelagode-mod.pbf");
        let reader = pbf::reader::Reader::new(&input_path)?;
        let mut file_info = FileInfo::default();
        file_info.with_writingprogram_str("pbf-io-example");
        let mut writer = pbf::writer::Writer::from_file_info(
            output_path,
            file_info,
            CompressionType::Zlib,
        )?;

        writer.write_header()?;
        let mut first = true;
        for element in reader.elements()? {
            let mut filter_out = false;
            match &element {
                Element::Node { node: _ } => {
                }
                Element::Way { way } => {
                    if first {
                        let mut tags = Vec::new();
                        tags.push(Tag::new("key".to_string(), "value".to_string()));
                        let new_node = Node::new(999999, 1, Coordinate::new(0.0, 0.0), 0, 0, 0, "argh".to_string(), true, tags);
                        let element: Element = Element::Node{node: new_node};
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
}