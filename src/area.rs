use std::collections::BTreeMap;
use std::error::Error;
use std::fs::File;
use std::path::PathBuf;
use std::str::FromStr;
use std::string::String;

use btreemultimap::BTreeMultiMap;
use csv::{ReaderBuilder, WriterBuilder};
use geo::{BoundingRect, Contains, Coord, coord, Intersects, MultiPolygon, Rect, HasDimensions};
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

pub fn wkt_string_to_multipolygon(wkt_string: &str) -> Result<MultiPolygon<f64>, Box<dyn Error>> {
    let geo: Wkt<f64> = Wkt::from_str(wkt_string)?;
    match geo.item {
        Geometry::MultiPolygon(mp) => Ok(mp.into()),
        Geometry::Polygon(p) => Ok(p.into()),
        _ => Err("Unsupported geometry type".into()),
    }
}

impl Default for AreaHandler {
    fn default() -> Self {
        Self {
            mapping: Mapping::default(),
            grid: {
                let mut grid: Vec<Tile> = Vec::new();
                for grid_lat in 0..180 {
                    for grid_lon in 0..360 {
                        let box_lon = grid_lon  - 180;
                        let box_lat = grid_lat  - 90;
                        let bbox = Rect::new(coord! {x: box_lon as f64, y: box_lat as f64},
                                             coord! {x: (box_lon + 1) as f64, y: (box_lat + 1) as f64},);
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
            let converted = wkt_string_to_multipolygon(record.geo.as_str());
            match converted {
                Ok (mp) => {
                    self.add_area(index, &record.id, &record.name, &mp);
                }
                Err(_) => {
                    log::warn!("Area CSV file contains row with unsupported geometry! ID: {}, Name: {}", record.id, record.name);
                }
            }
            index = index + 1;
        }
        AreaHandler::save_area_records(file_basename, &self.mapping);

        Ok(())
    }


    fn add_area(&mut self, country_index: u16, country_csv_id: &String, name: &String, area_geometry: &MultiPolygon) {
        self.mapping.id.insert(country_index, country_csv_id.to_string());
        self.mapping.name.insert(country_index, name.to_string());
        let area_bbox = &area_geometry.bounding_rect().unwrap();
        let mut _intersecting_grid_tiles = 0;
        for grid_id in 0..GRID_SIZE {//TODO make configurable grid size, ~0.2-0.5 degree could be better
            let tile_bbox = &self.grid[grid_id].bbox;
            if tile_bbox.intersects(area_bbox) && tile_bbox.intersects(area_geometry) {
                _intersecting_grid_tiles += 1;
                if area_geometry.contains(tile_bbox) && self.mapping.index[grid_id] == 0 {
                    self.mapping.index[grid_id] = country_index;
                } else {
                    let tile_poly = &self.grid[grid_id].poly;
                    let intersect = area_geometry.intersection(tile_poly);
                    if !intersect.is_empty() {
                        if self.mapping.index[grid_id] != 0 && self.mapping.index[grid_id] != AREA_ID_MULTIPLE {
                            self.mapping.area.insert(grid_id as u16, AreaIntersect{id: self.mapping.index[grid_id], geo: tile_poly.clone()});
                        }
                        self.mapping.index[grid_id] = AREA_ID_MULTIPLE;
                        self.mapping.area.insert(grid_id as u16, AreaIntersect{id: country_index, geo: intersect});
                    }
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
        let mut result_vec: Vec<String> = Vec::new();
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
                    if area.geo.intersects(&coord) {
                        result_vec.push(self.mapping.id[&area.id].to_string())
                    }
                }
            }
            area_id => { // single area
                // debug!("index: {x}");
                self.country_found_node_count += 1;
                result_vec.push(self.mapping.id[&area_id].to_string())
            }
        }
        let node = node;
        if ! result_vec.is_empty() {
            node.tags_mut().push(Tag::new("country".to_string(), result_vec.join(",")));
        }
    }
}

impl Handler for AreaHandler {
    fn name(&self) -> String { "AreaHandler".to_string() }

    fn handle_result(&mut self, result: &mut HandlerResult) {
        result.nodes.iter_mut().for_each(|node| self.handle_node(node));
        result.country_not_found_node_count = self.country_not_found_node_count;
        result.country_found_node_count = self.country_found_node_count;
    }

    fn add_result(&mut self, result: &mut HandlerResult){
        result.other.insert("mapping".to_string(), format!("index:{} area:{} id:{} name:{}",
                                                           &self.mapping.index.len(),
                                                           &self.mapping.area.len(),
                                                           &self.mapping.id.len(),
                                                           &self.mapping.name.len(), ));
    }
}

#[cfg(test)]
mod tests {
    use osm_io::osm::model::coordinate::Coordinate;
    use super::*;
    struct LonLat {
        lon: f64,
        lat: f64,
    }
    impl LonLat {
        fn new(lon: f64, lat: f64) -> Self {
            Self { lon, lat }
        }
        fn c(&self) -> Coordinate {
            Coordinate::new(self.lat, self.lon)
        }
    }
    #[test]
    fn test_area_handler_borders_identical_to_grid_edge() {
        let mut area_handler = AreaHandler::default();
        area_handler.add_area(
            1, &"SQA".to_string(), &"Squareland".to_string(),
            &wkt_string_to_multipolygon("POLYGON((1.0 1.0, 1.0 2.0, 2.0 2.0, 2.0 1.0, 1.0 1.0))").unwrap()
        );
        area_handler.add_area(
            2, &"REC".to_string(), &"Rectanglia".to_string(),
            &wkt_string_to_multipolygon("MULTIPOLYGON(((2.0 1.0, 2.0 2.0, 4.0 2.0, 4.0 1.0, 2.0 1.0)))").unwrap()
        );
        area_handler.add_area(
            3, &"TRI".to_string(), &"Trianglia".to_string(),
            &wkt_string_to_multipolygon("MULTIPOLYGON(((5.0 1.0, 7.0 1.0, 6.0 2.0, 5.0 1.0)))").unwrap()
        );

        AreaHandler::save_area_records("test_area_handler", &area_handler.mapping);
        /*
        |
       2| ssbrrrr   t
        | ssbrrrr  ttt
       1| ssbrrrr ttttt
        |
        +----------------
          1 2 3 4 5 6 7 8
        MULTIPOINT((1.5 1.5), (3.0 1.5), (2.0 1.5), (6.0 1.5), (1.0 3.0))
        MULTIPOINT((2.0 2.0), (3.5 2.0), (2.5 2.0), (6.5 2.0), (1.5 3.5))
         */

        let mut result = HandlerResult::default();
        result.nodes.push(Node::new(0, 1, LonLat::new(1.5, 1.5).c(), 1, 1, 1, "s".to_string(), true, vec![]));
        result.nodes.push(Node::new(1, 1, LonLat::new(3.0, 1.5).c(), 1, 1, 1, "r".to_string(), true, vec![]));
        result.nodes.push(Node::new(2, 1, LonLat::new(2.0, 1.5).c(), 1, 1, 1, "b".to_string(), true, vec![]));
        result.nodes.push(Node::new(3, 1, LonLat::new(6.0, 1.5).c(), 1, 1, 1, "t".to_string(), true, vec![]));
        result.nodes.push(Node::new(4, 1, LonLat::new(1.0, 3.0).c(), 1, 1, 1, "_".to_string(), true, vec![]));

        area_handler.handle_result(&mut result);
        println!("{} {} {:?}", &result.nodes[0].id(),  &result.nodes[0].user(), &result.nodes[0].tags());
        println!("{} {} {:?}", &result.nodes[1].id(),  &result.nodes[1].user(), &result.nodes[1].tags());
        println!("{} {} {:?}", &result.nodes[2].id(),  &result.nodes[2].user(), &result.nodes[2].tags());
        println!("{} {} {:?}", &result.nodes[3].id(),  &result.nodes[3].user(), &result.nodes[3].tags());
        println!("{} {} {:?}", &result.nodes[4].id(),  &result.nodes[4].user(), &result.nodes[4].tags());
        assert!(result.nodes[0].tags().iter().any(|tag| { tag.k() == "country" && tag.v() == "SQA"}));
        assert!(result.nodes[1].tags().iter().any(|tag| { tag.k() == "country" && tag.v() == "REC"}));
        assert!(result.nodes[3].tags().iter().all(|tag| { tag.k() == "country" && tag.v() == "TRI"}));
        assert!(result.nodes[4].tags().iter().all(|tag| tag.k() != "country"));

        // If a coordinate is on the border of two areas, and this border is identical to a grid edge,
        // the coordinate is not assigned to both areas. We accept this limitation, because it is a rare case.
        assert!(result.nodes[2].tags().iter().any(|tag| { tag.k() == "country" && (tag.v() == "SQA" || tag.v() == "REC" || tag.v() == "SQA,REC" || tag.v() == "REC,SQA")}));
    }
    #[test]
    fn test_area_handler() {
        let mut area_handler = AreaHandler::default();
        area_handler.add_area(3, &"TRI".to_string(), &"Trianglia".to_string(),
            &wkt_string_to_multipolygon("MULTIPOLYGON(((5.5 1.5, 7.5 1.5, 6.5 2.5, 5.5 1.5)))").unwrap()
        );
        area_handler.add_area(2, &"REC".to_string(), &"Rectanglia".to_string(),
                              &wkt_string_to_multipolygon("MULTIPOLYGON(((2.5 1.5, 2.5 2.5, 4.5 2.5, 4.5 1.5, 2.5 1.5)))").unwrap()
        );
        area_handler.add_area(1, &"SQA".to_string(), &"Squareland".to_string(),
                              &wkt_string_to_multipolygon("POLYGON((1.5 1.5, 1.5 2.5, 2.5 2.5, 2.5 1.5, 1.5 1.5))").unwrap()
        );

        AreaHandler::save_area_records("test_area_handler", &area_handler.mapping);
        /*
        |
       2| ssbrrrr   t
        | ssbrrrr  ttt
       1| ssbrrrr ttttt
        |
        +----------------
          1 2 3 4 5 6 7 8
        MULTIPOINT((1.5 1.5), (3.0 1.5), (2.0 1.5), (6.0 1.5), (1.0 3.0))
        MULTIPOINT((2.0 2.0), (3.5 2.0), (2.5 2.0), (6.5 2.0), (1.5 3.5))
         */

        let mut result = HandlerResult::default();
        result.nodes.push(Node::new(0, 1, LonLat::new(2.1, 2.1).c(), 1, 1, 1, "s".to_string(), true, vec![]));
        result.nodes.push(Node::new(1, 1, LonLat::new(3.6, 2.1).c(), 1, 1, 1, "r".to_string(), true, vec![]));
        result.nodes.push(Node::new(2, 1, LonLat::new(2.5, 2.1).c(), 1, 1, 1, "b".to_string(), true, vec![]));
        result.nodes.push(Node::new(3, 1, LonLat::new(6.6, 2.1).c(), 1, 1, 1, "t".to_string(), true, vec![]));
        result.nodes.push(Node::new(4, 1, LonLat::new(1.6, 3.6).c(), 1, 1, 1, "_".to_string(), true, vec![]));

        area_handler.handle_result(&mut result);
        println!("{} {} {:?}", &result.nodes[0].id(),  &result.nodes[0].user(), &result.nodes[0].tags());
        println!("{} {} {:?}", &result.nodes[1].id(),  &result.nodes[1].user(), &result.nodes[1].tags());
        println!("{} {} {:?}", &result.nodes[2].id(),  &result.nodes[2].user(), &result.nodes[2].tags());
        println!("{} {} {:?}", &result.nodes[3].id(),  &result.nodes[3].user(), &result.nodes[3].tags());
        println!("{} {} {:?}", &result.nodes[4].id(),  &result.nodes[4].user(), &result.nodes[4].tags());
        assert!(result.nodes[0].tags().iter().any(|tag| { tag.k() == "country" && tag.v() == "SQA"}));
        assert!(result.nodes[1].tags().iter().any(|tag| { tag.k() == "country" && tag.v() == "REC"}));
        assert!(result.nodes[2].tags().iter().any(|tag| { tag.k() == "country" && (tag.v() == "SQA,REC" || tag.v() == "REC,SQA")}));
        assert!(result.nodes[3].tags().iter().all(|tag| { tag.k() == "country" && tag.v() == "TRI"}));
        assert!(result.nodes[4].tags().iter().all(|tag| tag.k() != "country"));
    }

    #[test]
    fn check_intersect_vs_contains() {
        let a = geo::Rect::new( Coord{x: 1.0, y: 1.0},  Coord{x: 2.0, y: 2.0});
        let a_copy = geo::Rect::new(Coord{x: 1.0, y: 1.0}, Coord{x: 2.0, y: 2.0});
        let b_neighbor_a = geo::Rect::new(Coord{x: 2.0, y: 1.0}, Coord{x: 3.0, y: 2.0});
        let point_on_edge_a = Coord{x: 1.5, y: 1.0};
        assert!(   a.intersects(&a_copy) );
        assert!(   a.contains(&a_copy) );
        assert!(   a.intersects(&b_neighbor_a) );
        assert!( ! a.contains(&b_neighbor_a) );
        assert!(   a.intersects(&point_on_edge_a) );
        assert!( ! a.contains(&point_on_edge_a) );
        assert!(   point_on_edge_a.intersects(&a) );
    }
}
