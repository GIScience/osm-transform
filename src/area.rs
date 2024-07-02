use std::string::String;
use std::collections::HashMap;
use std::error::Error;
use std::fs::File;
use std::path::Path;
use std::str::FromStr;

use csv::{ReaderBuilder, Writer};
use geo::{Contains, Coord, Intersects, LineString, MultiPolygon, Polygon};
use geo::BooleanOps;
use log::debug;
use multimap::MultiMap;
use osm_io::osm::model::node::Node;
use serde::Deserialize;
use wkt::{Geometry, ToWkt};
use wkt::Wkt;

use crate::{Handler, into_next};
use crate::conf::Config;
use crate::io::{process_file, process_with_handler};

const GRID_SIZE: usize = 64800;
const AREA_ID_MULTIPLE: u16 = u16::MAX;

pub struct AreaHandler {
    pub next: Option<Box<dyn Handler>>,
    pub mapping: Mapping,
    grid: Vec<MultiPolygon<f64>>,
}

pub struct Mapping {
    pub index: [u16; GRID_SIZE],
    pub area: MultiMap<u16, AreaIntersect>,
    pub id: HashMap<u16, String>,
    pub name: HashMap<u16, String>,
}

impl Default for Mapping {
    fn default() -> Self {
        Self {
            index: [0; GRID_SIZE],
            area: MultiMap::new(),
            id: HashMap::new(),
            name: HashMap::new(),
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
            next: None,
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
    pub fn new(next: impl Handler + 'static) -> Self {
        Self {
            next: into_next(next),
            ..Self::default()
        }
    }

    pub fn load(&mut self, config: &Config) -> Result<(), Box<dyn Error>> {
        #[derive(Debug, Deserialize)]
        struct Record {
            id: String,
            name: String,
            geo: String,
        }


        let path = Path::new(config.country_path.as_str());
        let file_basename = path.file_stem().to_owned().unwrap_or_default().to_str().unwrap_or_default();
        if !AreaHandler::load_area_records(file_basename) {
            let file = File::open(path)?;
            let mut rdr = ReaderBuilder::new().delimiter(b';').from_reader(file);
            let mut index: u16 = 1;
            for result in rdr.deserialize() {
                let record: Record = result?;
                let geo: Wkt<f64> = Wkt::from_str(record.geo.as_str())?;
                let ls = match geo.item {
                    Geometry::MultiPolygon(mp) => {
                        let converted: MultiPolygon = mp.into();
                        self.add_area(index, &record.id, &record.name, converted);
                    }
                    Geometry::Polygon(p) => {
                        let converted: MultiPolygon = p.into();
                        self.add_area(index, &record.id, &record.name, converted);
                    }
                    _ => {
                        println!("Area CSV file contains row with unsupported geometry! ID: {}, Name: {}", record.id, record.name);
                    }
                };
                index = index + 1;
            }
            AreaHandler::save_area_records(file_basename, &self.mapping);
        }
        Ok(())
    }

    fn add_area(&mut self, index: u16, id: &String, name: &String, geo: MultiPolygon) {
        self.mapping.id.insert(index, id.to_string());
        self.mapping.name.insert(index, name.to_string());
        let mut intersecting_grid_tiles = 0;
        for i in 0..GRID_SIZE {
            let poly = &self.grid[i];
            if poly.intersects(&geo) {
                intersecting_grid_tiles += 1;
                if geo.contains(poly) {
                    self.mapping.index[i] = index;
                } else {
                    self.mapping.index[i] = AREA_ID_MULTIPLE;
                    self.mapping.area.insert(i as u16, AreaIntersect{id: index, geo: poly.intersection(&geo)});
                }
            }
        }
    }

    fn load_area_records(name: &str) -> bool {
        false
    }

    fn save_area_records(name: &str, mapping: &Mapping) {
        let id_file = name.to_string() + "_id.csv";
        let mut wtr = Writer::from_path(id_file).expect("failed to open writer");
        for (key, value) in mapping.id.iter() {
            wtr.write_record(&[key.to_string(), value.to_string()]).expect("failed to write");
        }
        wtr.flush().expect("failed to flush");
        let name_file = name.to_string() + "_name.csv";
        let mut wtr = Writer::from_path(name_file).expect("failed to open writer");
        for (key, value) in mapping.name.iter() {
            wtr.write_record(&[key.to_string(), value.to_string()]).expect("failed to write");
        }
        wtr.flush().expect("failed to flush");
        let index_file = name.to_string() + "_index.csv";
        let mut wtr = Writer::from_path(index_file).expect("failed to open writer");
        for id in 0..GRID_SIZE {
            if mapping.index[id] > 0 {
                wtr.write_record(&[id.to_string(), mapping.index[id].to_string()]).expect("failed to write");
            }
        }
        wtr.flush().expect("failed to flush");
        let area_file = name.to_string() + "_area.csv";
        let mut wtr = Writer::from_path(area_file).expect("failed to open writer");
        for (key, value) in mapping.area.iter() {
            wtr.write_record(&[key.to_string(), value.id.to_string(), value.geo.wkt_string()]).expect("failed to write");
        }
        wtr.flush().expect("failed to flush");
    }
}

impl Handler for AreaHandler {
    fn handle_node(&mut self, node: &Node) {
        let mut result: Vec<String> = Vec::new();
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
                debug!("index: {x}");
                result.push(self.mapping.id[&x].to_string())
            }
        }
        debug!("Area IDs for {}: {:?}", node.id(), result);
        self.handle_node_next(node);
    }

    fn get_next(&mut self) -> &mut Option<Box<dyn Handler>> {
        return &mut self.next;
    }
}

#[cfg(test)]
mod tests {
    use crate::area::AreaHandler;

    use super::*;

    #[test]
    fn test_area_handler() {
        let config = Config::default();
        let mut handler_chain = AreaHandler::default();
        handler_chain.load(&config).expect("Area handler failed to load CSV file");
        let _ = process_with_handler(config, &mut handler_chain).expect("Area handler failed");
        println!("Loaded: {}", handler_chain.mapping.id.len())
    }

    #[test]
    fn test_process() {
        process_file().expect("ARGH")
    }
}
