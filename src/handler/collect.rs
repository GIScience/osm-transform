use bit_vec::BitVec;
use log::warn;
use osm_io::osm::model::relation::Member;

use crate::handler::{Handler, HandlerData, OsmElementTypeSelection};

pub(crate) struct ReferencedNodeIdCollector {
    count_unique: usize,
}
impl ReferencedNodeIdCollector {
    pub(crate) fn default() -> Self {
        Self {
            count_unique: 0,
        }
    }


    fn handle_ways_result(&mut self, data: &mut HandlerData)  {
        for element in & data.ways {
            for &id in element.refs() {
                self.add_node_id(&mut data.accept_node_ids, id);
            }
        }
    }

    fn handle_relations_result(&mut self, data: &mut HandlerData)  {
        for element in &mut data.relations{
            for member in element.members() {
                match member {
                    Member::Node { member } => {
                        self.add_node_id(&mut data.accept_node_ids, member.id());
                    }
                    Member::Way { .. } => {}
                    Member::Relation { .. } => {}
                }
            }
        }
    }
    fn add_node_id(&mut self, node_ids: &mut BitVec, id: i64)  {
        if matches!(node_ids.get(id as usize), Some(false)) { self.count_unique += 1 }
        node_ids.set(id as usize, true);
    }
}
impl Handler for ReferencedNodeIdCollector {
    fn name(&self) -> String {
        "ReferencedNodeIdCollector".to_string()
    }

    fn handle(&mut self, data: &mut HandlerData) {
        self.handle_ways_result(data);
        self.handle_relations_result(data);
    }
}

fn safe_id(i: i64) -> Option<usize> {
    if i >= 0 {
        usize::try_from(i).ok() // Converts only if it's within `usize` bounds
    } else {
        warn!("Received negative id: {}", i);
        None // Negative numbers can't be `usize`
    }
}

pub(crate) struct IdCollector {
    pub(crate) handle_types: OsmElementTypeSelection,
}
impl Handler for IdCollector {
    fn name(&self) -> String {
        "IdCollector".to_string()
    }

    fn handle(&mut self, data: &mut HandlerData) {
        if self.handle_types.node {
            data.nodes.iter().for_each(|node| match safe_id(node.id()) {
                    Some(id) => data.accept_node_ids.set(id, true),
                    None => {}
            });
        }
        if self.handle_types.way {
            data.ways.iter().for_each(|way| match safe_id(way.id()) {
                    Some(id) => data.accept_way_ids.set(id, true),
                    None => {}
            });
        }
        if self.handle_types.relation {
            data.relations.iter().for_each(|relation| match safe_id(relation.id()) {
                    Some(id) => data.accept_relation_ids.set(id, true),
                    None => {}
            });
        }
    }
}

pub(crate) struct MinMaxIdCollector {
    pub(crate) handle_types: OsmElementTypeSelection,
    pub(crate) min_pos_node_id: i64,
    pub(crate) max_pos_node_id: i64,
    pub(crate) min_neg_node_id: i64,
    pub(crate) max_neg_node_id: i64,
    pub(crate) min_pos_way_id: i64,
    pub(crate) max_pos_way_id: i64,
    pub(crate) min_neg_way_id: i64,
    pub(crate) max_neg_way_id: i64,
    pub(crate) min_pos_relation_id: i64,
    pub(crate) max_pos_relation_id: i64,
    pub(crate) min_neg_relation_id: i64,
    pub(crate) max_neg_relation_id: i64,
}
impl MinMaxIdCollector {
    pub(crate) fn new(handle_types: OsmElementTypeSelection) -> Self {
        Self {
            handle_types,
            min_pos_node_id: i64::MAX,
            max_pos_node_id: i64::MIN,
            min_neg_node_id: 0,
            max_neg_node_id: 0,
            min_pos_way_id: i64::MAX,
            max_pos_way_id: i64::MIN,
            min_neg_way_id: 0,
            max_neg_way_id: 0,
            min_pos_relation_id: i64::MAX,
            max_pos_relation_id: i64::MIN,
            min_neg_relation_id: 0,
            max_neg_relation_id: 0,
        }
    }
}
impl Handler for MinMaxIdCollector {
    fn name(&self) -> String {
        "MinMaxIdCollector".to_string()
    }

    fn handle(&mut self, data: &mut HandlerData) {
        if self.handle_types.node {
            for node in &data.nodes {
                let id = node.id();
                if id > 0 {
                    self.min_pos_node_id = self.min_pos_node_id.min(id);
                    self.max_pos_node_id = self.max_pos_node_id.max(id);
                } else {
                    self.min_neg_node_id = self.min_neg_node_id.min(id);
                    self.max_neg_node_id = self.max_neg_node_id.max(id);
                }
            }
        }
        if self.handle_types.way {
            for way in &data.ways {
                let id = way.id();
                if id > 0 {
                    self.min_pos_way_id = self.min_pos_way_id.min(id);
                    self.max_pos_way_id = self.max_pos_way_id.max(id);
                } else {
                    self.min_neg_way_id = self.min_neg_way_id.min(id);
                    self.max_neg_way_id = self.max_neg_way_id.max(id);
                }
            }
        }
        if self.handle_types.relation {
            for relation in &data.relations {
                let id = relation.id();
                if id > 0 {
                    self.min_pos_relation_id = self.min_pos_relation_id.min(id);
                    self.max_pos_relation_id = self.max_pos_relation_id.max(id);
                } else {
                    self.min_neg_relation_id = self.min_neg_relation_id.min(id);
                    self.max_neg_relation_id = self.max_neg_relation_id.max(id);
                }
            }
        }
    }

    fn close(&mut self, data: &mut HandlerData) {
        if self.handle_types.node {
            data.other.insert("min_pos_node_id".to_string(), format!("{}", self.min_pos_node_id));
            data.other.insert("max_pos_node_id".to_string(), format!("{}", self.max_pos_node_id));
            data.other.insert("min_neg_node_id".to_string(), format!("{}", self.min_neg_node_id));
            data.other.insert("max_neg_node_id".to_string(), format!("{}", self.max_neg_node_id));
        }
        if self.handle_types.way {
            data.other.insert("min_pos_way_id".to_string(), format!("{}", self.min_pos_way_id));
            data.other.insert("max_pos_way_id".to_string(), format!("{}", self.max_pos_way_id));
            data.other.insert("min_neg_way_id".to_string(), format!("{}", self.min_neg_way_id));
            data.other.insert("max_neg_way_id".to_string(), format!("{}", self.max_neg_way_id));
        }
        if self.handle_types.relation {
            data.other.insert("min_pos_relation_id".to_string(), format!("{}", self.min_pos_relation_id));
            data.other.insert("max_pos_relation_id".to_string(), format!("{}", self.max_pos_relation_id));
            data.other.insert("min_neg_relation_id".to_string(), format!("{}", self.min_neg_relation_id));
            data.other.insert("max_neg_relation_id".to_string(), format!("{}", self.max_neg_relation_id));
        }
    }
}

#[cfg(test)]
mod test {
    use crate::handler::{HIGHEST_NODE_ID, Handler, HandlerData};
    use crate::handler::tests::{simple_node, simple_way, TestOnlyIdCollector};

    #[test]
    fn node_id_collector(){
        let mut collector = TestOnlyIdCollector::new(10);
        let mut data = HandlerData::default();
        data.nodes.push(simple_node(2, vec![]));
        collector.handle(&mut data);
        assert_eq!(false, data.accept_node_ids.get(0).unwrap_or(false));
        assert_eq!(false, data.accept_node_ids.get(1).unwrap_or(false));
        assert_eq!(true, data.accept_node_ids.get(2).unwrap_or(false));
    }
    #[test]
    #[should_panic(expected = "index out of bounds: 12 >= 10")]
    fn node_id_collector_out_of_bounds(){
        let mut collector = TestOnlyIdCollector::new(10);
        let mut data = HandlerData::default();
        data.ways.push(simple_way(12, vec![], vec![]));
        collector.handle(&mut data);
    }
    #[test]
    fn node_id_collector_out_of_bounds_real(){
        let mut collector = TestOnlyIdCollector::new(HIGHEST_NODE_ID as usize);
        let mut data = HandlerData::default();
        data.nodes.push(simple_node(1, vec![]));
        collector.handle(&mut data);
        assert_eq!(false, data.accept_node_ids.get(0).unwrap_or(false));
        assert_eq!(true, data.accept_node_ids.get(1).unwrap_or(false));
        assert_eq!(false, data.accept_node_ids.get(2).unwrap_or(false));
        assert_eq!(false, data.accept_node_ids.get(11414456780).unwrap_or(false));

        data.clear_elements();
        data.nodes.push(simple_node(11414456780, vec![]));
        collector.handle(&mut data);
        assert_eq!(true, data.accept_node_ids.get(11414456780).unwrap_or(false));
    }

}