use std::collections::BTreeMap;
use std::error::Error;
use std::fs::File;
use std::path::PathBuf;
use std::str::FromStr;
use std::string::String;

use btreemultimap::BTreeMultiMap;
use csv::{ReaderBuilder, WriterBuilder};
use geo::{Contains, Coord, Intersects, LineString, MultiPolygon, Polygon};
use geo::BooleanOps;
use osm_io::osm::model::node::Node;
use osm_io::osm::model::tag::Tag;
use serde::Deserialize;
use wkt::{Geometry, ToWkt};
use wkt::Wkt;

use crate::Config;
use crate::handler::Handler;

const GRID_SIZE: usize = 64800;
const AREA_ID_MULTIPLE: u16 = u16::MAX;

pub struct AreaHandler {
    pub mapping: Mapping,
    grid: Vec<MultiPolygon<f64>>,
}

pub struct Mapping {
    pub index: [u16; GRID_SIZE],
    pub area: BTreeMultiMap<u16, AreaIntersect>,
    pub id: BTreeMap<u16, String>,
    pub name: BTreeMap<u16, String>,
}

impl Default for Mapping {
    fn default() -> Self {
        Self {
            index: [0; GRID_SIZE],
            area: BTreeMultiMap::new(),
            id: BTreeMap::new(),
            name: BTreeMap::new(),
        }
    }
}

struct AreaIntersect {
    id: u16,
    geo: MultiPolygon<f64>,
}

impl Default for AreaHandler {
    fn default() -> Self {
        Self {
            mapping: Mapping::default(),
            grid: {
                let mut grid: Vec<MultiPolygon<f64>> = Vec::new();
                for grid_lat in 0..180 {
                    for grid_lon in 0..360 {
                        let box_lon: f64 = grid_lon as f64 - 180.0;
                        let box_lat: f64 = grid_lat as f64 - 90.0;
                        let polygon = Polygon::new(
                            LineString::from(vec![(box_lon, box_lat), (box_lon + 1f64, box_lat), (box_lon + 1f64, box_lat + 1f64), (box_lon, box_lat + 1f64), (box_lon, box_lat)]),
                            vec![],
                        );
                        grid.push(polygon.into());
                    }
                }
                grid
            },
        }
    }
}

impl AreaHandler {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn load(&mut self, path_buf: PathBuf) -> Result<(), Box<dyn Error>> {
        #[derive(Debug, Deserialize)]
        struct Record {
            id: String,
            name: String,
            geo: String,
        }

        let file_basename = path_buf.file_stem().to_owned().unwrap_or_default().to_str().unwrap_or_default();
        let file =  File::open(path_buf.clone())?;
        let mut rdr = ReaderBuilder::new().delimiter(b';').from_reader(file);
        let mut index: u16 = 1;
        for result in rdr.deserialize() {
            let record: Record = result?;
            let geo: Wkt<f64> = Wkt::from_str(record.geo.as_str())?;
            let _ls = match geo.item {
                Geometry::MultiPolygon(mp) => {
                    let converted: MultiPolygon = mp.into();
                    self.add_area(index, &record.id, &record.name, converted);
                }
                Geometry::Polygon(p) => {
                    let converted: MultiPolygon = p.into();
                    self.add_area(index, &record.id, &record.name, converted);
                }
                _ => {
                    log::warn!("Area CSV file contains row with unsupported geometry! ID: {}, Name: {}", record.id, record.name);
                }
            };
            index = index + 1;
        }
        AreaHandler::save_area_records(file_basename, &self.mapping);

        Ok(())
    }

    fn add_area(&mut self, index: u16, id: &String, name: &String, area_geometry: MultiPolygon) {
        self.mapping.id.insert(index, id.to_string());
        self.mapping.name.insert(index, name.to_string());
        let mut _intersecting_grid_tiles = 0;
        for i in 0..GRID_SIZE {
            let grid_polygon = &self.grid[i];
            if grid_polygon.intersects(&area_geometry) {
                _intersecting_grid_tiles += 1;
                if area_geometry.contains(grid_polygon) {
                    self.mapping.index[i] = index;
                } else {
                    self.mapping.index[i] = AREA_ID_MULTIPLE;
                    self.mapping.area.insert(i as u16, AreaIntersect{id: index, geo: grid_polygon.intersection(&area_geometry)});
                }
            }
        }
    }

    fn save_area_records(name: &str, mapping: &Mapping) {
        let mut wtr = WriterBuilder::new().delimiter(b';').from_path(name.to_string() + "_id.csv").expect("failed to open writer");
        for (key, value) in mapping.id.iter() {
            wtr.write_record(&[key.to_string(), value.to_string()]).expect("failed to write");
        }
        wtr.flush().expect("failed to flush");
        let mut wtr = WriterBuilder::new().delimiter(b';').from_path(name.to_string() + "_name.csv").expect("failed to open writer");
        for (key, value) in mapping.name.iter() {
            wtr.write_record(&[key.to_string(), value.to_string()]).expect("failed to write");
        }
        wtr.flush().expect("failed to flush");
        let mut wtr = WriterBuilder::new().delimiter(b';').from_path(name.to_string() + "_index.csv").expect("failed to open writer");
        for id in 0..GRID_SIZE {
            if mapping.index[id] > 0 {
                wtr.write_record(&[id.to_string(), mapping.index[id].to_string()]).expect("failed to write");
            }
        }
        wtr.flush().expect("failed to flush");
        let mut wtr = WriterBuilder::new().delimiter(b';').from_path(name.to_string() + "_area.csv").expect("failed to open writer");
        for (key, values) in mapping.area.iter() {
            wtr.write_record(&[key.to_string(), values.id.to_string(), values.geo.wkt_string()]).expect("failed to write");
        }
        wtr.flush().expect("failed to flush");
    }
}

impl Handler for AreaHandler {
    fn handle_node(&mut self, node: Node) -> Option<Node> {
        let mut result: Vec<String> = Vec::new();
        if node.coordinate().lat() >= 90.0 || node.coordinate().lat() <= -90.0 {
            return None;
        }
        let grid_index = (node.coordinate().lat() as i32 + 90) * 360 + (node.coordinate().lon() as i32 + 180);
        let coord = Coord {x: node.coordinate().lon(), y: node.coordinate().lat()};
        match self.mapping.index[grid_index as usize] {
            0 => { // no area
            }
            AREA_ID_MULTIPLE => { // multiple areas
                for area in self.mapping.area.get_vec(&(grid_index as u16)).unwrap() {
                    if area.geo.contains(&coord) {
                        result.push(self.mapping.id[&area.id].to_string())
                    }
                }
            }
            x => { // single area
                // debug!("index: {x}");
                result.push(self.mapping.id[&x].to_string())
            }
        }
        let mut node = node;
        node.tags_mut().push(Tag::new("country".to_string(), result.join(",")));
        Some(node)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use crate::area::AreaHandler;
    use crate::handler::{CountType, ElementCounter, HandlerChain, OsmElementTypeSelection};
    use crate::io::process_with_handler;

    use super::*;

    #[test]
    #[ignore] // fixme
    fn test_area_handler() {
        let config = Config {
            input_pbf: PathBuf::from("test/baarle_small.pbf"),
            output_pbf:  Some(PathBuf::from("output.pbf")),
            country_csv: Some(PathBuf::from("test/mapping_test.csv")),
            debug: 0,
            with_node_filtering: false,
            print_node_ids: HashSet::new(),
            print_way_ids: HashSet::new(),
            print_relation_ids: HashSet::new(),
        };

        let mut area_handler = AreaHandler::new();
        area_handler.load(config.country_csv.clone().unwrap()).expect("Area handler failed to load CSV file");

        let mut handler_chain = HandlerChain::default()
            .add(ElementCounter::new(OsmElementTypeSelection {node:true, way:false, relation:false}, CountType::ALL))
            .add(area_handler)
            .add(ElementCounter::new(OsmElementTypeSelection {node:true, way:false, relation:false}, CountType::ACCEPTED));

        let _ = process_with_handler(&config, &mut handler_chain).expect("process_with_handler failed");
        let result = handler_chain.collect_result();
        println!("result: {:?}", result )
    }

}
