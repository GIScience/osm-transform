use std::collections::BTreeMap;
use std::error::Error;
use std::fs::File;
use std::path::PathBuf;
use std::str::FromStr;
use std::string::String;

use btreemultimap::BTreeMultiMap;
use csv::{ReaderBuilder, WriterBuilder};
use geo::{BoundingRect, Contains, Coord, coord, Intersects, MultiPolygon, Rect};
use geo::BooleanOps;
use osm_io::osm::model::node::Node;
use osm_io::osm::model::tag::Tag;
use serde::Deserialize;
use wkt::{Geometry, ToWkt};
use wkt::Wkt;

use crate::handler::{Handler, HandlerResult};

const GRID_SIZE: usize = 64800;
const AREA_ID_MULTIPLE: u16 = u16::MAX;

pub struct Tile {
    bbox: Rect<f64>,
    poly: MultiPolygon<f64>,
}

pub(crate) struct AreaHandler {
    pub(crate) mapping: Mapping,
    grid: Vec<Tile>,
    country_not_found_node_count: u64,
    country_found_node_count: u64,
}

pub(crate) struct Mapping {
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

pub(crate) struct AreaIntersect {
    id: u16,
    geo: MultiPolygon<f64>,
}

impl Default for AreaHandler {
    fn default() -> Self {
        Self {
            mapping: Mapping::default(),
            grid: {
                let mut grid: Vec<Tile> = Vec::new();
                for grid_lat in 0..180 {
                    for grid_lon in 0..360 {
                        let box_lon: f64 = grid_lon as f64 - 180.0;
                        let box_lat: f64 = grid_lat as f64 - 90.0;
                        let bbox = Rect::new(coord! {x: box_lon, y: box_lat},
                                             coord! {x: box_lon + 1f64, y: box_lat + 1f64},);
                        let poly = bbox.to_polygon().into();
                        grid.push(Tile{bbox, poly});
                    }
                }
                grid
            },
            country_not_found_node_count: 0,
            country_found_node_count: 0,
        }
    }
}

impl AreaHandler {

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
                    self.add_area(index, &record.id, &record.name, &converted);
                }
                Geometry::Polygon(p) => {
                    let converted: MultiPolygon = p.into();
                    self.add_area(index, &record.id, &record.name, &converted);
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

    fn add_area(&mut self, index: u16, id: &String, name: &String, area_geometry: &MultiPolygon) {
        self.mapping.id.insert(index, id.to_string());
        self.mapping.name.insert(index, name.to_string());
        let area_bbox = &area_geometry.bounding_rect().unwrap();
        let mut _intersecting_grid_tiles = 0;
        for i in 0..GRID_SIZE {
            let tile_bbox = &self.grid[i].bbox;
            if tile_bbox.intersects(area_bbox) && tile_bbox.intersects(area_geometry) {
                _intersecting_grid_tiles += 1;
                if area_geometry.contains(tile_bbox) {
                    self.mapping.index[i] = index;
                } else {
                    let tile_poly = &self.grid[i].poly;
                    self.mapping.index[i] = AREA_ID_MULTIPLE;
                    self.mapping.area.insert(i as u16, AreaIntersect{id: index, geo: tile_poly.intersection(&area_geometry)});
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
    fn handle_node(&mut self, node: &mut Node) {
        let mut result: Vec<String> = Vec::new();
        if node.coordinate().lat() >= 90.0 || node.coordinate().lat() <= -90.0 {
            return;
        }
        let grid_index = (node.coordinate().lat() as i32 + 90) * 360 + (node.coordinate().lon() as i32 + 180);
        let coord = Coord {x: node.coordinate().lon(), y: node.coordinate().lat()};
        match self.mapping.index[grid_index as usize] {
            0 => { // no area
                self.country_not_found_node_count += 1;
            }
            AREA_ID_MULTIPLE => { // multiple areas
                self.country_found_node_count += 1;
                for area in self.mapping.area.get_vec(&(grid_index as u16)).unwrap() {
                    if area.geo.contains(&coord) {
                        result.push(self.mapping.id[&area.id].to_string())
                    }
                }
            }
            x => { // single area
                // debug!("index: {x}");
                self.country_found_node_count += 1;
                result.push(self.mapping.id[&x].to_string())
            }
        }
        let node = node;
        if ! result.is_empty() {
            node.tags_mut().push(Tag::new("country".to_string(), result.join(",")));
        }
    }
}

impl Handler for AreaHandler {
    fn name(&self) -> String { "AreaHandler".to_string() }
    fn handle_nodes(&mut self, mut elements: Vec<Node>) -> Vec<Node> {
        elements.iter_mut().for_each(|node| self.handle_node(node));
        elements
    }

    fn add_result(&mut self, mut result: HandlerResult) -> HandlerResult {
        result.country_not_found_node_count = self.country_not_found_node_count;
        result.country_found_node_count = self.country_found_node_count;
        result.other.insert("mapping".to_string(), format!("index:{} area:{} id:{} name:{}", &self.mapping.index.len(), &self.mapping.area.len(), &self.mapping.id.len(), &self.mapping.name.len(), ));
        result
    }
}

#[cfg(test)]
mod tests {
    //TODO: Add unit tests for AreaHandler
}
